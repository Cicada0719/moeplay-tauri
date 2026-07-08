//! 视频流代理 — 本地 HTTP 代理服务器
//!
//! 前端用 `http://127.0.0.1:{port}/proxy?url=xxx&referer=yyy` 播放视频。
//! Rust 端代理请求，处理 Referer / CORS / m3u8 分片地址改写。
//! 解决浏览器层 CORS / 防盗链 Referer / 混合内容问题。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use tauri::{AppHandle, Emitter};

/// 代理服务器监听端口（启动时随机分配）
static PROXY_PORT: AtomicU16 = AtomicU16::new(0);

/// 最大允许跟随的重定向次数
const MAX_REDIRECTS: u32 = 15;

/// 获取代理端口
pub fn get_proxy_port() -> u16 {
    PROXY_PORT.load(Ordering::Relaxed)
}

/// 获取代理 base URL
pub fn get_proxy_base() -> String {
    format!("http://127.0.0.1:{}", get_proxy_port())
}

/// 获取代理端口（供前端主动查询）
#[tauri::command]
pub fn get_video_proxy_port() -> u16 {
    get_proxy_port()
}

/// 把原始 URL 转为代理 URL
pub fn to_proxy_url(original_url: &str, referer: Option<&str>) -> String {
    let mut url = format!(
        "{}/proxy?url={}",
        get_proxy_base(),
        urlencoding::encode(original_url)
    );
    if let Some(r) = referer {
        url.push_str(&format!("&referer={}", urlencoding::encode(r)));
    }
    url
}

/// 把 m3u8 内容中的相对/绝对 URL 改写为代理 URL
fn rewrite_m3u8(content: &str, base_url: &str, referer: &str) -> String {
    let base = url::Url::parse(base_url).unwrap_or_else(|_| url::Url::parse("http://x").unwrap());
    let mut result = String::with_capacity(content.len() * 2);

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            // 保留原行空白，避免意外改变行号/结构
            result.push_str(line);
            result.push('\n');
        } else if trimmed.starts_with('#') {
            // 标签行：改写任意 URI="..." / URI='...' 属性。
            // 覆盖 EXT-X-KEY、EXT-X-MEDIA、EXT-X-MAP、EXT-X-I-FRAME-STREAM-INF 等。
            let rewritten = rewrite_all_uri_attributes(line, base_url, referer);
            result.push_str(&rewritten);
            result.push('\n');
        } else {
            // URL 行（分片或子 m3u8）
            let abs_url = resolve_url(&base, trimmed);
            let proxy = to_proxy_url(&abs_url, Some(referer));
            result.push_str(&proxy);
            result.push('\n');
        }
    }
    result
}

/// 改写 m3u8 标签行中所有 URI="..." / URI='...' 属性
fn rewrite_all_uri_attributes(line: &str, base_url: &str, referer: &str) -> String {
    let base = url::Url::parse(base_url).unwrap_or_else(|_| url::Url::parse("http://x").unwrap());
    let mut result = String::with_capacity(line.len() * 2);
    let mut rest = line;

    while let Some(start) = rest.find("URI=") {
        result.push_str(&rest[..start + 4]);
        rest = &rest[start + 4..];

        let quote_char = match rest.chars().next() {
            Some('"') => '"',
            Some('\'') => '\'',
            _ => {
                result.push_str(rest);
                break;
            }
        };
        result.push(quote_char);
        rest = &rest[1..];

        if let Some(end) = rest.find(quote_char) {
            let uri = &rest[..end];
            let abs_url = resolve_url(&base, uri);
            let proxy = to_proxy_url(&abs_url, Some(referer));
            result.push_str(&proxy);
            result.push(quote_char);
            rest = &rest[end + 1..];
        } else {
            result.push_str(rest);
            break;
        }
    }
    result.push_str(rest);
    result
}

/// 共享 ureq Agent —— 复用 TCP/TLS 连接；禁用自动重定向，由本模块手动处理，
/// 以便保留/记录 Referer、Range 等头，并把最终 URL 作为 m3u8 相对地址解析基址。
fn proxy_agent() -> &'static ureq::Agent {
    use std::sync::OnceLock;
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .redirects(0)
            .timeout(std::time::Duration::from_secs(30))
            .build()
    })
}

