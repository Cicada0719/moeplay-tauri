//! Video URL extractor — opens a hidden WebviewWindow, injects JS to sniff the
//! real video stream URL (m3u8 / mp4 …) out of the episode page, the same way
//! Kazumi does: hook `fetch`/`XHR` (and check for `#EXTM3U` response bodies),
//! `HTMLMediaElement.src`, and `<video>`/`<source>`/`<iframe>` DOM mutations.
//!
//! IPC back to Rust uses a **sentinel navigation** instead of `__TAURI_INTERNALS__`:
//! the injected script navigates the (hidden) page to `https://moeplay.invalid/...`
//! the moment it finds a URL, and Rust intercepts that in `on_navigation`.
//! This avoids the capability problem that silently blocked `plugin:event|emit`
//! from external-origin sniffer windows (which made extraction always time out).

use regex::Regex;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const SNIFF_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Sentinel host the injected script navigates to once a video URL is found.
const SENTINEL_HOST: &str = "moeplay.invalid";

/// 代理/广告域名过滤。
fn is_ad_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("googleads")
        || lower.contains("googlesyndication")
        || lower.contains("adtrafficquality")
        || lower.contains("doubleclick")
        || lower.contains("prestrain")
        || lower.contains("adservice")
        || lower.contains("/ads/")
        || lower.contains("adserver")
}

/// 判断一个 URL 是否像可播放的视频流地址（m3u8 / 直链 / DASH）。
fn is_video_stream_url(url: &str) -> bool {
    if url.starts_with("data:") || url.starts_with("blob:") {
        return false;
    }
    if url.is_empty() || is_ad_url(url) {
        return false;
    }
    let lower = url.to_lowercase();
    lower.contains(".m3u8")
        || lower.contains("/m3u8")
        || lower.contains("/hls/")
        || lower.contains("/dash/")
        || lower.contains(".mpd")
        || lower.contains(".mp4")
        || lower.contains(".flv")
        || lower.contains(".mkv")
        || lower.contains(".webm")
        || lower.contains(".mov")
}

/// 把 query 参数值做多次 percent-decode，处理部分源对 url 参数做了双重编码的情况。
fn fully_decode_value(value: &str) -> String {
    let mut current = value.to_string();
    for _ in 0..3 {
        match urlencoding::decode(&current) {
            Ok(decoded) if decoded.as_ref() != current => current = decoded.into_owned(),
            _ => break,
        }
    }
    current
}

#[derive(Serialize, Clone, Debug)]
pub struct VideoUrlResult {
    pub url: String,
    pub source: String,
    pub tab_url: String,
}

