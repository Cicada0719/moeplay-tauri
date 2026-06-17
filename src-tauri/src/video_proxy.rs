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

/// 获取代理端口
pub fn get_proxy_port() -> u16 {
    PROXY_PORT.load(Ordering::Relaxed)
}

/// 获取代理 base URL
pub fn get_proxy_base() -> String {
    format!("http://127.0.0.1:{}", get_proxy_port())
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
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            // 非 URL 行（注释/标签）。改写任意标签里的 URI="..."：
            // EXT-X-KEY（AES 密钥）、EXT-X-MEDIA（备用音轨/字幕）、EXT-X-MAP（fMP4 初始化段）等。
            if trimmed.starts_with('#') && trimmed.contains("URI=\"") {
                let rewritten = rewrite_uri_attribute(line, base_url, referer);
                result.push_str(&rewritten);
            } else {
                result.push_str(line);
            }
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

/// 改写 m3u8 标签中 URI="..." 属性
fn rewrite_uri_attribute(line: &str, base_url: &str, referer: &str) -> String {
    if let Some(start) = line.find("URI=\"") {
        let uri_start = start + 5;
        if let Some(end) = line[uri_start..].find('"') {
            let uri = &line[uri_start..uri_start + end];
            let base =
                url::Url::parse(base_url).unwrap_or_else(|_| url::Url::parse("http://x").unwrap());
            let abs_url = resolve_url(&base, uri);
            let proxy = to_proxy_url(&abs_url, Some(referer));
            return format!(
                "{}{}{}",
                &line[..uri_start],
                proxy,
                &line[uri_start + end..]
            );
        }
    }
    line.to_string()
}

/// 共享 ureq Agent —— 复用 TCP/TLS 连接，CDN 分片不必每次握手
fn proxy_agent() -> &'static ureq::Agent {
    use std::sync::OnceLock;
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(ureq::agent)
}