/// 构造上游请求，填充通用浏览器请求头
fn build_upstream_request<'a>(
    target_url: &'a str,
    referer: &'a str,
    range_header: Option<&'a str>,
) -> ureq::Request {
    let mut req = proxy_agent()
        .get(target_url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .set("Accept", "*/*")
        .set("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        // identity: 避免压缩后 Content-Length 与 body 不一致；同时兼容只接受 identity 的 CDN
        .set("Accept-Encoding", "identity")
        .set("Connection", "keep-alive")
        .set("Cache-Control", "no-cache")
        .set("Pragma", "no-cache");

    if !referer.is_empty() {
        req = req.set("Referer", referer);
        if let Ok(origin) = url::Url::parse(referer) {
            req = req.set("Origin", &origin.origin().ascii_serialization());
        }
        // 部分 CDN 通过 Sec-Fetch-* 判断请求来源，补全后降低 403 概率
        req = req.set("Sec-Fetch-Site", "cross-site");
        req = req.set("Sec-Fetch-Mode", "cors");
        req = req.set("Sec-Fetch-Dest", "empty");
    }

    if let Some(range) = range_header {
        req = req.set("Range", range);
    }

    req
}

/// 解析相对 URL 为绝对 URL
fn resolve_url(base: &url::Url, relative: &str) -> String {
    let relative = relative.trim();
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    base.join(relative)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| relative.to_string())
}

/// 判断 HTTP 状态码是否为重定向
fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// 向上游发起请求并手动跟随重定向，返回 (响应, 最终 URL, 重定向次数)
fn fetch_with_redirects(
    initial_url: &str,
    referer: &str,
    range_header: Option<&str>,
) -> Result<(ureq::Response, String, u32), Box<ureq::Error>> {
    let mut url = initial_url.to_string();
    let mut redirects: u32 = 0;

    loop {
        let req = build_upstream_request(&url, referer, range_header);
        // ureq 在非 2xx 时会返回 Err::Status；手动模式需要把 3xx 也当正常响应处理才能取到 Location。
        let (resp, status) = match req.call() {
            Ok(r) => {
                let s = r.status();
                (r, s)
            }
            Err(ureq::Error::Status(s, r)) => (r, s),
            Err(e) => return Err(Box::new(e)),
        };

        if is_redirect_status(status) && redirects < MAX_REDIRECTS {
            if let Some(loc) = resp.header("location") {
                redirects += 1;
                let base =
                    url::Url::parse(&url).unwrap_or_else(|_| url::Url::parse("http://x").unwrap());
                let resolved = resolve_url(&base, loc);
                tracing::info!(
                    "[proxy] HTTP {} redirect #{}: {} -> {}",
                    status,
                    redirects,
                    url,
                    resolved
                );
                url = resolved;
                continue;
            }
        }

        // ureq 的 get_url() 返回最终地址；为空时退守当前请求 URL
        let final_url = {
            let u = resp.get_url();
            if u.is_empty() {
                url.clone()
            } else {
                u.to_string()
            }
        };
        return Ok((resp, final_url, redirects));
    }
}

/// 启动代理服务器（在独立线程上运行，不阻塞 tokio）
pub fn start_proxy_server(app: AppHandle) {
    std::thread::spawn(move || {
        let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("视频代理服务器绑定失败: {}", e);
                return;
            }
        };
        let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
        PROXY_PORT.store(port, Ordering::Relaxed);
        tracing::info!("视频代理服务器启动: http://127.0.0.1:{}", port);

        // 通知前端代理已就绪
        let _ = app.emit("video-proxy-ready", port);

        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("代理连接接受失败: {}", e);
                    continue;
                }
            };
            std::thread::spawn(move || {
                handle_connection(stream);
            });
        }
    });
}