/// JS injected at document-start into every frame of the sniffer window.
/// Comprehensive sniffer covering the cases Kazumi handles.
/// Results are written to window.__MOEPLAY_VIDEO_URL__ for Rust-side polling.
fn sniff_js() -> String {
    r#"
    (function(){
      if (window.__moe_sniff) return;
      window.__moe_sniff = true;
      window.__MOEPLAY_VIDEO_URL__ = '';
      window.__MOEPLAY_VIDEO_SRC__ = '';
      var done = false;
      var initTimer = 0;

      function isAd(u){
        return /googleads|googlesyndication|adtrafficquality|doubleclick|prestrain|adservice/i.test(u);
      }
      // Report a found stream URL — stores globally + navigates to sentinel.
      function report(url, source){
        if (done || !url) return;
        url = String(url).trim();
        if (!url || url.indexOf('data:') === 0 || url.indexOf('blob:') === 0) return;
        if (isAd(url)) return;
        done = true;
        if (initTimer) { try { clearInterval(initTimer); } catch(e){} }
        // Many sources host the actual <video>/player inside a cross-origin iframe.
        // Neither detection path on the Rust side sees a sub-frame: WebView2's
        // NavigationStarting (on_navigation) only fires for the TOP frame, and the
        // poll evals the top frame's global. The sniffer DOES run inside the iframe,
        // so bubble the found URL up to the top frame via postMessage (cross-origin
        // safe) and let it perform the sentinel navigation Rust can intercept.
        if (window.top !== window.self) {
          try { window.top.postMessage({ __moeplay__: true, url: url, source: source }, '*'); } catch(e){}
        }
        window.__MOEPLAY_VIDEO_URL__ = url;
        window.__MOEPLAY_VIDEO_SRC__ = source;
        // Write to document.title for Rust-side polling (most reliable cross-platform)
        try { document.title = '__MOE_VIDEO__:' + url; } catch(e){}
        // Also try sentinel navigation
        try {
          var a = document.createElement('a');
          a.href = 'https://moeplay.invalid/__moevideo__?s='
            + encodeURIComponent(source) + '&u=' + encodeURIComponent(url);
          a.style.display = 'none';
          document.body.appendChild(a);
          a.click();
        } catch(e){
          try {
            location.href = 'https://moeplay.invalid/__moevideo__?s='
              + encodeURIComponent(source) + '&u=' + encodeURIComponent(url);
          } catch(e2){}
        }
      }
      // Top frame: receive URLs bubbled up from child (cross-origin) iframes and
      // re-report them here so the sentinel navigation happens at the top level.
      try {
        window.addEventListener('message', function(ev){
          var d = ev && ev.data;
          if (d && d.__moeplay__ === true && typeof d.url === 'string') {
            report(d.url, (d.source || 'iframe') + ':bubbled');
          }
        }, false);
      } catch(e){}
      function consider(url, source){
        if (!url || done) return;
        url = String(url);
        if (/^(data|blob):/i.test(url)) return;
        // resolve relative URLs (e.g. /video/xxx.m3u8) so all checks work uniformly
        if (!/^https?:\/\//i.test(url)) {
          try { url = new URL(url, location.href).href; } catch(e){ return; }
        }
        if (/\.(m3u8|mpd|mp4|flv|mkv|webm)(\?|#|$)/i.test(url) && !/\bads?\b/i.test(url)) report(url, source);
        // tokenized/path-based HLS — common pattern: /hls/xxx/index or /m3u8?token=xxx
        if (/[/.]m3u8|\/hls\//i.test(url)) report(url, source);
      }
      // parse JSON strings for embedded video URLs (many modern players fetch URL from API)
      function extractVideoUrlFromJson(text, source){
        if (!text || done || text.length > 65536) return;
        try {
          var t = text.replace(/^\s+|\xEF\xBB\xBF/g, '');
          var c = t.charAt(0);
          if (c !== '{' && c !== '[' && !/^\w+\s*\(/.test(t)) return;
          var re = /https?:(?:\\\/\\\/|\/\/)[^"'\s]*?\.(m3u8|mp4|flv|mkv|webm)(?:\?[^"'\s]*)?/gi;
          var m;
          while ((m = re.exec(text)) !== null) {
            var found = m[0].replace(/\\\//g, '/');
            if (!isAd(found)) { report(found, source + ':json'); return; }
          }
        } catch(e){}
      }

      // ── hook fetch (URL pattern + #EXTM3U body) ───────────────────────
      var origFetch = window.fetch;
      if (origFetch) {
        window.fetch = function(){
          var a = arguments[0];
          var u = (typeof a === 'string') ? a : (a && a.url);
          consider(u, 'fetch');
          var p = origFetch.apply(this, arguments);
          try {
            return p.then(function(resp){
              try {
                var ru = (resp && resp.url) || u;
                resp.clone().text().then(function(t){
                  if (t && t.slice(0, 7) === '#EXTM3U') report(ru, 'fetch-m3u8');
                  else extractVideoUrlFromJson(t, 'fetch');
                }).catch(function(){});
              } catch(e){}
              return resp;
            });
          } catch(e){ return p; }
        };
      }

      // ── hook XMLHttpRequest (URL pattern + #EXTM3U body) ──────────────
      var origOpen = XMLHttpRequest.prototype.open;
      XMLHttpRequest.prototype.open = function(m, u){
        consider(u, 'xhr');
        try {
          this.addEventListener('load', function(){
            try {
              var t = this.responseText;
              if (t && t.slice(0, 7) === '#EXTM3U') report(u, 'xhr-m3u8');
              else extractVideoUrlFromJson(t, 'xhr');
            } catch(e){}
          });
        } catch(e){}
        return origOpen.apply(this, arguments);
      };

      // ── hook HTMLMediaElement.src setter ──────────────────────────────
      try {
        var desc = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');
        if (desc && desc.set) {
          Object.defineProperty(HTMLMediaElement.prototype, 'src', {
            set: function(v){ consider(v, 'media'); return desc.set.call(this, v); },
            get: desc.get,
            configurable: true
          });
        }
      } catch(e){}

      // ── hook <source> element src setter ────────────────────────────
      try {
        var srcDesc = Object.getOwnPropertyDescriptor(HTMLSourceElement.prototype, 'src');
        if (srcDesc && srcDesc.set) {
          Object.defineProperty(HTMLSourceElement.prototype, 'src', {
            set: function(v){ consider(v, 'source-el'); return srcDesc.set.call(this, v); },
            get: srcDesc.get,
            configurable: true
          });
        }
      } catch(e){}

      // ── hook HTMLMediaElement.currentSrc (read-only, but poll it) ─────
      // Some players use MSE (MediaSource Extensions) → blob URLs.
      // We can't intercept blob: URLs, but we can detect when currentSrc changes.
      // For MSE-based players, we watch for video element's 'playing' event
      // and check if there's an m3u8 URL in the page's network requests.

      // ── scan <video>/<source>/<iframe> (existing + mutations) ─────────
      function scan(node){
        if (!node || node.nodeType !== 1) return;
        var tag = node.tagName;
        if (tag === 'VIDEO' || tag === 'SOURCE') consider(node.src || node.getAttribute('src'), 'dom');
        if (tag === 'IFRAME') consider(node.src || node.getAttribute('src'), 'iframe');
        if (node.querySelectorAll) {
          node.querySelectorAll('video,source').forEach(function(el){ consider(el.src || el.getAttribute('src'), 'dom'); });
          node.querySelectorAll('iframe').forEach(function(el){ consider(el.src || el.getAttribute('src'), 'iframe'); });
        }
      }
      try {
        var mo = new MutationObserver(function(ms){ ms.forEach(function(m){ m.addedNodes.forEach(scan); }); });
        mo.observe(document.documentElement, { childList: true, subtree: true });
      } catch(e){}

      function init(){
        if (done) return;
        document.querySelectorAll('video,source').forEach(function(el){ consider(el.src || el.getAttribute('src'), 'dom.init'); });
        document.querySelectorAll('iframe').forEach(function(el){ consider(el.src || el.getAttribute('src'), 'iframe.init'); });
        // Also check video.currentSrc for MSE-based players
        document.querySelectorAll('video').forEach(function(v){
          if (v.currentSrc && !v.currentSrc.startsWith('blob:')) consider(v.currentSrc, 'currentSrc');
        });
      }
      if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
      else init();
      // periodic re-scan for late dynamic players (Kazumi does this too)
      try { initTimer = setInterval(init, 1000); } catch(e){}
    })();
    "#
    .to_string()
}

/// Shared implementation: open a hidden window, inject the sniffer, wait for a
/// sentinel navigation carrying the found URL (or time out).
/// Triple detection:
///   1. JS injection in the main frame (fetch/XHR/DOM).
///   2. `on_web_resource_request` for **all frames** (covers cross-origin iframes
///      where the initialization script can't run).
///   3. Periodic JS polling as a safety net.
async fn run_sniff(
    app: tauri::AppHandle,
    episode_url: String,
    user_agent: Option<String>,
) -> Result<VideoUrlResult, String> {
    let label = format!(
        "video-sniff-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let url_parsed: url::Url = episode_url
        .parse()
        .map_err(|e: url::ParseError| e.to_string())?;

    let (tx, rx) = tokio::sync::oneshot::channel::<VideoUrlResult>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_nav = tx.clone();
    let tx_resource = tx.clone();

    tracing::info!("[sniff] 创建嗅探窗口: label={}, url={}", label, episode_url);
    let label_log = label.clone();

    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(url_parsed))
        .visible(false)
        .initialization_script(&sniff_js())
        .inner_size(1280.0, 720.0);

    if let Some(ref ua) = user_agent {
        if !ua.is_empty() {
            builder = builder.user_agent(ua);
        }
    }

    let _webview = builder
        .on_page_load(move |_window, payload| {
            tracing::info!("[sniff] 页面加载事件: event={:?} url={}", payload.event(), payload.url());
        })
        .on_navigation(move |url| {
            tracing::info!("[sniff] on_navigation: {}", url);
            if url.host_str() == Some(SENTINEL_HOST) {
                let mut found = String::new();
                let mut source = String::new();
                for (k, v) in url.query_pairs() {
                    match k.as_ref() {
                        "u" => found = v.into_owned(),
                        "s" => source = v.into_owned(),
                        _ => {}
                    }
                }
                if !found.is_empty() {
                    if let Ok(mut guard) = tx_nav.lock() {
                        if let Some(sender) = guard.take() {
                            let _ = sender.send(VideoUrlResult {
                                url: found,
                                source,
                                tab_url: String::new(),
                            });
                        }
                    }
                }
                // cancel the sentinel navigation — it must never actually load
                return false;
            }
            true
        })
        .on_web_resource_request(move |request, response| {
            // 通过 WebView2 网络层拦截所有帧的请求/响应，弥补 JS 注入无法进入跨域 iframe 的缺陷。
            let url = request.uri().to_string();
            let mut found: Option<(String, String)> = None;

            if is_video_stream_url(&url) {
                found = Some((url.clone(), "webresource:url".into()));
            } else {
                // 检查响应内容：m3u8 master/media playlist 通常以 #EXTM3U 开头。
                let body = response.body().as_ref();
                if body.len() >= 7
                    && &body[..7] == b"#EXTM3U"
                    && !is_ad_url(&url)
                {
                    found = Some((url.clone(), "webresource:m3u8-body".into()));
                }
            }

            if let Some((found_url, source)) = found {
                tracing::info!("[sniff] web_resource 命中: source={}, url={}", source, found_url);
                if let Ok(mut guard) = tx_resource.lock() {
                    if let Some(sender) = guard.take() {
                        let _ = sender.send(VideoUrlResult {
                            url: found_url,
                            source,
                            tab_url: String::new(),
                        });
                    }
                }
            }
        })
        .build()
        .map_err(|e| format!("创建提取窗口失败: {}", e))?;

    // Spawn a polling task that checks window.__MOEPLAY_VIDEO_URL__ every 500ms.
    // Uses eval() to trigger sentinel navigation from JS side when URL found.
    let app_poll = app.clone();
    let label_poll = label.clone();
    let tx_poll = tx.clone();
    let poll_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        interval.tick().await; // skip first immediate tick
        for _ in 0..140 { // max 35s (140 * 250ms) — overshoot SNIFF_TIMEOUT slightly so sentinel has time
            interval.tick().await;
            // Check if already resolved via on_navigation
            if tx_poll.lock().map(|g| g.is_none()).unwrap_or(true) {
                return;
            }
            // Use eval to check the global var and trigger sentinel nav if found
            if let Some(w) = app_poll.get_webview_window(&label_poll) {
                let check_js = r#"
                (function(){
                  var u = window.__MOEPLAY_VIDEO_URL__;
                  if (u && u.length > 5) {
                    try {
                      location.href = 'https://moeplay.invalid/__moevideo__?s=poll&u=' + encodeURIComponent(u);
                    } catch(e) {}
                    return 'found:' + u;
                  }
                  return 'not_found';
                })()
                "#;
                match w.eval(check_js) {
                    Ok(_) => {
                        // eval succeeded; the sentinel nav should trigger on_navigation
                    }
                    Err(e) => {
                        tracing::warn!("轮询 eval 失败: {}", e);
                        return;
                    }
                }
            } else {
                // Webview was closed
                return;
            }
        }
    });

    tracing::info!("[sniff] 等待嗅探结果 (超时 {}s)…", SNIFF_TIMEOUT.as_secs());
    let result = tokio::time::timeout(SNIFF_TIMEOUT, rx).await;

    // Cleanup — 不主动关闭窗口！
    // wry 0.55 在销毁隐藏 WebView2 窗口时会 null pointer panic，
    // 即使 catch_unwind 捕获了，也会破坏 UI 事件循环状态，
    // 导致后续的 Svelte 响应式更新失效。
    // 窗口保持隐藏，在应用退出时自动清理。
    poll_handle.abort();

    match result {
        Ok(Ok(v)) => {
            tracing::info!("[sniff] 嗅探成功: url={}, source={}", v.url, v.source);
            Ok(v)
        }
        Ok(Err(e)) => {
            tracing::error!("[sniff] 通道错误: {}", e);
            Err(format!("嗅探通道错误: {}", e))
        }
        Err(_) => {
            tracing::error!("[sniff] 嗅探超时 ({}s), label={}", SNIFF_TIMEOUT.as_secs(), label_log);
            Err("video-url-timeout".into())
        }
    }
}

/// 很多源把真实流地址塞在播放器页 URL 的查询参数里，例如
/// `.../artplayer/index.html?url=https://cdn/.../index.m3u8`。
/// 把内层真实视频地址解出来（最多解三层，防嵌套/死循环），并支持双重 URL 编码。
fn unwrap_player_url(raw: &str) -> String {
    let mut current = raw.to_string();
    for _ in 0..3 {
        let parsed = match url::Url::parse(&current) {
            Ok(u) => u,
            Err(_) => break,
        };
        let inner = parsed.query_pairs().find_map(|(k, v)| {
            let key = k.to_ascii_lowercase();
            let val = fully_decode_value(&v);
            if !val.starts_with("http") {
                return None;
            }
            let is_video_key = matches!(
                key.as_str(),
                "m3u8" | "playurl" | "video" | "videourl" | "stream" | "play_url"
                    | "mediaurl" | "play" | "vurl" | "vid" | "dash" | "hls"
            );
            let is_generic_key = matches!(
                key.as_str(),
                "url" | "v" | "src" | "link" | "file" | "source" | "media" | "jx"
            );
            if is_video_key {
                return Some(val);
            }
            if is_generic_key
                && (val.contains(".m3u8")
                    || val.contains(".mp4")
                    || val.contains(".flv")
                    || val.contains(".mkv")
                    || val.contains(".webm")
                    || val.contains(".mpd")
                    || val.contains("/m3u8")
                    || val.contains("/hls/")
                    || val.contains("/dash/"))
            {
                return Some(val);
            }
            None
        });
        match inner {
            Some(v) if v != current => current = v,
            _ => break,
        }
    }
    current
}

/// 传统解析：直接请求播放器页 HTML，用正则匹配内嵌的视频地址。
/// 对把 URL 直接写在页面 JS/JSON 里的源很有用，且不需要 WebView。
async fn legacy_extract(
    episode_url: &str,
    referer: Option<&str>,
    user_agent: Option<&str>,
) -> Result<VideoUrlResult, String> {
    tracing::info!("[legacy] 请求页面: {}", episode_url);
    let client = crate::http_client::build_reqwest_client(
        15,
        user_agent.unwrap_or(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        ),
    );
    let mut req = client.get(episode_url);
    if let Some(r) = referer {
        req = req.header("Referer", r);
    }
    let text = req
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // 1. 裸 URL
    let re_url = Regex::new(r#"https?://[^\s"'<>]+?\.(?:m3u8|mp4|mpd|flv|mkv|webm|mov)(?:\?[^\s"'<>]*)?"#).unwrap();
    for m in re_url.find_iter(&text) {
        let u = m.as_str();
        if !is_ad_url(u) {
            return Ok(VideoUrlResult {
                url: u.to_string(),
                source: "legacy:url".into(),
                tab_url: episode_url.to_string(),
            });
        }
    }

    // 2. JSON/JS 中常见的 url/src/video/playUrl 字段
    let re_json = Regex::new(r#"["'](?:url|src|video|playUrl|play_url|m3u8|mp4|stream|link)["']\s*[:=]\s*["'](https?://[^"']+?)["']"#).unwrap();
    for cap in re_json.captures_iter(&text) {
        if let Some(u) = cap.get(1) {
            let u = u.as_str();
            if !is_ad_url(u) {
                return Ok(VideoUrlResult {
                    url: u.to_string(),
                    source: "legacy:json".into(),
                    tab_url: episode_url.to_string(),
                });
            }
        }
    }

    Err("传统解析未找到视频地址".into())
}

/// Tauri command: extract the real video stream URL from an episode page.
#[tauri::command]
pub async fn anime_extract_video_url(
    app: tauri::AppHandle,
    episode_url: String,
    referer: Option<String>,
    use_legacy_parser: bool,
    user_agent: Option<String>,
) -> Result<VideoUrlResult, String> {
    tracing::info!("开始提取视频 URL: {}", episode_url);

    let mut result = if use_legacy_parser {
        legacy_extract(&episode_url, referer.as_deref(), user_agent.as_deref()).await
    } else {
        let mut res = run_sniff(app, episode_url.clone(), user_agent.clone()).await;
        if let Err(ref e) = res {
            let msg = e.to_lowercase();
            if msg.contains("timeout") || msg.contains("video-url-timeout") {
                tracing::info!("[提取] WebView 嗅探超时，回退到传统解析: {}", episode_url);
                res = legacy_extract(&episode_url, referer.as_deref(), user_agent.as_deref()).await;
            }
        }
        res
    }?;

    // 解出内层真实流地址；Referer 固定用播放器页本身最可靠。
    let real = unwrap_player_url(&result.url);
    result.tab_url = episode_url;
    if real != result.url {
        tracing::info!("解出内层视频地址: {} (来源页 {})", real, result.tab_url);
        result.url = real;
    }
    tracing::info!("视频提取成功: {} (source: {})", result.url, result.source);
    Ok(result)
}

/// Tauri command: simple variant used by other call sites.
#[tauri::command]
pub async fn extract_video_url(
    app: tauri::AppHandle,
    target_url: String,
) -> Result<VideoUrlResult, String> {
    let mut result = run_sniff(app, target_url.clone(), None).await?;
    let player_url = result.url.clone();
    result.url = unwrap_player_url(&result.url);
    result.tab_url = player_url;
    Ok(result)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_player_url_extracts_inner_m3u8() {
        let raw = "https://player.example.com/artplayer/index.html?url=https%3A%2F%2Fcdn.example.com%2Fplaylist.m3u8";
        assert_eq!(
            unwrap_player_url(raw),
            "https://cdn.example.com/playlist.m3u8"
        );
    }

    #[test]
    fn unwrap_player_url_handles_double_encoding() {
        let raw = "https://player.example.com/?url=https%253A%252F%252Fcdn.example.com%252Fplaylist.m3u8";
        assert_eq!(
            unwrap_player_url(raw),
            "https://cdn.example.com/playlist.m3u8"
        );
    }

    #[test]
    fn fully_decode_value_decodes_multiple_times() {
        assert_eq!(
            fully_decode_value("https%253A%252F%252Fcdn.example.com%252Fplaylist.m3u8"),
            "https://cdn.example.com/playlist.m3u8"
        );
    }

    #[test]
    fn is_video_stream_url_recognises_common_manifests() {
        assert!(is_video_stream_url("https://cdn.example.com/playlist.m3u8"));
        assert!(is_video_stream_url("https://cdn.example.com/hls/123/index.m3u8"));
        assert!(is_video_stream_url("https://cdn.example.com/video.mp4"));
        assert!(!is_video_stream_url("https://example.com/page.html"));
        assert!(!is_video_stream_url("blob:https://example.com/abc"));
        assert!(!is_video_stream_url("https://googleads.g.doubleclick.net/pagead/id"));
    }
}