/// 解析相对 URL 为绝对 URL
fn resolve_url(base: &url::Url, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    base.join(relative)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| relative.to_string())
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
    let short_url = if target_url.len() > 100 { format!("{}...", &target_url[..100]) } else { target_url.clone() };
    tracing::info!("[proxy] {} → {}", if is_m3u8_content("", &target_url) { "m3u8" } else { "media" }, short_url);
    if !referer.is_empty() {
        tracing::info!("[proxy] Referer: {}", referer);
    }

    // 解析客户端的 Range 头（hls.js 分片请求必须透传，否则返回 200 而非 206 → hls.js 拒绝）
    let range_header = request.lines()
        .find(|l| l.to_lowercase().starts_with("range:"))
        .map(|l| l.trim().to_string());

    // 用 ureq 代理请求（纯同步，不依赖 tokio），Agent 复用连接池
    let mut req = proxy_agent()
        .get(&target_url)
        .timeout(std::time::Duration::from_secs(30))
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        );

    if !referer.is_empty() {
        req = req.set("Referer", &referer);
        if let Ok(origin) = url::Url::parse(&referer) {
            req = req.set("Origin", &origin.origin().ascii_serialization());
        }
    }

    // 透传 Range 头
    if let Some(ref range) = range_header {
        let range_val = range.strip_prefix("Range:").or_else(|| range.strip_prefix("range:")).unwrap_or(range);
        req = req.set("Range", range_val.trim());
    }

    let resp = match req.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(code, resp)) => {
            tracing::warn!("[proxy] upstream HTTP {} → {}", code, short_url);
            resp
        }
        Err(e) => {
            tracing::error!("[proxy] 请求失败: {} → {}", e, short_url);
            let body = format!("HTTP/1.1 502 Bad Gateway\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}", e);
            let _ = stream.write_all(body.as_bytes());
            return;
        }
    };

    let status = resp.status();
    let content_type = resp
        .header("content-type")
        .unwrap_or("application/octet-stream")
        .to_string();
    // 透传 Content-Range（hls.js Range 请求必须拿到这个才能正确缓冲）
    let content_range = resp.header("content-range").map(|s| s.to_string());
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
        _ => "200 OK",
    };

    if is_m3u8_content(&content_type, &target_url) {
        // m3u8 是文本、量小：整体读取后改写分片地址，再一次性发出
        let mut body = Vec::new();
        if resp.into_reader().read_to_end(&mut body).is_err() {
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n");
            return;
        }
        let content = String::from_utf8_lossy(&body);
        // 上游返回了非 m3u8 内容（HTML 错误页 / 403 / 空响应）→ 不要改写，直接透传
        // 否则 rewrite_m3u8 把 HTML 里的行当 URL 重写 → hls.js 拿到垃圾 manifest 报更奇怪的错
        let is_real_m3u8 = content.trim_start().starts_with("#EXTM3U")
            || content.trim_start().starts_with("#EXT-X-");
        let rewritten = if is_real_m3u8 {
            rewrite_m3u8(&content, &target_url, &referer).into_bytes()
        } else {
            tracing::warn!("[proxy] m3u8 URL 返回非 HLS 内容 ({}B), 跳过改写", body.len());
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
        // 分片 / mp4：边收边发流式转发，不全缓冲。
        // 旧实现 read_to_end 把整段读进内存才发第一个字节 → 大分片/慢 CDN 下
        // hls.js 前向缓冲撑过十几秒就枯竭 → 播放卡死。流式后边收边发，缓冲不再断流。
        let content_length = resp.header("content-length").map(|s| s.to_string());
        let mut header = format!(
            "HTTP/1.1 {}\r\n\
             Content-Type: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Access-Control-Allow-Methods: GET, OPTIONS\r\n\
             Access-Control-Allow-Headers: *\r\n\
             Access-Control-Expose-Headers: Content-Range, Content-Length, Content-Type\r\n\
             Connection: close\r\n\
             Cache-Control: no-cache\r\n",
            status_text,
            content_type
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
        // 64KB 缓冲循环拷贝；客户端 seek 会断开旧连接 → write 出错属正常，break 即可
        let mut reader = resp.into_reader();
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

fn is_m3u8_content(content_type: &str, url: &str) -> bool {
    content_type.contains("mpegurl")
        || content_type.contains("m3u8")
        || (content_type.contains("octet-stream") && url.contains(".m3u8"))
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
        assert_eq!(resolve_url(&base, "seg1.ts"), "https://cdn.example.com/hls/seg1.ts");
        assert_eq!(resolve_url(&base, "/v/seg2.ts"), "https://cdn.example.com/v/seg2.ts");
        assert_eq!(resolve_url(&base, "https://other.com/s.ts"), "https://other.com/s.ts");
    }

    #[test]
    fn rewrite_m3u8_proxies_segments_keys_and_media() {
        let m3u8 = "#EXTM3U\n\
                    #EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n\
                    #EXT-X-MEDIA:TYPE=AUDIO,URI=\"audio/a.m3u8\"\n\
                    #EXTINF:5.0,\n\
                    seg1.ts\n\
                    https://cdn.x/seg2.ts\n";
        let out = rewrite_m3u8(m3u8, "https://cdn.example.com/hls/index.m3u8", "https://site.com/");
        let enc = |s: &str| urlencoding::encode(s).to_string();
        assert!(out.contains("#EXTM3U"));
        // 相对分片按 base 解析后走代理
        assert!(out.contains(&enc("https://cdn.example.com/hls/seg1.ts")));
        // 绝对分片走代理
        assert!(out.contains(&enc("https://cdn.x/seg2.ts")));
        // EXT-X-KEY 密钥 URI 改写为代理
        assert!(out.contains("URI=\"http://127.0.0.1"));
        assert!(out.contains(&enc("https://cdn.example.com/hls/key.bin")));
        // EXT-X-MEDIA URI 也改写（本次补的缺口）
        assert!(out.contains(&enc("https://cdn.example.com/hls/audio/a.m3u8")));
        // Referer 透传到每个代理 URL
        assert!(out.contains(&enc("https://site.com/")));
    }
}