fn handle_connection(mut stream: std::net::TcpStream) {
    use std::io::{Read, Write};

    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .ok();
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(30)))
        .ok();

    let mut buf = vec![0u8; 16384];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or("");

    // 解析 GET /proxy?url=xxx&referer=yyy HTTP/1.1
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    // CORS 预检：hls.js 对带 Range 的分片请求会先发 OPTIONS，必须放行否则分片全失败
    if parts.first() == Some(&"OPTIONS") {
        tracing::info!("[proxy] OPTIONS 预检请求");
        let resp = "HTTP/1.1 204 No Content\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, OPTIONS\r\n\
                    Access-Control-Allow-Headers: *\r\n\
                    Access-Control-Max-Age: 86400\r\n\
                    Content-Length: 0\r\n\
                    Connection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }
    if parts.len() < 2 || parts[0] != "GET" {
        let resp = "HTTP/1.1 405 Method Not Allowed\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let path = parts[1];
    let (target_url, referer) = match parse_proxy_path(path) {
        Some(v) => v,
        None => {
            tracing::warn!("[proxy] 无效路径: {}", path);
            let resp = "HTTP/1.1 400 Bad Request\r\n\r\nMissing url parameter";
            let _ = stream.write_all(resp.as_bytes());
            return;
        }
    };

    // 截短 URL 用于日志
    let short_url = if target_url.len() > 100 {
        format!("{}...", &target_url[..100])
    } else {
        target_url.clone()
    };
    tracing::info!(
        "[proxy] {} → {}",
        if is_m3u8_url(&target_url) {
            "m3u8"
        } else {
            "media"
        },
        short_url
    );
    if !referer.is_empty() {
        tracing::info!("[proxy] Referer: {}", referer);
    }

    // 解析客户端的 Range 头（hls.js 分片请求必须透传，否则返回 200 而非 206 → hls.js 拒绝）
    let range_header = request
        .lines()
        .find(|l| l.to_lowercase().starts_with("range:"))
        .map(|l| {
            let val = l
                .strip_prefix("Range:")
                .or_else(|| l.strip_prefix("range:"))
                .unwrap_or(l);
            val.trim().to_string()
        });

    if let Some(ref r) = range_header {
        tracing::debug!("[proxy] client Range: {}", r);
    }

    // 发起上游请求并手动跟随重定向
    let (resp, final_url, redirects) =
        match fetch_with_redirects(&target_url, &referer, range_header.as_deref()) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("[proxy] 请求失败: {} → {}", e, short_url);
                let body = format!(
                    "HTTP/1.1 502 Bad Gateway\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
                    e
                );
                let _ = stream.write_all(body.as_bytes());
                return;
            }
        };

    let status = resp.status();
    let content_type = resp
        .header("content-type")
        .unwrap_or("application/octet-stream")
        .to_string();
    let content_length = resp.header("content-length").map(|s| s.to_string());
    let content_range = resp.header("content-range").map(|s| s.to_string());

    if redirects > 0 {
        tracing::info!(
            "[proxy] 最终 URL (after {} redirect(s)): {}",
            redirects,
            final_url
        );
    }
    tracing::debug!(
        "[proxy] upstream response: status={} content-type={} content-length={:?}",
        status,
        content_type,
        content_length
    );

    // 分片返回非 2xx 是"播一会儿卡死"的头号信号 —— 单独 warn 出来便于定位
    if !(200..300).contains(&status) {
        tracing::warn!("[proxy] upstream {} → {}", status, short_url);
    }

    // 状态文本
    let status_text = match status {
        200 => "200 OK",
        206 => "206 Partial Content",
        301 => "301 Moved Permanently",
        302 => "302 Found",
        304 => "304 Not Modified",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        416 => "416 Range Not Satisfiable",
        _ => "200 OK",
    };

    // 先读一小段头，用"内容"判断是否为 HLS manifest。
    let mut head = [0u8; 8192];
    let mut reader = resp.into_reader();
    let head_n = reader.read(&mut head).unwrap_or(0);
    let head_bytes = &head[..head_n];

    let head_looks_m3u8 = looks_like_m3u8(head_bytes);
    let looks_m3u8 = is_m3u8_content(&content_type, &target_url) || head_looks_m3u8;

    if looks_m3u8 {
        // m3u8 是文本、量小：读完整后改写分片地址，再一次性发出
        let mut body = head_bytes.to_vec();
        let mut rest = Vec::new();
        if reader.read_to_end(&mut rest).is_err() {
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n");
            return;
        }
        body.extend_from_slice(&rest);

        let content = String::from_utf8_lossy(&body);
        // 严格判断：去掉 BOM / 首尾空白后是否以 HLS 标签开头
        let is_real_m3u8 = looks_like_m3u8(body.as_slice());
        tracing::debug!(
            "[proxy] m3u8 detection: head_looks_m3u8={}, is_real_m3u8={}, len={}",
            head_looks_m3u8,
            is_real_m3u8,
            body.len()
        );

        let rewritten = if is_real_m3u8 {
            tracing::info!("[proxy] 改写 m3u8，基址 URL: {}", final_url);
            rewrite_m3u8(&content, &final_url, &referer).into_bytes()
        } else {
            tracing::warn!(
                "[proxy] m3u8 URL 返回非 HLS 内容 ({}B, content-type={}), 跳过改写",
                body.len(),
                content_type
            );
            body
        };

        let mut header = format!(
            "HTTP/1.1 {}\r\n\
             Content-Type: application/vnd.apple.mpegurl\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Access-Control-Allow-Methods: GET, OPTIONS\r\n\
             Access-Control-Allow-Headers: *\r\n\
             Access-Control-Expose-Headers: Content-Range, Content-Length, Content-Type\r\n\
             Connection: close\r\n\
             Cache-Control: no-cache\r\n",
            status_text,
            rewritten.len()
        );
        if let Some(ref cr) = content_range {
            header.push_str(&format!("Content-Range: {}\r\n", cr));
        }
        header.push_str("\r\n");
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(&rewritten);
    } else {
        // 分片 / mp4：先发已读出的头段，再边收边发流式转发，不全缓冲。
        let mut header = format!(
            "HTTP/1.1 {}\r\n\
             Content-Type: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Access-Control-Allow-Methods: GET, OPTIONS\r\n\
             Access-Control-Allow-Headers: *\r\n\
             Access-Control-Expose-Headers: Content-Range, Content-Length, Content-Type\r\n\
             Connection: close\r\n\
             Cache-Control: no-cache\r\n",
            status_text, content_type
        );
        if let Some(ref cl) = content_length {
            header.push_str(&format!("Content-Length: {}\r\n", cl));
        }
        if let Some(ref cr) = content_range {
            header.push_str(&format!("Content-Range: {}\r\n", cr));
        }
        header.push_str("\r\n");
        if stream.write_all(header.as_bytes()).is_err() {
            return;
        }
        // 先把已读出的头段发出去
        if stream.write_all(head_bytes).is_err() {
            return;
        }
        // 64KB 缓冲循环拷贝剩余；客户端 seek 会断开旧连接 → write 出错属正常，break 即可
        let mut buf = [0u8; 65536];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if stream.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }
}

