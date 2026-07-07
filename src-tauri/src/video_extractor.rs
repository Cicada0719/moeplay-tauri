//! Video URL extractor — opens a hidden WebviewWindow, injects JS to sniff the
//! real video stream URL (m3u8 / mp4 …) out of the episode page, the same way
//! Kazumi does: hook `fetch`/`XHR` (and check for `#EXTM3U` response bodies),
//! `HTMLMediaElement.src`, and `<video>`/`<source>`/`<iframe>` DOM mutations.
//!
//! IPC back to Rust uses a **sentinel navigation** instead of `__TAURI_INTERNALS__`:
//! the injected script navigates the (hidden) page to `https://moeplay.invalid/...`
//! the moment it finds a URL, and Rust intercepts this in `on_navigation`.
//! This avoids the capability problem that silently blocked `plugin:event|emit`
//! from external-origin sniffer windows (which made extraction always time out).

use regex::Regex;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const SNIFF_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
/// 找到候选 URL 后继续等待一小段时间，让页面完成「激活回调」（部分源在解析出
/// 地址后还需向服务器发送心跳/回调标记流为可用，参考 Kazumi issue #117）。
const SETTLE_DURATION: std::time::Duration = std::time::Duration::from_millis(1200);

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
        || lower.contains("googletagmanager")
        || lower.contains("google-analytics")
        || lower.contains("umeng")
        || lower.contains("baidu.com")
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

/// 尝试判断字符串是否为 base64 编码的 URL
fn looks_like_base64_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 12 || trimmed.len() % 4 != 0 {
        return false;
    }
    let allowed = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');
    if !allowed {
        return false;
    }
    // base64 解码后应为 http 链接
    if let Ok(decoded) = base64::decode(trimmed) {
        if let Ok(s) = String::from_utf8(decoded) {
            return s.starts_with("http") && (s.contains(".m3u8") || s.contains(".mp4"));
        }
    }
    false
}

