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

use serde::Serialize;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const SNIFF_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Sentinel host the injected script navigates to once a video URL is found.
const SENTINEL_HOST: &str = "moeplay.invalid";

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
        if (/\.(m3u8|mpd|mp4|flv|mkv|webm)(\?|#|$)/i.test(url)) report(url, source);
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
      try { setInterval(init, 1000); } catch(e){}
    })();
    "#
    .to_string()
}

/// Shared implementation: open a hidden window, inject the sniffer, wait for a
/// sentinel navigation carrying the found URL (or time out).
/// Dual detection: on_navigation (sentinel URL) + periodic JS polling (global var).
async fn run_sniff(app: tauri::AppHandle, episode_url: String) -> Result<VideoUrlResult, String> {
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
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
    let tx_nav = tx.clone();

    let webview = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(url_parsed))
        .visible(false)
        .initialization_script(&sniff_js())
        .inner_size(1280.0, 720.0)
        .on_navigation(move |url| {
            tracing::info!("on_navigation: {}", url);
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
        .build()
        .map_err(|e| format!("创建提取窗口失败: {}", e))?;

    // Spawn a polling task that checks window.__MOEPLAY_VIDEO_URL__ every 500ms.
    // Uses eval() to trigger sentinel navigation from JS side when URL found.
    let app_poll = app.clone();
    let label_poll = label.clone();
    let tx_poll = tx.clone();
    let poll_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.tick().await; // skip first immediate tick
        for _ in 0..60 { // max 30s (60 * 500ms)
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

    let result = tokio::time::timeout(SNIFF_TIMEOUT, rx).await;

    // Cleanup
    poll_handle.abort();
    let _ = webview.close();
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.close();
    }

    match result {
        Ok(Ok(v)) => Ok(v),
        _ => Err("video-url-timeout".into()),
    }
}

/// 很多源把真实流地址塞在播放器页 URL 的查询参数里，例如
/// `.../artplayer/index.html?url=https://cdn/.../index.m3u8`。
/// 把内层真实视频地址解出来（最多解三层，防嵌套/死循环）。
fn unwrap_player_url(raw: &str) -> String {
    let mut current = raw.to_string();
    for _ in 0..3 {
        let parsed = match url::Url::parse(&current) {
            Ok(u) => u,
            Err(_) => break,
        };
        let inner = parsed.query_pairs().find_map(|(k, v)| {
            let key = k.to_ascii_lowercase();
            if matches!(
                key.as_str(),
                "url" | "vurl" | "v" | "src" | "m3u8" | "playurl" | "video" | "link"
            ) {
                let val = v.into_owned();
                if val.starts_with("http")
                    && (val.contains(".m3u8") || val.contains(".mp4") || val.contains(".flv"))
                {
                    return Some(val);
                }
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

/// Tauri command: extract the real video stream URL from an episode page.
#[tauri::command]
pub async fn anime_extract_video_url(
    app: tauri::AppHandle,
    episode_url: String,
    referer: Option<String>,
    use_legacy_parser: bool,
) -> Result<VideoUrlResult, String> {
    tracing::info!("开始提取视频 URL: {}", episode_url);
    let _ = (referer, use_legacy_parser);
    let mut result = run_sniff(app, episode_url).await?;
    // 解出内层真实流地址；把播放器页地址作为 Referer（CDN 防盗链通常认播放器域，最准）。
    let player_url = result.url.clone();
    let real = unwrap_player_url(&result.url);
    result.tab_url = player_url;
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
    let mut result = run_sniff(app, target_url).await?;
    let player_url = result.url.clone();
    result.url = unwrap_player_url(&result.url);
    result.tab_url = player_url;
    Ok(result)
}