/// 解析代理路径: /proxy?url=xxx&referer=yyy → (url, referer)
fn parse_proxy_path(path: &str) -> Option<(String, String)> {
    let query_start = path.find('?')?;
    let query = &path[query_start + 1..];
    let mut params: HashMap<String, String> = HashMap::new();
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        let k = kv.next()?;
        let v = kv.next().unwrap_or("");
        params.insert(
            urlencoding::decode(k).ok()?.to_string(),
            urlencoding::decode(v).ok()?.to_string(),
        );
    }
    let url = params.get("url")?.clone();
    let referer = params.get("referer").cloned().unwrap_or_default();
    Some((url, referer))
}

/// 检查字节流是否像 HLS manifest（跳过 BOM 与空白）
fn looks_like_m3u8(bytes: &[u8]) -> bool {
    let mut i = 0;
    if bytes.len() >= 3 && &bytes[0..3] == b"\xEF\xBB\xBF" {
        i += 3;
    }
    while i < bytes.len()
        && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\r' || bytes[i] == b'\n')
    {
        i += 1;
    }
    let prefix = &bytes[i..];
    prefix.starts_with(b"#EXTM3U") || prefix.starts_with(b"#EXT-X-")
}

fn is_m3u8_url(url: &str) -> bool {
    url.contains(".m3u8") || url.contains("/m3u8")
}