/// 把 query 参数值做多次 percent-decode，处理部分源对 url 参数做了双重编码的情况。
/// 同时尝试 base64 解码（部分播放器会把完整 m3u8 URL base64 后放在 url/data 参数里）。
fn fully_decode_value(value: &str) -> String {
    let mut current = value.to_string();
    for _ in 0..3 {
        match urlencoding::decode(&current) {
            Ok(decoded) if decoded.as_ref() != current => current = decoded.into_owned(),
            _ => break,
        }
    }
    // 若解码后仍是 base64 形态的 URL，再解一次
    if looks_like_base64_url(&current) {
        if let Ok(bytes) = base64::decode(current.trim()) {
            if let Ok(s) = String::from_utf8(bytes) {
                return s;
            }
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
    r##"
    (function(){
      if (window.__moe_sniff) return;
      window.__moe_sniff = true;
      window.__MOEPLAY_VIDEO_URL__ = '';
      window.__MOEPLAY_VIDEO_SRC__ = '';
      var done = false;
      var initTimer = 0;

      function isAd(u){
        return /googleads|googlesyndication|adtrafficquality|doubleclick|prestrain|adservice|googletagmanager|google-analytics|umeng|baidu\.com/i.test(u);
      }

      function log(msg){
        try { console.log('[moe-sniff]', msg); } catch(e){}
      }

      // Report a found stream URL — stores globally + navigates to sentinel.
      function report(url, source){
        if (!url) return;
        url = String(url).trim();
        if (!url || url.indexOf('data:') === 0 || url.indexOf('blob:') === 0) return;
        if (isAd(url)) return;
        // 跳过常见的 404/广告后缀占位
        if (/\.(jpg|jpeg|png|gif|webp|css|js|woff2?)(\?|#|$)/i.test(url) && !/\.(m3u8|mp4|mpd|flv|mkv|webm)/i.test(url)) return;
        // 始终更新全局变量，让 Rust 轮询可以在 settle 窗口内拿到更优/更晚的地址
        // （解决部分源找到地址后还要等激活回调才有效的 Kazumi 式问题）。
        if (window.__MOEPLAY_VIDEO_URL__ !== url) {
          window.__MOEPLAY_VIDEO_URL__ = url;
          window.__MOEPLAY_VIDEO_SRC__ = source;
          log('found ' + source + ': ' + url);
          // Write to document.title for Rust-side polling (most reliable cross-platform)
          try { document.title = '__MOE_VIDEO__:' + url; } catch(e){}
        }
        if (done) return;
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
        // Also try sentinel navigation (只触发一次，避免反复跳转)
        try {
          var a = document.createElement('a');
          a.href = 'https://moeplay.invalid/__moevideo__?s='
            + encodeURIComponent(source) + '&u=' + encodeURIComponent(url);
          a.style.display = 'none';
          if (document.body) document.body.appendChild(a);
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

      function isVideoUrl(u){
        return u && /^https?:\/\//i.test(u) && !isAd(u)
          && /\.(m3u8|mpd|mp4|flv|mkv|webm|mov)(\?|#|$)|[\/.]m3u8|\/hls\/|\/dash\//i.test(u);
      }

      function consider(url, source){
        if (!url) return;
        url = String(url);
        if (/^(data|blob):/i.test(url)) return;
        // resolve relative URLs (e.g. /video/xxx.m3u8) so all checks work uniformly
        if (!/^https?:\/\//i.test(url)) {
          try { url = new URL(url, location.href).href; } catch(e){ return; }
        }
        if (isVideoUrl(url)) report(url, source);
      }

      // 从页面 URL 查询参数中直接解出内层视频地址（常见 Artplayer/DPlayer/Plyr）
      function unwrapPlayerUrl(raw){
        try {
          var u = new URL(raw);
          var keys = ['url','v','src','link','file','source','video','playUrl','play_url','m3u8',
                      'stream','mediaurl','play','vurl','vid','dash','hls','player','dplayer',
                      'artplayer','ckplayer','videojs','plyr','video_url','m3u8_url','api','data','jx'];
          for (var i = 0; i < keys.length; i++) {
            var v = u.searchParams.get(keys[i]);
            if (!v) continue;
            v = decodeURIComponent(v);
            // 双重编码
            for (var k = 0; k < 2; k++) {
              try {
                var d = decodeURIComponent(v);
                if (d !== v) v = d; else break;
              } catch(e){ break; }
            }
            // base64
            if (/^[A-Za-z0-9+/=]+$/.test(v) && v.length % 4 === 0) {
              try {
                var b = atob(v);
                if (/^https?:\/\//.test(b) && /\.(m3u8|mp4|mpd|flv)/i.test(b)) v = b;
              } catch(e){}
            }
            if (isVideoUrl(v)) return v;
          }
        } catch(e){}
        return null;
      }

      // parse JSON strings for embedded video URLs (many modern players fetch URL from API)
      function extractVideoUrlFromJson(text, source){
        if (!text || text.length > 65536) return;
        try {
          var t = text.replace(/^\s+|\xEF\xBB\xBF/g, '');
          var c = t.charAt(0);
          if (c !== '{' && c !== '[' && !/^\w+\s*\(/.test(t)) return;
          var re = /https?:(?:\\\/\\\/|\/\/)[^"'\s]*?\.(m3u8|mp4|flv|mkv|webm|mpd)(?:\?[^"'\s]*)?/gi;
          var m;
          while ((m = re.exec(text)) !== null) {
            var found = m[0].replace(/\\\//g, '/');
            if (!isAd(found)) { report(found, source + ':json'); return; }
          }
        } catch(e){}
      }

      // 检查常见的全局播放器对象
      function checkGlobalPlayers(){
        try {
          // DPlayer
          if (window.dp && window.dp.video && window.dp.video.src) {
            consider(window.dp.video.src, 'dplayer');
          }
          if (window.DPlayer && window.DPlayer.players && window.DPlayer.players.length) {
            var p = window.DPlayer.players[0];
            if (p && p.video && p.video.src) consider(p.video.src, 'dplayer');
          }
          // Artplayer
          if (window.art && window.art.url) {
            consider(window.art.url, 'artplayer');
          }
          if (window.Artplayer && window.Artplayer.instances) {
            window.Artplayer.instances.forEach(function(inst){
              if (inst && inst.url) consider(inst.url, 'artplayer');
              if (inst && inst.option && inst.option.url) consider(inst.option.url, 'artplayer');
            });
          }
          // Video.js
          if (window.videojs && window.videojs.getPlayers) {
            var players = window.videojs.getPlayers();
            for (var k in players) {
              var pl = players[k];
              if (pl && pl.src) consider(pl.src(), 'videojs');
              if (pl && pl.currentSrc) consider(pl.currentSrc(), 'videojs');
            }
          }
          if (window.player && window.player.src) {
            consider(player.src(), 'player-global');
          }
          // Plyr
          if (window.Plyr && window.Plyr.instances) {
            window.Plyr.instances.forEach(function(inst){
              if (inst && inst.source) {
                var src = (inst.source.sources && inst.source.sources[0] && inst.source.sources[0].src) || inst.source;
                if (typeof src === 'string') consider(src, 'plyr');
              }
            });
          }
        } catch(e){}
      }

      // 检查页面 URL 本身是否携带视频地址
      function checkPageUrl(){
        var u = unwrapPlayerUrl(location.href);
        if (u) report(u, 'page-url');
      }

      // ── PerformanceResourceTiming：部分播放器加载视频后不会暴露到全局对象，
      // 但 performance.getEntriesByType('resource') 能拿到实际网络请求地址。
      function checkPerformanceEntries(){
        try {
          var entries = performance.getEntriesByType('resource');
          for (var i = 0; i < entries.length; i++) {
            var name = entries[i].name;
            if (name) consider(name, 'performance');
          }
        } catch(e){}
      }
      try {
        var origGetEntriesByType = performance.getEntriesByType;
        performance.getEntriesByType = function(type){
          var entries = origGetEntriesByType.apply(this, arguments);
          if (type === 'resource') checkPerformanceEntries();
          return entries;
        };
      } catch(e){}

      // ── hook URL.createObjectURL / revokeObjectURL ────────────────────
      // 部分源用 blob/url 包装真实流，createObjectURL 的输入可能是 MediaSource/Buffer。
      try {
        var origCreateObjectURL = URL.createObjectURL;
        URL.createObjectURL = function(obj){
          var url = origCreateObjectURL.apply(this, arguments);
          try {
            if (obj && (obj.type || '').indexOf('video') !== -1) {
              report(url, 'blob:video');
            }
          } catch(e){}
          return url;
        };
      } catch(e){}

      // ── hook MediaSource.addSourceBuffer ──────────────────────────────
      // MSE 播放器在 addSourceBuffer 时传入的 mimeType 包含视频信息，
      // 虽然拿不到 URL，但可以确认页面确实在初始化 MSE 视频。
      try {
        var origAddSourceBuffer = MediaSource.prototype.addSourceBuffer;
        MediaSource.prototype.addSourceBuffer = function(mimeType){
          try {
            if (mimeType && /video/i.test(mimeType)) {
              log('MSE video source buffer: ' + mimeType);
            }
          } catch(e){}
          return origAddSourceBuffer.apply(this, arguments);
        };
      } catch(e){}

      // ── hook WebSocket ────────────────────────────────────────────────
      // 少数实时流/协议通过 WebSocket 传输，拦截 send/open 中的 URL。
      try {
        var OrigWebSocket = window.WebSocket;
        window.WebSocket = function(url, protocols){
          if (url) consider(url, 'websocket');
          return new OrigWebSocket(url, protocols);
        };
        window.WebSocket.prototype = OrigWebSocket.prototype;
      } catch(e){}

      // ── hook fetch (URL pattern + #EXTM3U body) ───────────────────────
      var origFetch = window.fetch;
      if (origFetch) {
        window.fetch = function(){
          var a = arguments[0];
          var u = (typeof a === 'string') ? a : (a && a.url);
          consider(u, 'fetch');
          // 如果 fetch 到的响应是 m3u8 直链，第一时间上报最终响应 URL
          var p = origFetch.apply(this, arguments);
          try {
            return p.then(function(resp){
              try {
                var ru = (resp && resp.url) || u;
                resp.clone().text().then(function(t){
                  if (t && (t.slice(0, 7) === '#EXTM3U' || /^\s*#EXT-X-/.test(t))) report(ru, 'fetch-m3u8');
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
              if (t && (t.slice(0, 7) === '#EXTM3U' || /^\s*#EXT-X-/.test(t))) report(u, 'xhr-m3u8');
              else extractVideoUrlFromJson(t, 'xhr');
            } catch(e){}
          });
        } catch(e){}
        return origOpen.apply(this, arguments);
      };

      // ── hook Response.prototype.text (Kazumi 关键技巧) ────────────────
      // 很多站点 fetch(url).then(r => r.text()) 取 m3u8，拦 text() 能拿到解压后明文。
      // 比直接拦 fetch 的 URL 更可靠——能命中压缩传输 / 动态生成 / 第三方库(axios)请求。
      // 即使上层 fetch hook 被绕过(库提前缓存了原始 fetch)，text() 是最终读取口，必经此处。
      try {
        var origText = Response.prototype.text;
        if (origText) {
          Response.prototype.text = function(){
            var self = this;
            var p = origText.apply(this, arguments);
            return p.then(function(t){
              try {
                if (t && (t.slice(0, 7) === '#EXTM3U' || /^\s*#EXT-X-/.test(t))) {
                  var ru = (self && self.url) || '';
                  if (ru) report(ru, 'resp.text:m3u8');
                  else {
                    // URL 丢失时从 manifest 内容反推绝对分片地址
                    extractVideoUrlFromJson(t, 'resp.text:m3u8-nourl');
                  }
                } else {
                  extractVideoUrlFromJson(t, 'resp.text');
                }
              } catch(e){}
              return t;
            });
          };
        }
      } catch(e){}

      // ── hook Response.prototype.json (部分源用 JSON API 返回视频 URL) ──
      try {
        var origJson = Response.prototype.json;
        if (origJson) {
          Response.prototype.json = function(){
            var self = this;
            var p = origJson.apply(this, arguments);
            return p.then(function(j){
              try {
                var s = JSON.stringify(j);
                if (s) extractVideoUrlFromJson(s, 'resp.json');
              } catch(e){}
              return j;
            });
          };
        }
      } catch(e){}

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

      // ── scan <video>/<source>/<iframe> (existing + mutations) ─────────
      function scan(node){
        if (!node || node.nodeType !== 1) return;
        var tag = node.tagName;
        if (tag === 'VIDEO' || tag === 'SOURCE') consider(node.src || node.getAttribute('src'), 'dom');
        if (tag === 'IFRAME') {
          var fs = node.src || node.getAttribute('src');
          consider(fs, 'iframe');
          // 若 iframe src 本身携带视频参数，直接解出
          var inner = unwrapPlayerUrl(fs);
          if (inner) report(inner, 'iframe:param');
          monitorIframe(node);
        }
        if (node.querySelectorAll) {
          node.querySelectorAll('video,source').forEach(function(el){ consider(el.src || el.getAttribute('src'), 'dom'); });
          node.querySelectorAll('iframe').forEach(function(el){
            consider(el.src || el.getAttribute('src'), 'iframe');
            var inner2 = unwrapPlayerUrl(el.src || el.getAttribute('src'));
            if (inner2) report(inner2, 'iframe:param');
            monitorIframe(el);
          });
        }
      }

      // 同域 iframe：等待加载后尝试注入/扫描其内部
      function monitorIframe(el){
        try {
          if (!el.addEventListener) return;
          el.addEventListener('load', function(){
            try {
              var doc = el.contentDocument;
              if (doc) {
                scan(doc.documentElement);
                // 递归扫描嵌套 iframe
                var iframes = doc.querySelectorAll('iframe');
                iframes.forEach(monitorIframe);
              }
            } catch(e){ /* cross-origin */ }
          });
        } catch(e){}
      }

      try {
        var mo = new MutationObserver(function(ms){ ms.forEach(function(m){ m.addedNodes.forEach(scan); }); });
        mo.observe(document.documentElement, { childList: true, subtree: true });
      } catch(e){}

      function init(){
        checkPageUrl();
        checkGlobalPlayers();
        checkPerformanceEntries();
        document.querySelectorAll('video,source').forEach(function(el){ consider(el.src || el.getAttribute('src'), 'dom.init'); });
        document.querySelectorAll('iframe').forEach(function(el){
          consider(el.src || el.getAttribute('src'), 'iframe.init');
          var inner = unwrapPlayerUrl(el.src || el.getAttribute('src'));
          if (inner) report(inner, 'iframe.init:param');
          monitorIframe(el);
        });
        document.querySelectorAll('video').forEach(function(v){
          if (v.currentSrc && !v.currentSrc.startsWith('blob:')) consider(v.currentSrc, 'currentSrc');
        });
      }
      if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
      else init();
      // periodic re-scan for late dynamic players (Kazumi does this too)
      try {
        initTimer = setInterval(function(){
          init();
          checkGlobalPlayers();
        }, 800);
      } catch(e){}
    })();
    "##.to_string()
}

/// 来源可信度：实际验证过 m3u8 内容的最高；URL 模式匹配次之；DOM/media 再次。
fn source_score(source: &str) -> i32 {
    if source.contains("m3u8-body")
        || source.contains("resp.text:m3u8")
        || source.contains("fetch-m3u8")
        || source.contains("xhr-m3u8")
    {
        100
    } else if source.contains("webresource") {
        90
    } else if source.contains("fetch") || source.contains("xhr") {
        80
    } else if source.contains("performance") || source.contains("websocket") {
        75
    } else if source.contains("media")
        || source.contains("dplayer")
        || source.contains("artplayer")
        || source.contains("videojs")
        || source.contains("plyr")
    {
        70
    } else if source.contains("dom")
        || source.contains("source-el")
        || source.contains("currentSrc")
    {
        60
    } else if source.contains("iframe") || source.contains("page-url") {
        50
    } else {
        40
    }
}

fn url_score(url: &str) -> i32 {
    let lower = url.to_lowercase();
    if lower.contains(".m3u8") || lower.contains("/m3u8") {
        20
    } else if lower.contains(".mp4") {
        15
    } else if lower.contains("/hls/") || lower.contains("/dash/") || lower.contains(".mpd") {
        10
    } else {
        0
    }
}

fn is_better_result(new: &VideoUrlResult, old: &VideoUrlResult) -> bool {
    source_score(&new.source) + url_score(&new.url)
        > source_score(&old.source) + url_score(&old.url)
}

/// Shared implementation: open a hidden window, inject the sniffer, wait for a
/// sentinel navigation carrying the found URL (or time out).
/// Triple detection:
///   1. JS injection in the main frame (fetch/XHR/DOM).
///   2. `on_web_resource_request` for **all frames** (covers cross-origin iframes
///      where the initialization script can't run).
///   3. Periodic JS polling as a safety net.
///
/// 找到 URL 后不会立即返回，而是继续等待 SETTLE_DURATION，让页面完成「激活回调」
/// 并有机会上报更优/更晚的地址（参考 Kazumi issue #117）。
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

    // 用共享 best result + Notify 替代 oneshot：找到结果后继续 settle，可更新为更优地址。
    let best: Arc<Mutex<Option<VideoUrlResult>>> = Arc::new(Mutex::new(None));
    let notify = Arc::new(tokio::sync::Notify::new());

    let best_nav = best.clone();
    let notify_nav = notify.clone();
    let best_resource = best.clone();
    let notify_resource = notify.clone();

    let current_url = Arc::new(Mutex::new(episode_url.clone()));
    let current_url_nav = current_url.clone();
    let current_url_resource = current_url.clone();

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
            tracing::info!(
                "[sniff] 页面加载事件: event={:?} url={}",
                payload.event(),
                payload.url()
            );
            // 更新当前页面 URL（处理重定向）
            if let Ok(mut guard) = current_url.lock() {
                *guard = payload.url().to_string();
                tracing::debug!("[sniff] 更新当前页面 URL: {}", *guard);
            }
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
                    // 使用当前页面 URL 作为 tab_url（用于 Referer），保留重定向后的真实地址
                    let tab_url = if let Ok(url_guard) = current_url_nav.lock() {
                        url_guard.clone()
                    } else {
                        String::new()
                    };
                    let candidate = VideoUrlResult {
                        url: found,
                        source,
                        tab_url,
                    };
                    let mut guard = best_nav.lock().unwrap();
                    if guard
                        .as_ref()
                        .map_or(true, |old| is_better_result(&candidate, old))
                    {
                        tracing::info!(
                            "[sniff] sentinel 记录候选: source={}, url={}",
                            candidate.source,
                            candidate.url
                        );
                        *guard = Some(candidate);
                        notify_nav.notify_one();
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
            let mut candidate: Option<VideoUrlResult> = None;

            if is_video_stream_url(&url) && !is_ad_url(&url) {
                candidate = Some(VideoUrlResult {
                    url: url.clone(),
                    source: "webresource:url".into(),
                    tab_url: current_url_resource
                        .lock()
                        .map(|g| g.clone())
                        .unwrap_or_default(),
                });
            } else {
                // 检查响应内容：m3u8 master/media playlist 通常以 #EXTM3U 开头。
                let body = response.body().as_ref();
                if body.len() >= 7 && &body[..7] == b"#EXTM3U" && !is_ad_url(&url) {
                    candidate = Some(VideoUrlResult {
                        url: url.clone(),
                        source: "webresource:m3u8-body".into(),
                        tab_url: current_url_resource
                            .lock()
                            .map(|g| g.clone())
                            .unwrap_or_default(),
                    });
                }
            }

            if let Some(c) = candidate {
                tracing::info!(
                    "[sniff] web_resource 命中: source={}, url={}",
                    c.source,
                    c.url
                );
                let mut guard = best_resource.lock().unwrap();
                if guard.as_ref().map_or(true, |old| is_better_result(&c, old)) {
                    *guard = Some(c);
                    notify_resource.notify_one();
                }
            }
        })
        .build()
        .map_err(|e| format!("创建提取窗口失败: {}", e))?;

    // Spawn a polling task that checks window.__MOEPLAY_VIDEO_URL__ every 250ms.
    // Uses eval() to trigger sentinel navigation from JS side when URL found.
    let app_poll = app.clone();
    let label_poll = label.clone();
    let poll_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        interval.tick().await; // skip first immediate tick
        for _ in 0..140 {
            // max 35s (140 * 250ms) — overshoot SNIFF_TIMEOUT slightly so sentinel has time
            interval.tick().await;
            // Use eval to check the global var and trigger sentinel nav if found
            if let Some(w) = app_poll.get_webview_window(&label_poll) {
                let check_js = r#"
                (function(){
                  var u = window.__MOEPLAY_VIDEO_URL__;
                  if (u && u.length > 5) {
                    // URL 发生变化时再次发送 sentinel，让 Rust 在 settle 窗口内拿到更优/更晚地址
                    if (window.__MOEPLAY_SENTINEL_URL__ !== u) {
                      window.__MOEPLAY_SENTINEL_URL__ = u;
                      try {
                        location.href = 'https://moeplay.invalid/__moevideo__?s=poll&u=' + encodeURIComponent(u);
                      } catch(e) {}
                    }
                    return 'found:' + u;
                  }
                  return 'not_found';
                })()
                "#;
                match w.eval(check_js) {
                    Ok(_) => {}
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

    // Phase 1: 等待首个候选 URL
    let first_found = tokio::time::timeout(SNIFF_TIMEOUT, notify.notified()).await;

    // Phase 2: settle 窗口，继续收集更优/更晚的地址，并给页面激活回调留出时间
    let result = match first_found {
        Ok(()) => {
            tracing::info!(
                "[sniff] 找到候选，进入 {}ms settle 窗口",
                SETTLE_DURATION.as_millis()
            );
            tokio::time::sleep(SETTLE_DURATION).await;
            best.lock().unwrap().clone()
        }
        Err(_) => {
            tracing::error!(
                "[sniff] 嗅探超时 ({}s), label={}",
                SNIFF_TIMEOUT.as_secs(),
                label_log
            );
            None
        }
    };

    // Cleanup — 不主动关闭窗口！
    // wry 0.55 在销毁隐藏 WebView2 窗口时会 null pointer panic，
    // 即使 catch_unwind 捕获了，也会破坏 UI 事件循环状态，
    // 导致后续的 Svelte 响应式更新失效。
    // 窗口保持隐藏，在应用退出时自动清理。
    poll_handle.abort();

    match result {
        Some(v) => {
            tracing::info!("[sniff] 嗅探成功: url={}, source={}", v.url, v.source);
            Ok(v)
        }
        None => Err("video-url-timeout".into()),
    }
}

/// 很多源把真实流地址塞在播放器页 URL 的查询参数里，例如
/// `.../artplayer/index.html?url=https://cdn/.../index.m3u8`。
/// 把内层真实视频地址解出来（最多解三层，防嵌套/死循环），并支持双重 URL 编码 / base64。
fn unwrap_player_url(raw: &str) -> String {
    let mut current = raw.to_string();
    for depth in 0..3 {
        let parsed = match url::Url::parse(&current) {
            Ok(u) => u,
            Err(_) => break,
        };
        let mut candidates: Vec<(String, &str)> = Vec::new();
        for (k, v) in parsed.query_pairs() {
            let key = k.to_ascii_lowercase();
            let val = fully_decode_value(&v);
            if !val.starts_with("http") {
                continue;
            }
            // 视频专用 key：可信度最高
            let is_video_key = matches!(
                key.as_str(),
                "m3u8"
                    | "playurl"
                    | "video"
                    | "videourl"
                    | "stream"
                    | "play_url"
                    | "mediaurl"
                    | "play"
                    | "vurl"
                    | "vid"
                    | "dash"
                    | "hls"
                    | "player"
                    | "dplayer"
                    | "artplayer"
                    | "ckplayer"
                    | "videojs"
                    | "plyr"
                    | "video_url"
                    | "m3u8_url"
                    | "api"
                    | "data"
                    | "jx"
            );
            // 通用 key：需要值本身像视频地址
            let is_generic_key = matches!(
                key.as_str(),
                "url" | "v" | "src" | "link" | "file" | "source" | "media"
            );
            if is_video_key {
                candidates.push((val, "video_key"));
            } else if is_generic_key && is_video_stream_url(&val) {
                candidates.push((val, "generic_key"));
            }
        }
        // 优先取可信度高的；同可信度取最长（通常更具体）
        candidates.sort_by(|a, b| {
            let score = |t: &str| match t {
                "video_key" => 2,
                "generic_key" => 1,
                _ => 0,
            };
            let sa = score(a.1);
            let sb = score(b.1);
            if sa != sb {
                sb.cmp(&sa)
            } else {
                b.0.len().cmp(&a.0.len())
            }
        });
        match candidates.into_iter().next() {
            Some((v, _)) if v != current => {
                tracing::debug!("[unwrap] depth={} 解出内层 URL: {}", depth, v);
                current = v;
            }
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

    // 1. 裸 URL（支持 url 参数中嵌套编码的 URL）
    let re_url =
        Regex::new(r#"https?://[^\s"'<>]+?\.(?:m3u8|mp4|mpd|flv|mkv|webm|mov)(?:\?[^\s"'<>]*)?"#)
            .unwrap();
    for m in re_url.find_iter(&text) {
        let u = m.as_str();
        if !is_ad_url(u) {
            let decoded = fully_decode_value(u);
            return Ok(VideoUrlResult {
                url: decoded,
                source: "legacy:url".into(),
                tab_url: episode_url.to_string(),
            });
        }
    }

    // 2. JSON/JS 中常见的 url/src/video/playUrl 字段
    let re_json = Regex::new(r#"["'](?:url|src|video|playUrl|play_url|m3u8|mp4|stream|link|file|source|hls|dash)["']\s*[:=]\s*["'](https?://[^"']+?)["']"#).unwrap();
    for cap in re_json.captures_iter(&text) {
        if let Some(u) = cap.get(1) {
            let u = u.as_str();
            if !is_ad_url(u) {
                let decoded = fully_decode_value(u);
                return Ok(VideoUrlResult {
                    url: decoded,
                    source: "legacy:json".into(),
                    tab_url: episode_url.to_string(),
                });
            }
        }
    }

    // 3. URL 查询参数形式的播放器页，尝试从 URL 本身解出
    if let Some(u) = unwrap_player_url_if_stream(episode_url) {
        return Ok(VideoUrlResult {
            url: u,
            source: "legacy:page-url".into(),
            tab_url: episode_url.to_string(),
        });
    }

    Err("传统解析未找到视频地址".into())
}

/// 若 episode_url 本身携带内层视频地址则直接返回，否则 None
fn unwrap_player_url_if_stream(raw: &str) -> Option<String> {
    let unwrapped = unwrap_player_url(raw);
    if unwrapped != raw && is_video_stream_url(&unwrapped) {
        Some(unwrapped)
    } else {
        None
    }
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

    // 解出内层真实流地址；Referer 优先用嗅探到的最终页面 URL（含重定向），
    // 仅当嗅探未提供时才回退到原始 episode_url。这样 CDN 防盗链成功率更高。
    let real = unwrap_player_url(&result.url);
    if result.tab_url.is_empty() {
        result.tab_url = episode_url;
    }
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

// 引入 base64 crate 依赖时可用；若未引入，使用简化 base64 解码。
// 这里使用标准库兼容实现，避免增加新依赖。
mod base64 {
    pub fn decode(input: &str) -> Result<Vec<u8>, ()> {
        let mut chars = input.chars().filter(|c| !c.is_whitespace()).peekable();
        let mut out = Vec::with_capacity(input.len() / 4 * 3);
        let table = |c: char| -> Result<u8, ()> {
            match c {
                'A'..='Z' => Ok(c as u8 - b'A'),
                'a'..='z' => Ok(c as u8 - b'a' + 26),
                '0'..='9' => Ok(c as u8 - b'0' + 52),
                '+' => Ok(62),
                '/' => Ok(63),
                '=' => Ok(0),
                _ => Err(()),
            }
        };

        while chars.peek().is_some() {
            let mut buf = [0u8; 4];
            let mut pad = 0usize;
            for i in 0..4 {
                match chars.next() {
                    Some('=') | None => {
                        pad += 1;
                    }
                    Some(c) => buf[i] = table(c)?,
                }
            }
            if pad > 2 {
                return Err(());
            }
            let triple = ((buf[0] as u32) << 18)
                | ((buf[1] as u32) << 12)
                | ((buf[2] as u32) << 6)
                | (buf[3] as u32);
            out.push((triple >> 16) as u8);
            if pad < 2 {
                out.push(((triple >> 8) & 0xFF) as u8);
            }
            if pad < 1 {
                out.push((triple & 0xFF) as u8);
            }
        }
        Ok(out)
    }
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
        let raw =
            "https://player.example.com/?url=https%253A%252F%252Fcdn.example.com%252Fplaylist.m3u8";
        assert_eq!(
            unwrap_player_url(raw),
            "https://cdn.example.com/playlist.m3u8"
        );
    }

    #[test]
    fn unwrap_player_url_prefers_video_keys() {
        let raw = "https://player.example.com/?foo=https://other.com/page.html&m3u8=https%3A%2F%2Fcdn.example.com%2Fplaylist.m3u8";
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
    fn fully_decode_value_handles_base64_url() {
        let raw = "aHR0cHM6Ly9jZG4uZXhhbXBsZS5jb20vcGxheWxpc3QubTN1OA==";
        assert_eq!(
            fully_decode_value(raw),
            "https://cdn.example.com/playlist.m3u8"
        );
    }

    #[test]
    fn is_video_stream_url_recognises_common_manifests() {
        assert!(is_video_stream_url("https://cdn.example.com/playlist.m3u8"));
        assert!(is_video_stream_url(
            "https://cdn.example.com/hls/123/index.m3u8"
        ));
        assert!(is_video_stream_url("https://cdn.example.com/video.mp4"));
        assert!(!is_video_stream_url("https://example.com/page.html"));
        assert!(!is_video_stream_url("blob:https://example.com/abc"));
        assert!(!is_video_stream_url(
            "https://googleads.g.doubleclick.net/pagead/id"
        ));
    }
}