fn is_m3u8_content(content_type: &str, url: &str) -> bool {
    let ct = content_type.to_lowercase();
    ct.contains("mpegurl")
        || ct.contains("m3u8")
        || (ct.contains("octet-stream") && url.contains(".m3u8"))
        || url.contains(".m3u8")
        || url.contains("/m3u8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proxy_path_extracts_url_and_referer() {
        let (u, r) = parse_proxy_path(
            "/proxy?url=https%3A%2F%2Fa.com%2Fx.m3u8&referer=https%3A%2F%2Fb.com%2F",
        )
        .expect("should parse");
        assert_eq!(u, "https://a.com/x.m3u8");
        assert_eq!(r, "https://b.com/");
    }

    #[test]
    fn parse_proxy_path_missing_url_is_none() {
        assert!(parse_proxy_path("/proxy?referer=x").is_none());
    }

    #[test]
    fn resolve_url_relative_and_absolute() {
        let base = url::Url::parse("https://cdn.example.com/hls/playlist.m3u8").unwrap();
        assert_eq!(
            resolve_url(&base, "seg1.ts"),
            "https://cdn.example.com/hls/seg1.ts"
        );
        assert_eq!(
            resolve_url(&base, "/v/seg2.ts"),
            "https://cdn.example.com/v/seg2.ts"
        );
        assert_eq!(
            resolve_url(&base, "https://other.com/s.ts"),
            "https://other.com/s.ts"
        );
    }

    #[test]
    fn rewrite_m3u8_proxies_segments_keys_and_media() {
        let m3u8 = "#EXTM3U\n\
                    #EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n\
                    #EXT-X-MEDIA:TYPE=AUDIO,URI=\"audio/a.m3u8\"\n\
                    #EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100000,URI=\"iframe.m3u8\"\n\
                    #EXTINF:5.0,\n\
                    seg1.ts\n\
                    https://cdn.x/seg2.ts\n";
        let out = rewrite_m3u8(
            m3u8,
            "https://cdn.example.com/hls/index.m3u8",
            "https://site.com/",
        );
        let enc = |s: &str| urlencoding::encode(s).to_string();
        assert!(out.contains("#EXTM3U"));
        // 相对分片按 base 解析后走代理
        assert!(out.contains(&enc("https://cdn.example.com/hls/seg1.ts")));
        // 绝对分片走代理
        assert!(out.contains(&enc("https://cdn.x/seg2.ts")));
        // EXT-X-KEY 密钥 URI 改写为代理
        assert!(out.contains("URI=\"http://127.0.0.1"));
        assert!(out.contains(&enc("https://cdn.example.com/hls/key.bin")));
        // EXT-X-MEDIA URI 也改写
        assert!(out.contains(&enc("https://cdn.example.com/hls/audio/a.m3u8")));
        // EXT-X-I-FRAME-STREAM-INF URI 改写
        assert!(out.contains(&enc("https://cdn.example.com/hls/iframe.m3u8")));
        // Referer 透传到每个代理 URL
        assert!(out.contains(&enc("https://site.com/")));
    }

    #[test]
    fn rewrite_m3u8_handles_single_quotes_and_whitespace() {
        let bytes =
            b"\xEF\xBB\xBF\n\n   #EXT-X-KEY:METHOD=AES-128,URI='key.bin'\n#EXTINF:5,\nseg.ts\n";
        let m3u8 = String::from_utf8_lossy(bytes);
        let out = rewrite_m3u8(
            &m3u8,
            "https://cdn.example.com/hls/index.m3u8",
            "https://site.com/",
        );
        assert!(out.contains("URI='http://127.0.0.1"));
    }

    #[test]
    fn looks_like_m3u8_with_bom_and_whitespace() {
        assert!(looks_like_m3u8(b"\xEF\xBB\xBF\n  #EXTM3U\n#EXTINF:5,\n"));
        assert!(looks_like_m3u8(b"  \n#EXT-X-VERSION:3\n"));
        assert!(!looks_like_m3u8(b"<!DOCTYPE html>"));
    }
}
