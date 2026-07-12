<script lang="ts">
  import Hls from "hls.js";
  import { invokeCmd } from "../../api/core";
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { animeStore } from "../../stores/anime.svelte";
  import { settingsStore } from "../../stores/settings.svelte";
  import { reassertNativeFullscreen } from "../../utils/window-fullscreen";
  import Icon from "../Icon.svelte";
  import DanmakuOverlay from "./DanmakuOverlay.svelte";
  import { animeDownloadEpisode } from "../../api";
  import { debugLog } from "../../utils/debug";
  import { AsyncState } from "../ui-v2";
  import { focusTrap } from "../../actions/a11y/focusTrap";
  import { focusWhenAvailable } from "./a11y";
  import { AnimePlaybackShell } from "./playback";

  const status = $derived(animeStore.playerExtractStatus); // extracting | found | timeout | error
  const videoSrc = $derived(animeStore.playerVideoSrc);
  const isM3u8 = $derived(animeStore.playerIsM3u8);
  const pageUrl = $derived(animeStore.playerWebUrl || animeStore.playerPageUrl || animeStore.playerUrl); // 源站播放页（解析失败时降级用）
  const nativePageUrl = $derived(animeStore.playerPageUrl || animeStore.playerUrl);
  const failureKind = $derived(animeStore.playerFailureKind);
  const failureMessage = $derived(animeStore.playerFailureMessage);
  const epName = $derived(animeStore.playerEpisodeName);
  const roads = $derived(animeStore.roads);
  const roadIdx = $derived(animeStore.playerRoadIdx);
  const epIdx = $derived(animeStore.playerEpisodeIdx);
  const road = $derived(roads[roadIdx]);
  const hasPrev = $derived(epIdx > 0 && status !== 'extracting');
  const hasNext = $derived(road ? epIdx < road.episodes.length - 1 && status !== 'extracting' : false);
  const playbackSourceLabel = $derived(animeStore.playerRuleName || animeStore.detailRuleName || '规则源');
  const episodePosition = $derived(road ? `${road.name} · ${epIdx + 1} / ${road.episodes.length}` : '');
  const nextEpisodeTitle = $derived(hasNext ? (road?.episodes[epIdx + 1]?.name ?? '') : '');

  // 换源自愈状态
  const failoverStatus = $derived(animeStore.failoverStatus);
  const failoverMessage = $derived(animeStore.failoverMessage);
  const failoverTotal = $derived(animeStore.failoverTotal);
  const failoverCurrent = $derived(animeStore.failoverCurrent);
  const playerStatusTitle = $derived.by(() => {
    if (failoverStatus === 'trying') return '正在自动换源';
    if (status === 'extracting') return extractStep;
    if (status === 'timeout') return '解析超时';
    if (failoverStatus === 'allFailed') return '备用来源均不可用';
    if (status === 'error') return '未能提取到视频地址';
    return '播放器就绪';
  });
  const playerStatusDescription = $derived.by(() => {
    if (failoverStatus === 'trying') return failoverMessage || `正在尝试第 ${failoverCurrent} / ${failoverTotal} 个来源。`;
    if (status === 'extracting') return `从播放页提取真实视频流 · ${extractElapsed}s / 30s`;
    if (failoverStatus === 'allFailed') return `已自动尝试 ${failoverTotal} 个备用源，可手动选源或使用网页播放。`;
    if (status === 'timeout') return '播放页响应过慢或被反爬拦截，可重试、换源或用网页播放。';
    return failureMessage || '视频地址提取失败，可重试、换源或用网页播放。';
  });

  // 弹幕状态
  const danmakuComments = $derived(animeStore.danmakuComments);
  const danmakuEnabled = $derived(animeStore.danmakuEnabled);
  const danmakuLoading = $derived(animeStore.danmakuLoading);

  // 章节评论状态
  const episodeComments = $derived(animeStore.episodeComments);
  const episodeCommentsLoading = $derived(animeStore.episodeCommentsLoading);

  let videoEl = $state<HTMLVideoElement | null>(null);
  let overlayEl = $state<HTMLDivElement | null>(null);
  let useWebFallback = $state(false); // 用户选择「用网页播放」时加载站点自带播放器
  let webFrameLoaded = $state(false);
  let webFrameTimedOut = $state(false);
  let webFrameKey = $state(0);
  let isFullscreen = $state(false);
  let hostWindowWasFullscreen = $state(false);
  let restoreFullscreenTimer: number | null = null;
  let fullscreenGuardTimer: number | null = null;
  let currentTime = $state(0);
  let mediaAspectRatio = $state(16 / 9);
  let showCommentsPanel = $state(false);
  let commentsPanelTab = $state<'comments' | 'danmaku'>('comments');
  let downloading = $state(false);
  let downloadMsg = $state('');

  // 提取进度反馈
  let extractElapsed = $state(0);
  let extractTimer: number | null = null;
  const extractStep = $derived(
    extractElapsed < 3 ? '正在连接播放页…' :
    extractElapsed < 8 ? '正在嗅探视频流…' :
    extractElapsed < 13 ? '正在提取真实地址…' :
    '等待响应，可能较慢…'
  );

  // 播放器设置
  const pendingSeekMs = $derived(animeStore.pendingSeekMs);
  const autoNext = $derived(animeStore.autoNext);
  let playbackRate = $state(animeStore.playbackRate);
  let showSpeedMenu = $state(false);
  let showDanmakuSettings = $state(false);
  const speedOptions = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0];

  // 手势状态
  let gestureActive = $state(false);
  let gestureStartX = $state(0);
  let gestureStartY = $state(0);
  let gestureType = $state<'none' | 'brightness' | 'volume' | 'seek'>('none');
  let gestureValue = $state(0);
  let gestureLabel = $state('');
  let longPressTimer = $state<number | null>(null);
  let isLongPressing = $state(false);
  let brightness = $state(1);

  // ── 选集面板 ──
  let showEpisodePanel = $state(false);
  let pickerRoadIdx = $state(0);
  const pickerEpisodes = $derived(roads[pickerRoadIdx]?.episodes ?? []);

  function toggleEpisodePanel() {
    if (!showEpisodePanel) {
      pickerRoadIdx = roadIdx; // 打开时定位到当前线路
      showCommentsPanel = false; // 两个右侧面板互斥
    }
    showEpisodePanel = !showEpisodePanel;
  }

  function pickEpisode(ri: number, ei: number) {
    if (ri === roadIdx && ei === epIdx) { showEpisodePanel = false; return; }
    useWebFallback = false;
    showEpisodePanel = false;
    // 切线路时保持当前集的进度
    const seekMs = (ri !== roadIdx && ei === epIdx && videoEl) ? Math.floor(videoEl.currentTime * 1000) : undefined;
    animeStore.playEpisode(ri, ei, seekMs);
  }

  // ── PiP (画中画) ────────────────────────────────────────────────────────
  let isPipSupported = $state(false);
  let isPipActive = $state(false);

  onMount(() => {
    document.addEventListener('fullscreenchange', onFullscreenChange);
    document.addEventListener('keydown', onKeyDown);
    isPipSupported = !!document.pictureInPictureEnabled;
    hostWindowWasFullscreen = ["fullscreen", "big-picture"].includes(settingsStore.settings.startup_mode ?? "fullscreen");
    try {
      getCurrentWindow().isFullscreen().then((value) => {
        hostWindowWasFullscreen ||= value;
        if (hostWindowWasFullscreen) void restoreHostWindowFullscreen();
      }).catch(() => {});
      fullscreenGuardTimer = window.setInterval(() => { void restoreHostWindowFullscreen(); }, 320);
    } catch { /* 浏览器预览环境 */ }
  });
  onDestroy(() => {
    document.removeEventListener('fullscreenchange', onFullscreenChange);
    document.removeEventListener('keydown', onKeyDown);
    if (extractTimer) clearInterval(extractTimer);
    if (restoreFullscreenTimer) clearTimeout(restoreFullscreenTimer);
    if (fullscreenGuardTimer) clearInterval(fullscreenGuardTimer);
  });

  // 离开 found 状态时关闭视频相关弹出面板
  $effect(() => {
    if (status !== 'found') {
      showSpeedMenu = false;
      showDanmakuSettings = false;
      showCommentsPanel = false;
    }
  });

  const currentRule = $derived(animeStore.rules.find(r => r.name === animeStore.playerRuleName));
  const prefersWebPlayback = $derived(!!currentRule && currentRule.useNativePlayer === false);

  $effect(() => {
    const active = useWebFallback;
    const url = pageUrl;
    if (!active || !url) return;
    webFrameLoaded = false;
    webFrameTimedOut = false;
    const timer = window.setTimeout(() => {
      if (!webFrameLoaded) {
        webFrameTimedOut = true;
        animeStore.markPlayerFailure('iframeBlocked', '源站禁止嵌入播放或加载超时，可用浏览器打开');
      }
    }, 12_000);
    return () => clearTimeout(timer);
  });

  // 只有明确禁用原生播放器的源，才直接切源站播放器；useWebview 仅代表规则允许网页能力。
  $effect(() => {
    if ((status === 'extracting' || status === 'error' || status === 'timeout') && !useWebFallback && pageUrl && prefersWebPlayback) {
      console.log('[播放器] 规则要求网页播放，自动切换源站播放器');
      invokeCmd('frontend_log', { level: 'info', message: '[播放器] 规则要求网页播放，自动切换源站播放器' }).catch(() => {});
      switchToWebFallback(status === 'extracting');
    }
  });

  // 全局兜底：只要提取失败/超时且用户启用了自动网页播放，就自动切到网页播放。
  // 这是 Kazumi 风格的兼容策略——当内置解析搞不定时，直接用源站播放器。
  $effect(() => {
    if ((status === 'error' || status === 'timeout') && !useWebFallback && pageUrl && animeStore.autoWebFallback) {
      console.log('[播放器] 内置解析失败，自动切换网页播放兜底');
      invokeCmd('frontend_log', { level: 'info', message: '[播放器] 内置解析失败，自动切换网页播放兜底' }).catch(() => {});
      switchToWebFallback();
    }
  });

  // 提取进度计时器
  $effect(() => {
    if (status === 'extracting' && failoverStatus !== 'trying') {
      extractElapsed = 0;
      if (extractTimer) clearInterval(extractTimer);
      extractTimer = window.setInterval(() => { extractElapsed += 1; }, 1000);
    } else {
      if (extractTimer) { clearInterval(extractTimer); extractTimer = null; }
    }
  });

  // 当视频元素可用时检测 PiP 支持 & 监听事件
  $effect(() => {
    const el = videoEl;
    if (!el) return;

    const onEnter = () => { isPipActive = true; };
    const onLeave = () => { isPipActive = false; };
    el.addEventListener('enterpictureinpicture', onEnter);
    el.addEventListener('leavepictureinpicture', onLeave);

    return () => {
      el.removeEventListener('enterpictureinpicture', onEnter);
      el.removeEventListener('leavepictureinpicture', onLeave);
    };
  });

  async function togglePip() {
    const el = videoEl;
    if (!el) return;
    try {
      if (document.pictureInPictureElement) {
        await document.exitPictureInPicture();
      } else {
        await el.requestPictureInPicture();
      }
    } catch (e) {
      console.warn('PiP toggle failed:', e);
    }
  }

  function switchToWebFallback(cancelRunningExtract = false) {
    if (!pageUrl) return;
    webFrameLoaded = false;
    webFrameTimedOut = false;
    webFrameKey += 1;
    useWebFallback = true;
    if (cancelRunningExtract && (status === 'extracting' || failoverStatus === 'trying')) {
      animeStore.cancelExtract();
    }
  }

  function shouldPreserveHostFullscreen() {
    return hostWindowWasFullscreen || ["fullscreen", "big-picture"].includes(settingsStore.settings.startup_mode ?? "fullscreen");
  }

  async function restoreHostWindowFullscreen(force = false) {
    if (!shouldPreserveHostFullscreen()) return;
    try { await reassertNativeFullscreen(getCurrentWindow(), force); }
    catch { /* 浏览器预览环境 */ }
  }

  function scheduleHostFullscreenRestore(delay = 100, force = false) {
    if (!shouldPreserveHostFullscreen()) return;
    if (restoreFullscreenTimer) clearTimeout(restoreFullscreenTimer);
    restoreFullscreenTimer = window.setTimeout(() => { void restoreHostWindowFullscreen(force); }, delay);
  }

  async function setPlayerFullscreen(next: boolean) {
    if (next === isFullscreen) return;

    showSpeedMenu = false;
    showDanmakuSettings = false;
    showEpisodePanel = false;
    showCommentsPanel = false;

    // Player fullscreen is intentionally CSS/DOM scoped. Never mutate the Tauri
    // application window here: a click inside the player must not restore or resize
    // the entire MoePlay window.
    await restoreHostWindowFullscreen();
    isFullscreen = next;
    if (!next && document.fullscreenElement && overlayEl?.contains(document.fullscreenElement)) {
      try { await document.exitFullscreen(); } catch {}
    }
    if (!next) await restoreHostWindowFullscreen(true);
    scheduleHostFullscreenRestore(next ? 60 : 420, !next);
  }

  function toggleFullscreen() {
    void setPlayerFullscreen(!isFullscreen);
  }

  async function closePlayer() {
    const returnRoad = roadIdx;
    const returnEpisode = epIdx;
    await setPlayerFullscreen(false);
    animeStore.closePlayer();
    // SourceSheet remains mounted while the player is active. Reopen its retained
    // episode step so keyboard users return to the exact episode that launched playback.
    animeStore.sourceSheetOpen = true;
    focusWhenAvailable(`[data-episode-key="${returnRoad}-${returnEpisode}"]`);
  }

  function handlePlayerEscape() {
    if (isFullscreen) toggleFullscreen();
    else void closePlayer();
  }
  async function onFullscreenChange() {
    // WebView2 退出 video/iframe 的 DOM fullscreen 时有机会把 Tauri 宿主窗口一并还原成带边框窗口。
    // 播放器只管理自身展示；若进入播放器前宿主就是全屏，则在 DOM fullscreen 结束后恢复宿主状态。
    if (!document.fullscreenElement) scheduleHostFullscreenRestore(120, true);
  }

  // HLS.js ↔ 原生双模兜底：加载阶段和播放阶段各自有看门狗，避免灰屏/黑屏空转。
  $effect(() => {
    const el = videoEl;
    const src = videoSrc;
    const m3u8 = isM3u8;
    invokeCmd('frontend_log', { level: 'info', message: `[播放器$effect] el=${!!el} status=${status} src=${src ? src.substring(0, 60) : 'null'}` }).catch(() => {});
    if (!el || status !== "found" || !src) return;
    const v: HTMLVideoElement = el;
    debugLog("[播放器] 初始化视频", { src: src.substring(0, 120), m3u8 });
    invokeCmd('frontend_log', { level: 'info', message: `[播放器] 初始化视频: m3u8=${m3u8}, src=${src.substring(0, 80)}` }).catch(() => {});

    let hls: Hls | null = null;
    let netRetry = 0;       // 网络错误重试次数（封顶防死循环）
    let recoverCount = 0;   // 媒体错误恢复次数
    let attempt = 0;        // 0=未开始 1=首选方式 2=兜底方式
    let settled = false;    // 已成功加载到元数据 或 已最终判 error —— 之后不再做初次兜底
    let watchdog: number | null = null;
    let playbackWatchdog: number | null = null;
    const nativeHls = v.canPlayType("application/vnd.apple.mpegurl") !== "";
    // 首选方式：能用 hls.js 且看着像 m3u8 就先 hls，否则先原生
    const firstIsHls = m3u8 && !nativeHls && Hls.isSupported();

    const clearWatchdog = () => { if (watchdog !== null) { clearTimeout(watchdog); watchdog = null; } };
    const clearPlaybackWatchdog = () => {
      if (playbackWatchdog !== null) {
        clearTimeout(playbackWatchdog);
        playbackWatchdog = null;
      }
    };
    const armWatchdog = () => {
      clearWatchdog();
      // 10s 内拿不到元数据视为这条 src 放不出来（黑屏静默失败的兜底信号）
      watchdog = window.setTimeout(() => {
        if (settled) return;
        if (v.readyState >= 1) return; // 已有元数据，别误杀慢源
        console.warn("[播放器] 10s 未加载到元数据，触发兜底");
        fail("timeout");
      }, 10000);
    };

    const markPlaybackReady = () => {
      if (v.readyState >= 3 || v.currentTime > 0) clearPlaybackWatchdog();
    };

    const armPlaybackWatchdog = () => {
      clearPlaybackWatchdog();
      // 有些反爬/CDN 会让 video 拿到元数据但永远没有可播放帧，表现为黑屏或灰屏。
      playbackWatchdog = window.setTimeout(() => {
        if (!settled) return;
        if (v.readyState >= 3 || v.currentTime > 0) return;
        console.warn('[播放器] 元数据已加载但长时间无可播放帧，触发兜底');
        fail('playback stalled');
      }, 15000);
    };

    // 成功拿到元数据：标记 settled，停掉看门狗
    const succeed = () => {
      settled = true;
      clearWatchdog();
      armPlaybackWatchdog();
    };

    // 加载失败：首次失败且还有备用方式 → 换方式；否则判 error 让用户换源/网页播放
    const fail = (why: string) => {
      clearWatchdog();
      clearPlaybackWatchdog();
      if (hls) { try { hls.destroy(); } catch {} hls = null; }
      const canTryAlternate = attempt < 2 && (!settled || (v.currentTime === 0 && v.readyState < 3));
      if (canTryAlternate) {
        console.warn(`[播放器] 第${attempt}次加载失败(${why})，自动切换播放方式兜底`);
        settled = false;
        v.removeAttribute("src");
        try { v.load(); } catch {}
        startAttempt();
      } else {
        console.error(`[播放器] 加载失败(${why})，尝试保进度自动换源`);
        settled = true;
        if (pageUrl) animeStore.invalidateVideoCache(pageUrl);
        const failureKind = why.includes('network') || why.includes('timeout') || why.includes('stalled')
          ? 'proxyHttp'
          : 'extractEncrypted';
        const recoveryStarted = animeStore.recoverPlaybackFailure(
          failureKind,
          `播放中断（${why}），正在尝试备用源`,
          Math.floor(v.currentTime * 1000),
        );
        if (recoveryStarted) {
          invokeCmd('frontend_log', { level: 'info', message: `[播放器] 播放失败(${why})，已启动自动换源` }).catch(() => {});
          return;
        }
        animeStore.playerExtractStatus = "error";
        if ((animeStore.autoWebFallback || prefersWebPlayback) && pageUrl) {
          invokeCmd('frontend_log', { level: 'info', message: '[播放器] 无可用备用源，自动切换网页播放兜底' }).catch(() => {});
          switchToWebFallback();
        }
      }
    };

    // 续播 + 倍速 + 跳片头：元数据就绪后执行
    const onLoadedMetadata = () => {
      debugLog("[播放器] loadedmetadata, duration:", v.duration);
      if (v.videoWidth > 0 && v.videoHeight > 0) mediaAspectRatio = v.videoWidth / v.videoHeight;
      succeed();
      v.playbackRate = playbackRate;
      if (pendingSeekMs > 0) {
        v.currentTime = pendingSeekMs / 1000;
        animeStore.pendingSeekMs = 0;
      } else if (animeStore.skipOpening > 0) {
        v.currentTime = animeStore.skipOpening;
      }
      v.play().catch(() => {});
    };
    v.addEventListener('loadedmetadata', onLoadedMetadata);

    // video 元素错误：加载后也要兜底，避免元数据就绪后黑屏卡死。
    const onVideoError = () => {
      const err = v.error;
      console.error("[播放器] video 元素错误:", err ? `code=${err.code} message=${err.message}` : "未知");
      fail("video error");
    };
    v.addEventListener('error', onVideoError);

    v.addEventListener('canplay', markPlaybackReady);
    v.addEventListener('playing', markPlaybackReady);
    v.addEventListener('timeupdate', markPlaybackReady);

    // 自动连播 + 跳片尾
    const onEnded = () => {
      debugLog("[播放器] 视频播放结束");
      if (autoNext && hasNext) {
        animeStore.nextEpisode();
      }
    };
    v.addEventListener('ended', onEnded);

    const onTimeUpdateForSkip = () => {
      if (animeStore.skipEnding > 0 && v.duration > 0 && v.currentTime >= v.duration - animeStore.skipEnding) {
        if (autoNext && hasNext) {
          animeStore.nextEpisode();
        }
      }
    };
    v.addEventListener('timeupdate', onTimeUpdateForSkip);

    function attachHls() {
      debugLog("[播放器] 使用 HLS.js 播放");
      // 从代理 URL 提取 Referer 供 xhrSetup 使用
      const extractReferer = (): string => {
        try {
          const u = new URL(src);
          const r = u.searchParams.get("referer");
          return r ? decodeURIComponent(r) : "";
        } catch { return ""; }
      };
      const hlsReferer = extractReferer();
      const hlsOrigin = hlsReferer ? (() => { try { return new URL(hlsReferer).origin; } catch { return ""; } })() : "";

      hls = new Hls({
        maxBufferLength: 60,
        maxMaxBufferLength: 120,
        backBufferLength: 30,
        enableWorker: true,
        lowLatencyMode: false,
        fragLoadingMaxRetry: 11,
        fragLoadingRetryDelay: 1000,
        fragLoadingMaxRetryTimeout: 64000,
        manifestLoadingMaxRetry: 6,
        manifestLoadingRetryDelay: 1000,
        levelLoadingMaxRetry: 6,
        nudgeMaxRetry: 10,
        xhrSetup: function(xhr: XMLHttpRequest) {
          if (hlsReferer) {
            try { xhr.setRequestHeader("Referer", hlsReferer); } catch {}
          }
          if (hlsOrigin) {
            try { xhr.setRequestHeader("Origin", hlsOrigin); } catch {}
          }
        }
      });
      hls.loadSource(src);
      hls.attachMedia(v);
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        debugLog("[播放器] HLS manifest 已解析，开始播放");
        v.play().catch(() => {});
      });
      // 致命错误要自愈而不是直接判死（旧逻辑一遇 fatal 就 error → 播一会儿就卡死、必须退出重进）
      hls.on(Hls.Events.ERROR, (_e, data) => {
        if (!data.fatal) {
          console.warn("[hls] 非致命错误", data.type, data.details);
          return;
        }
        console.error("[hls] 致命错误", data.type, data.details);
        switch (data.type) {
          case Hls.ErrorTypes.NETWORK_ERROR:
            if (netRetry++ < 6) {
              console.warn(`[hls] 网络错误恢复，第 ${netRetry} 次 startLoad`);
              hls?.startLoad();
            } else {
              fail("hls network");
            }
            break;
          case Hls.ErrorTypes.MEDIA_ERROR:
            if (recoverCount++ < 3) {
              console.warn(`[hls] 媒体错误恢复，第 ${recoverCount} 次 recoverMediaError`);
              hls?.recoverMediaError();
            } else {
              fail("hls media");
            }
            break;
          default:
            fail("hls other");
        }
      });
      armWatchdog();
    }

    function attachNative() {
      debugLog("[播放器] 原生 <video> 直接播放");
      v.src = src;
      try { v.load(); } catch {}
      v.play().catch(() => {});
      armWatchdog();
    }

    // 发起一次加载尝试：attempt 1 用首选方式，attempt 2 用另一种
    function startAttempt() {
      attempt++;
      const useHls = attempt === 1 ? firstIsHls : !firstIsHls;
      if (useHls && Hls.isSupported()) attachHls();
      else attachNative();
    }

    startAttempt();

    return () => {
      clearWatchdog();
      clearPlaybackWatchdog();
      v.removeEventListener('loadedmetadata', onLoadedMetadata);
      v.removeEventListener('error', onVideoError);
      v.removeEventListener('canplay', markPlaybackReady);
      v.removeEventListener('playing', markPlaybackReady);
      v.removeEventListener('timeupdate', markPlaybackReady);
      v.removeEventListener('ended', onEnded);
      v.removeEventListener('timeupdate', onTimeUpdateForSkip);
      if (hls) { try { hls.destroy(); } catch {} }
    };
  });

  function handleTimeUpdate() {
    if (videoEl) {
      currentTime = videoEl.currentTime;
      animeStore.updateProgress(Math.floor(videoEl.currentTime * 1000));
    }
  }
  function retry() {
    useWebFallback = false;
    webFrameLoaded = false;
    webFrameTimedOut = false;
    animeStore.playEpisode(roadIdx, epIdx);
  }

  function refreshWebFrame() {
    if (!pageUrl) return;
    webFrameLoaded = false;
    webFrameTimedOut = false;
    webFrameKey += 1;
  }

  function openInBrowser() {
    if (pageUrl) invokeCmd("open_url", { url: pageUrl }).catch(() => {});
  }

  async function switchSource() {
    await setPlayerFullscreen(false);
    animeStore.closePlayer();
    animeStore.openSourceSheet();
  }

  async function launchExternalPlayer() {
    const targetUrl = videoSrc || nativePageUrl || pageUrl;
    if (!targetUrl) return;
    try {
      const players = await invokeCmd<{ name: string; display_name: string; available: boolean }[]>("anime_get_external_players");
      const available = players.filter(p => p.available);
      if (available.length === 0) {
        alert("未检测到外部播放器（mpv / VLC / PotPlayer）");
        return;
      }
      // 优先选 mpv，否则选第一个可用的
      const player = available.find(p => p.name === "mpv") || available[0];
      const msg = await invokeCmd<string>("anime_launch_external_player", {
        url: targetUrl,
        player: player.name,
        referer: animeStore.playerReferer || animeStore.rules.find(r => r.name === animeStore.playerRuleName)?.baseUrl || null,
      });
      debugLog("External player:", msg);
    } catch (e) {
      console.warn("外部播放器启动失败:", e);
    }
  }
  function goPrev() { useWebFallback = false; webFrameLoaded = false; webFrameTimedOut = false; animeStore.prevEpisode(); }
  function goNext() { useWebFallback = false; webFrameLoaded = false; webFrameTimedOut = false; animeStore.nextEpisode(); }

  // 倍速切换
  function setPlaybackRate(rate: number) {
    playbackRate = rate;
    animeStore.playbackRate = rate;
    if (videoEl) videoEl.playbackRate = rate;
    showSpeedMenu = false;
  }

  function handleToolbarClickOutside(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (showSpeedMenu && !t.closest('.speed-control')) showSpeedMenu = false;
    if (showDanmakuSettings && !t.closest('.danmaku-settings-btn') && !t.closest('.danmaku-settings-panel')) showDanmakuSettings = false;
  }

  // 手势处理
  function onPointerDown(e: PointerEvent) {
    if (useWebFallback || status !== "found") return;
    const target = e.target as HTMLElement;
    if (target.closest('button') || target.closest('.comments-panel') || target.closest('.episodes-panel')) return;
    // 视频底部原生控制条区域不接管手势：否则拖进度条 / 按播放键会和手势(快进/长按倍速)打架。
    const vrect = videoEl?.getBoundingClientRect();
    if (vrect && e.clientY > vrect.bottom - 56) return;

    gestureActive = true;
    gestureStartX = e.clientX;
    gestureStartY = e.clientY;
    gestureType = 'none';
    gestureValue = 0;

    // 长按检测
    longPressTimer = window.setTimeout(() => {
      if (gestureActive && gestureType === 'none') {
        isLongPressing = true;
        if (videoEl) videoEl.playbackRate = animeStore.longPressRate;
      }
    }, 400);
  }

  function onPointerMove(e: PointerEvent) {
    if (!gestureActive) return;

    const dx = e.clientX - gestureStartX;
    const dy = e.clientY - gestureStartY;
    const absDx = Math.abs(dx);
    const absDy = Math.abs(dy);

    // 移动超过阈值则取消长按
    if (absDx > 10 || absDy > 10) {
      if (longPressTimer) {
        clearTimeout(longPressTimer);
        longPressTimer = null;
      }
      isLongPressing = false;
    }

    if (gestureType === 'none') {
      if (absDx < 15 && absDy < 15) return;
      // 判断手势类型
      if (absDy > absDx) {
        // 上下拖动
        const rect = overlayEl?.getBoundingClientRect();
        if (rect && gestureStartX < rect.left + rect.width / 2) {
          gestureType = 'brightness';
          gestureLabel = '亮度';
        } else {
          gestureType = 'volume';
          gestureLabel = '音量';
        }
      } else {
        gestureType = 'seek';
        gestureLabel = '';
      }
    }

    // 更新手势值
    if (gestureType === 'brightness') {
      const delta = -dy / 200;
      gestureValue = Math.max(0.2, Math.min(1, brightness + delta));
      gestureLabel = `亮度 ${Math.round(gestureValue * 100)}%`;
    } else if (gestureType === 'volume') {
      const delta = -dy / 200;
      gestureValue = Math.max(0, Math.min(1, (videoEl?.volume ?? 1) + delta));
      gestureLabel = `音量 ${Math.round(gestureValue * 100)}%`;
    } else if (gestureType === 'seek') {
      const delta = dx / 10; // 10px = 1s
      gestureValue = delta;
      gestureLabel = delta > 0 ? `+${delta.toFixed(0)}s` : `${delta.toFixed(0)}s`;
    }
  }

  function onPointerUp() {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }

    if (isLongPressing) {
      isLongPressing = false;
      if (videoEl) videoEl.playbackRate = playbackRate;
    } else if (gestureActive) {
      if (gestureType === 'brightness') {
        brightness = gestureValue;
        if (overlayEl) overlayEl.style.filter = `brightness(${brightness})`;
      } else if (gestureType === 'volume' && videoEl) {
        videoEl.volume = gestureValue;
      } else if (gestureType === 'seek' && videoEl) {
        videoEl.currentTime = Math.max(0, videoEl.currentTime + gestureValue);
      }
    }

    gestureActive = false;
    gestureType = 'none';
    gestureValue = 0;
    gestureLabel = '';
  }

  // 键盘快捷键
  function onKeyDown(e: KeyboardEvent) {
    // 输入框聚焦时不拦截
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return;

    if (e.key === 'f' || e.key === 'F') {
      e.preventDefault();
      toggleFullscreen();
      return;
    }

    if (useWebFallback) return;

    switch (e.key) {
      case ' ':
      case 'k':
        e.preventDefault();
        if (videoEl) videoEl.paused ? videoEl.play() : videoEl.pause();
        break;
      case 'ArrowLeft':
        e.preventDefault();
        if (videoEl) videoEl.currentTime = Math.max(0, videoEl.currentTime - 10);
        break;
      case 'ArrowRight':
        e.preventDefault();
        if (videoEl) videoEl.currentTime = Math.min(videoEl.duration || 0, videoEl.currentTime + 10);
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (videoEl) videoEl.volume = Math.min(1, videoEl.volume + 0.1);
        break;
      case 'ArrowDown':
        e.preventDefault();
        if (videoEl) videoEl.volume = Math.max(0, videoEl.volume - 0.1);
        break;
      case 'd':
      case 'D':
        e.preventDefault();
        animeStore.danmakuEnabled = !animeStore.danmakuEnabled;
        break;
      case 'n':
      case 'N':
        e.preventDefault();
        goNext();
        break;
      case 'p':
      case 'P':
        e.preventDefault();
        goPrev();
        break;
    }
  }

  function toggleCommentsPanel() {
    if (!showCommentsPanel) showEpisodePanel = false; // 两个右侧面板互斥
    showCommentsPanel = !showCommentsPanel;
  }

  async function handleDownload() {
    if (downloading) return;
    const src = videoSrc;
    if (!src || status !== 'found') {
      downloadMsg = '请先等待视频解析完成';
      setTimeout(() => downloadMsg = '', 3000);
      return;
    }
    downloading = true;
    downloadMsg = '';
    try {
      const ext = isM3u8 ? '.ts' : '.mp4';
      const safeName = (epName || 'episode').replace(/[<>:"/\\|?*]/g, '_');
      const filename = `${safeName}${ext}`;
      const referer = animeStore.rules.find(r => r.name === animeStore.playerRuleName)?.baseUrl || undefined;
      const animeName = animeStore.detailName || undefined;
      await animeDownloadEpisode(src, filename, undefined, animeName, epName || undefined, referer);
      downloadMsg = '已添加到下载队列';
      setTimeout(() => downloadMsg = '', 3000);
    } catch (e) {
      downloadMsg = `下载失败: ${e}`;
      setTimeout(() => downloadMsg = '', 5000);
    } finally {
      downloading = false;
    }
  }

  function formatCommentDate(dateStr: string): string {
    if (!dateStr) return '';
    try {
      const d = new Date(dateStr);
      if (isNaN(d.getTime())) return dateStr;
      const now = new Date();
      const diff = now.getTime() - d.getTime();
      const mins = Math.floor(diff / 60000);
      if (mins < 1) return '刚刚';
      if (mins < 60) return `${mins}分钟前`;
      const hours = Math.floor(mins / 60);
      if (hours < 24) return `${hours}小时前`;
      const days = Math.floor(hours / 24);
      if (days < 30) return `${days}天前`;
      return dateStr;
    } catch {
      return dateStr;
    }
  }


</script>

<div
  class="player-overlay"
  class:fullscreen={isFullscreen}
  role="dialog"
  aria-modal="true"
  aria-labelledby="anime-player-title"
  aria-describedby="anime-player-status"
  tabindex="-1"
  bind:this={overlayEl}
  use:focusTrap={{
    initialFocus: '[data-player-close]',
    returnFocus: false,
    closeOnEscape: true,
    onEscape: handlePlayerEscape,
  }}
>
  <h2 class="sr-only" id="anime-player-title">{animeStore.detailName} · {epName || "播放器"}</h2>
  <p class="sr-only" id="anime-player-status" aria-live="polite">{playerStatusTitle}。{playerStatusDescription}</p>

  <AnimePlaybackShell
    title={animeStore.detailName}
    episodeTitle={epName || "未知集数"}
    artworkUrl={animeStore.detailImage}
    sourceLabel={playbackSourceLabel}
    {episodePosition}
    {nextEpisodeTitle}
    aspectRatio={mediaAspectRatio}
    fullscreen={isFullscreen}
    panelOpen={showCommentsPanel || showEpisodePanel}
    variant="classic"
    stageLabel={`${animeStore.detailName} ${epName || "播放器"} 播放区域`}
  >
    {#snippet toolbar()}
  <div class="player-toolbar" role="toolbar" aria-label="播放器工具栏" tabindex="-1" onclick={handleToolbarClickOutside} onkeydown={(e) => { if (e.key === "Escape") { showSpeedMenu = false; showDanmakuSettings = false; showEpisodePanel = false; showCommentsPanel = false; } }}>
    <button class="tool-btn" type="button" data-player-close aria-label="关闭播放器并返回当前剧集" onclick={() => void closePlayer()}>
      <Icon name="x" size={16} /> 关闭
    </button>
    <div class="toolbar-sep"></div>
    <div class="ep-nav">
      <button class="nav-btn" type="button" aria-label="播放上一集" onclick={goPrev} disabled={!hasPrev}>
        <Icon name="chevronLeft" size={15} /> 上一集
      </button>
      <button class="nav-btn" type="button" aria-label="播放下一集" onclick={goNext} disabled={!hasNext}>
        下一集 <Icon name="chevronRight" size={15} />
      </button>
      <button class="nav-btn fullscreen-toggle" type="button" onclick={toggleFullscreen} aria-label={isFullscreen ? '退出全屏' : '进入全屏'} title={isFullscreen ? '退出全屏' : '全屏'}>
        <Icon name={isFullscreen ? 'x' : 'maximize'} size={15} />
      </button>
      {#if status === "found"}
        {#if isPipSupported}
          <button
            class="nav-btn pip-toggle"
          aria-label={isPipActive ? '退出画中画' : '开启画中画'}
            class:active={isPipActive}
            onclick={togglePip}
            title={isPipActive ? '退出画中画' : '画中画'}
          >
            <Icon name="pictureInPicture" size={15} />
            <span class="pip-label">PIP</span>
          </button>
        {/if}
        <button
          class="nav-btn danmaku-toggle"
          aria-label={danmakuEnabled ? '关闭弹幕' : '开启弹幕'}
          class:active={danmakuEnabled}
          onclick={() => { animeStore.danmakuEnabled = !animeStore.danmakuEnabled; }}
          title={danmakuEnabled ? '关闭弹幕' : '开启弹幕'}
        >
          <span class="danmaku-icon">弹</span>
          {#if danmakuLoading}
            <span class="danmaku-count">…</span>
          {:else if danmakuComments.length > 0}
            <span class="danmaku-count">{danmakuComments.length}</span>
          {/if}
        </button>
        <button
          class="nav-btn danmaku-settings-btn"
          aria-label={'打开弹幕设置'}
          class:active={showDanmakuSettings}
          onclick={() => { showDanmakuSettings = !showDanmakuSettings; showSpeedMenu = false; }}
          title="弹幕设置"
        >
          <Icon name="settings" size={13} />
        </button>
      {/if}
      {#if road && road.episodes.length > 1}
        <div class="toolbar-sep"></div>
        <button
          class="nav-btn episodes-toggle"
          aria-label={showEpisodePanel ? '关闭选集面板' : '打开选集面板'}
          class:active={showEpisodePanel}
          onclick={toggleEpisodePanel}
          title={showEpisodePanel ? '关闭选集' : '选集'}
        >
          <Icon name="list" size={14} /> 选集
        </button>
      {/if}
      {#if status === "found"}
        <!-- 倍速控制 -->
        <div class="speed-control">
          <button
            class="nav-btn speed-btn"
          aria-label={'选择播放速度'}
            class:active={showSpeedMenu}
            onclick={() => showSpeedMenu = !showSpeedMenu}
            title="倍速播放"
          >
            {playbackRate}x
          </button>
          {#if showSpeedMenu}
            <div class="speed-menu">
              {#each speedOptions as speed}
                <button
                  class="speed-option"
                  class:current={playbackRate === speed}
                  onclick={() => setPlaybackRate(speed)}
                >
                  {speed}x
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <!-- 弹幕设置面板 -->
        {#if showDanmakuSettings}
          <div class="danmaku-settings-panel">
            <div class="settings-section">
              <span class="settings-label">显示区域</span>
              <div class="settings-row">
                {#each [{l:'1/4',v:0},{l:'1/2',v:1},{l:'全屏',v:2}] as opt}
                  <button class="settings-chip" class:active={animeStore.danmakuArea===opt.v} onclick={()=>animeStore.danmakuArea=opt.v}>{opt.l}</button>
                {/each}
              </div>
            </div>
            <div class="settings-section">
              <span class="settings-label">不透明度 {Math.round(animeStore.danmakuOpacity*100)}%</span>
              <input type="range" min="0.1" max="1" step="0.05" value={animeStore.danmakuOpacity} oninput={e=>animeStore.danmakuOpacity=parseFloat((e.target as HTMLInputElement).value)} />
            </div>
            <div class="settings-section">
              <span class="settings-label">字号 {animeStore.danmakuFontSize}px</span>
              <input type="range" min="16" max="40" step="1" value={animeStore.danmakuFontSize} oninput={e=>animeStore.danmakuFontSize=parseInt((e.target as HTMLInputElement).value)} />
            </div>
            <div class="settings-section">
              <span class="settings-label">速度 {animeStore.danmakuSpeed}x</span>
              <input type="range" min="0.5" max="2" step="0.1" value={animeStore.danmakuSpeed} oninput={e=>animeStore.danmakuSpeed=parseFloat((e.target as HTMLInputElement).value)} />
            </div>
            <div class="settings-section">
              <span class="settings-label">屏蔽</span>
              <div class="settings-row">
                <label class="settings-check"><input type="checkbox" checked={animeStore.danmakuBlockScroll} onchange={()=>animeStore.danmakuBlockScroll=!animeStore.danmakuBlockScroll} /> 滚动</label>
                <label class="settings-check"><input type="checkbox" checked={animeStore.danmakuBlockTop} onchange={()=>animeStore.danmakuBlockTop=!animeStore.danmakuBlockTop} /> 顶部</label>
                <label class="settings-check"><input type="checkbox" checked={animeStore.danmakuBlockBottom} onchange={()=>animeStore.danmakuBlockBottom=!animeStore.danmakuBlockBottom} /> 底部</label>
              </div>
            </div>
          </div>
        {/if}
        <button
          class="nav-btn comments-toggle"
          aria-label={showCommentsPanel ? '关闭章节评论' : '打开章节评论'}
          class:active={showCommentsPanel}
          onclick={toggleCommentsPanel}
          title={showCommentsPanel ? '关闭评论' : '章节评论'}
        >
          <Icon name="messageCircle" size={14} /> 评论
        </button>
      {/if}
      {#if status === "found" && videoSrc}
        <button
          class="nav-btn download-btn"
          aria-label={downloading ? '正在添加下载任务' : '下载本集'}
          class:downloading
          onclick={handleDownload}
          disabled={downloading}
          title={downloading ? '正在添加...' : '下载本集'}
        >
          <Icon name="download" size={14} />
          {#if downloading}
            <span class="dl-label">...</span>
          {:else}
            <span class="dl-label">下载</span>
          {/if}
        </button>
      {/if}
      {#if downloadMsg}
        <span class="download-toast" class:error={downloadMsg.includes('失败') || downloadMsg.includes('请先')}>{downloadMsg}</span>
      {/if}
    </div>
  </div>
    {/snippet}

    {#snippet media()}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="player-body"
      class:with-panel={showCommentsPanel || showEpisodePanel}
      role="region"
      aria-label="播放器"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      onpointerleave={onPointerUp}
    >
      {#if useWebFallback && pageUrl}
        <div class="web-player-shell" class:loaded={webFrameLoaded}>
          {#if !webFrameLoaded && !webFrameTimedOut}
            <div class="web-player-loading" aria-live="polite">
              <div class="spinner"></div>
              <span>正在打开源站播放器…</span>
            </div>
          {/if}
          {#if webFrameTimedOut}
            <div class="web-fallback-panel" aria-live="polite">
              <Icon name="externalLink" size={28} />
              <span>{failureKind === 'iframeBlocked' ? '源站禁止嵌入播放' : '源站网页加载超时'}</span>
              <small>可以刷新网页、换源，或在外部浏览器打开源站页面。</small>
              <div class="state-actions">
                <button class="state-btn primary" onclick={retry}>重试原生提取</button>
                <button class="state-btn" onclick={refreshWebFrame}>刷新网页</button>
                <button class="state-btn" onclick={switchSource}>换源</button>
                <button class="state-btn" onclick={openInBrowser}>外部浏览器打开</button>
                <button class="state-btn" onclick={launchExternalPlayer}>外部播放器播放</button>
              </div>
            </div>
          {/if}
          {#key `${pageUrl}:${webFrameKey}`}
            <iframe
              src={pageUrl}
              title={epName}
              class="player-iframe"
              allow="autoplay; encrypted-media; picture-in-picture; clipboard-write"
              sandbox="allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox allow-forms allow-presentation"
              onload={() => { webFrameLoaded = true; webFrameTimedOut = false; }}
            ></iframe>
          {/key}
        </div>
      {:else if status === "extracting"}
        <div class="player-state player-state--async">
          <AsyncState
            state={failoverStatus === 'trying' ? 'refreshing' : 'loading'}
            title={playerStatusTitle}
            description={playerStatusDescription}
            loadingDelayMs={0}
            details={failoverStatus === 'trying' ? `来源 ${failoverCurrent} / ${failoverTotal}` : `解析 ${extractElapsed}s / 30s`}
          />
          <div class="failover-progress" role="progressbar" aria-label={playerStatusTitle} aria-valuemin="0" aria-valuemax="100" aria-valuenow={failoverStatus === 'trying' && failoverTotal > 0 ? Math.round(failoverCurrent / failoverTotal * 100) : Math.min(Math.round(extractElapsed / 30 * 100), 100)}>
            <div class="failover-bar" style="width: {failoverStatus === 'trying' && failoverTotal > 0 ? (failoverCurrent / failoverTotal * 100) : Math.min(extractElapsed / 30 * 100, 100)}%"></div>
          </div>
          <div class="state-actions">
            <button class="state-btn primary" type="button" onclick={() => animeStore.cancelExtract()}>取消加载</button>
            {#if failoverStatus === 'trying'}
              <button class="state-btn" type="button" onclick={() => animeStore.cancelFailover()}>仅取消换源</button>
            {/if}
            <button class="state-btn" type="button" onclick={() => void closePlayer()}>返回剧集</button>
            {#if pageUrl}
              <button class="state-btn" type="button" onclick={() => switchToWebFallback(true)}>用网页播放</button>
            {/if}
          </div>
        </div>
      {:else if status === "found" && videoSrc}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          bind:this={videoEl}
          class="player-video"
          controls
          controlslist="nofullscreen noremoteplayback"
          disablepictureinpicture={false}
          autoplay
          ontimeupdate={handleTimeUpdate}
        ></video>
        <DanmakuOverlay
          comments={danmakuComments}
          currentTime={currentTime}
          enabled={danmakuEnabled}
        />
        <!-- 手势提示浮层 -->
        {#if gestureActive && gestureLabel}
          <div class="gesture-hint">
            <span class="gesture-label">{gestureLabel}</span>
          </div>
        {/if}
        <!-- 长按倍速提示 -->
        {#if isLongPressing}
          <div class="long-press-hint">
            <span>{animeStore.longPressRate}x ▶▶</span>
          </div>
        {/if}
      {:else}
        <div class="player-state player-state--async">
          <AsyncState
            state="error"
            title={playerStatusTitle}
            description={playerStatusDescription}
            details={failureKind || undefined}
            primaryAction={{ label: '重试解析', onSelect: retry }}
            secondaryAction={{ label: '手动选源', onSelect: () => void switchSource() }}
          />
          {#if pageUrl}
            <div class="state-actions">
              <button class="state-btn" type="button" onclick={() => switchToWebFallback()}>用网页播放</button>
              <button class="state-btn" type="button" onclick={openInBrowser}>浏览器打开</button>
              <button class="state-btn" type="button" onclick={launchExternalPlayer}>
                <Icon name="externalLink" size={13} /> 外部播放
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>
    {/snippet}

    {#snippet panel()}
    {#if showCommentsPanel}
      <section class="comments-panel" aria-labelledby="anime-comments-title">
        <div class="comments-header">
          <h3 class="comments-title" id="anime-comments-title">章节评论</h3>
          <button class="comments-close" type="button" aria-label="关闭章节评论" onclick={() => showCommentsPanel = false}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div class="comments-body">
          {#if episodeCommentsLoading}
            <div class="comments-loading"><div class="spinner-sm"></div> 加载中...</div>
          {:else if episodeComments.length === 0}
            <div class="comments-empty">暂无评论</div>
          {:else}
            {#each episodeComments as comment, i (i)}
              <div class="comment-card">
                <div class="comment-header">
                  {#if comment.avatar}
                    <img class="comment-avatar" src={comment.avatar} alt="" />
                  {:else}
                    <div class="comment-avatar-placeholder">
                      {comment.user ? comment.user[0] : '?'}
                    </div>
                  {/if}
                  <div class="comment-user-info">
                    <span class="comment-username">{comment.user || '匿名'}</span>
                    <span class="comment-date">{formatCommentDate(comment.date)}</span>
                  </div>
                </div>
                <div class="comment-text">{comment.comment}</div>
              </div>
            {/each}
          {/if}
        </div>
      </section>
    {/if}

    {#if showEpisodePanel}
      <section class="episodes-panel" aria-labelledby="anime-episodes-title">
        <div class="comments-header">
          <h3 class="comments-title" id="anime-episodes-title">选集</h3>
          <button class="comments-close" type="button" aria-label="关闭选集面板" onclick={() => showEpisodePanel = false}>
            <Icon name="x" size={14} />
          </button>
        </div>
        {#if roads.length > 1}
          <div class="ep-road-tabs" role="tablist" aria-label="播放器线路">
            {#each roads as r, i}
              <button
                class="ep-road-tab"
                type="button"
                role="tab"
                class:active={pickerRoadIdx === i}
                aria-selected={pickerRoadIdx === i}
                tabindex={pickerRoadIdx === i ? 0 : -1}
                onclick={() => { pickerRoadIdx = i; }}
              >
                {r.name || `线路${i + 1}`}
              </button>
            {/each}
          </div>
        {/if}
        <div class="episodes-panel-body">
          <div class="ep-panel-grid">
            {#each pickerEpisodes as ep, i (ep.url + i)}
              <button
                class="ep-panel-btn"
                class:current={pickerRoadIdx === roadIdx && i === epIdx}
                onclick={() => pickEpisode(pickerRoadIdx, i)}
                title={ep.name}
              >
                {ep.name || `第${i + 1}集`}
              </button>
            {/each}
          </div>
        </div>
      </section>
    {/if}
    {/snippet}

    {#snippet footer()}
  {#if !isFullscreen}
    <div class="player-bottom">
      <button class="bottom-btn" type="button" aria-label="播放上一集" onclick={goPrev} disabled={!hasPrev}>
        <Icon name="chevronLeft" size={16} /> 上一集
      </button>
      <button class="bottom-btn close" type="button" onclick={() => void closePlayer()}>返回当前剧集</button>
      <button class="bottom-btn" type="button" aria-label="播放下一集" onclick={goNext} disabled={!hasNext}>
        下一集 <Icon name="chevronRight" size={16} />
      </button>
    </div>
  {/if}
    {/snippet}
  </AnimePlaybackShell>
</div>

<style>
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0; }
  .player-state--async { width: min(36rem, calc(100% - 2rem)); }
  .player-state--async :global(.v2-async-state) { width: 100%; }

  .player-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, #07110d 0%, #020503 100%);
    z-index: 30;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .player-overlay.fullscreen {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: #000;
  }
  .player-overlay.fullscreen .player-toolbar {
    background: linear-gradient(180deg, rgba(0,0,0,0.72), rgba(0,0,0,0.2));
    border-bottom: 0;
  }
  .player-overlay.fullscreen .player-body {
    width: 100%;
    height: 100%;
  }
  .player-overlay.fullscreen .player-video,
  .player-overlay.fullscreen .player-iframe {
    height: 100%;
  }
  .fullscreen-toggle {
    border-color: rgba(255,255,255,0.2) !important;
  }
  .pip-toggle {
    gap: 4px;
    font-size: 12px;
  }
  .pip-toggle.active {
    border-color: var(--accent-ring, rgba(232,85,127,0.4));
    background: var(--accent-lo, rgba(232,85,127,0.12));
    color: var(--accent);
  }
  .pip-label {
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-mono);
  }
  .danmaku-toggle, .comments-toggle, .download-btn {
    gap: 4px;
    font-size: 12px;
  }
  .danmaku-toggle.active, .comments-toggle.active {
    border-color: var(--accent-ring, rgba(232,85,127,0.4));
    background: var(--accent-lo, rgba(232,85,127,0.12));
    color: var(--accent);
  }
  .download-btn {
    border-color: rgba(74,222,128,0.3);
    color: rgba(74,222,128,0.9);
  }
  .download-btn:hover:not(:disabled) {
    border-color: rgba(74,222,128,0.6);
    background: rgba(74,222,128,0.1);
    color: #4ade80;
  }
  .download-btn.downloading {
    opacity: 0.5;
    cursor: wait;
  }
  .dl-label {
    font-size: 11px;
    font-weight: 600;
  }
  .download-toast {
    font-size: 11px;
    color: #4ade80;
    white-space: nowrap;
    animation: fade-in 0.2s ease;
  }
  .download-toast.error {
    color: #f87171;
  }
  @keyframes fade-in {
    from { opacity: 0; transform: translateY(-2px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .danmaku-icon {
    font-weight: 700;
    font-size: 13px;
  }
  .danmaku-count {
    font-size: 10px;
    opacity: 0.7;
    font-family: var(--font-mono);
  }
  .danmaku-settings-btn {
    padding: 6px 8px;
  }
  .danmaku-settings-btn.active {
    border-color: var(--accent-ring, rgba(232,85,127,0.4));
    background: var(--accent-lo, rgba(232,85,127,0.12));
    color: var(--accent);
  }

  /* 弹幕设置面板 */
  .danmaku-settings-panel {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    width: 220px;
    background: rgba(20, 22, 28, 0.95);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 10px;
    padding: 10px;
    z-index: 100;
    backdrop-filter: blur(12px);
    animation: fade-in 0.15s ease;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .settings-label {
    font-size: 11px;
    color: var(--text-muted);
    font-weight: 500;
  }
  .settings-row {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .settings-chip {
    padding: 3px 10px;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 12px;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .settings-chip:hover {
    border-color: var(--accent);
    color: var(--text-primary);
  }
  .settings-chip.active {
    background: var(--accent-lo, rgba(232,85,127,0.15));
    border-color: var(--accent);
    color: var(--accent);
  }
  .settings-check {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
  }
  .settings-check input[type="checkbox"] {
    width: 14px;
    height: 14px;
    accent-color: var(--accent);
  }
  .danmaku-settings-panel input[type="range"] {
    width: 100%;
    height: 4px;
    -webkit-appearance: none;
    appearance: none;
    background: rgba(255,255,255,0.1);
    border-radius: 2px;
    outline: none;
  }
  .danmaku-settings-panel input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
  }

  .player-toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    background: rgba(10, 12, 18, 0.9);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    backdrop-filter: blur(8px);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .player-toolbar::-webkit-scrollbar { display: none; }
  .toolbar-sep {
    width: 1px;
    height: 20px;
    background: rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
  }
  .tool-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 6px 12px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-muted);
    font-size: 12.5px; cursor: pointer; transition: all 0.15s; flex-shrink: 0;
  }
  .tool-btn:hover { background: rgba(255, 255, 255, 0.1); color: var(--text-primary); }
  .ep-nav { display: flex; gap: 6px; flex-shrink: 0; }
  .nav-btn {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 6px 12px; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 6px;
    background: transparent; color: var(--text-muted); font-size: 12px; cursor: pointer; transition: all 0.15s;
  }
  .nav-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .nav-btn:not(:disabled):hover { border-color: var(--accent); color: var(--accent); }

  .player-body {
    flex: 1; min-height: 0;
    display: flex; align-items: center; justify-content: center;
    background: linear-gradient(180deg, #020503 0%, #000 100%);
    position: relative;
    transition: flex 0.2s;
  }
  .player-body.with-panel {
    flex: 1;
  }
  .player-video {
    width: 100%; height: 100%; border: none; outline: none; background: #000;
  }
  /* WebView2 native/source fullscreen can corrupt the Tauri host window style.
     MoePlay owns fullscreen through the toolbar, so inner media controls must not request DOM fullscreen. */
  .player-video::-webkit-media-controls-fullscreen-button { display: none !important; }
  .web-player-shell {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: linear-gradient(180deg, #031008 0%, #000 100%);
  }
  .web-player-shell::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(34, 197, 94, 0.08), transparent 36%);
    pointer-events: none;
    z-index: 0;
  }
  .web-player-shell.loaded::before {
    opacity: 0;
  }
  .player-iframe {
    position: relative;
    z-index: 1;
    width: 100%; height: 100%; border: none; outline: none; background: #000;
  }
  .web-fallback-panel {
    position: absolute;
    inset: 0;
    z-index: 3;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 24px;
    background: linear-gradient(180deg, rgba(2, 9, 6, 0.9), rgba(0, 0, 0, 0.94));
    color: var(--text-muted);
    text-align: center;
  }
  .web-fallback-panel span {
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 650;
  }
  .web-fallback-panel small {
    max-width: 420px;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .web-player-loading {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 12px;
    background: linear-gradient(180deg, rgba(2, 5, 3, 0.78), rgba(0, 0, 0, 0.88));
    color: var(--text-secondary);
    font-size: 13px;
    pointer-events: none;
  }
  .player-state {
    display: flex; flex-direction: column; align-items: center; gap: 12px;
    color: var(--text-muted); text-align: center; padding: 24px;
  }
  .state-actions { display: flex; gap: 8px; margin-top: 6px; flex-wrap: wrap; justify-content: center; }
  .state-btn {
    padding: 8px 16px; border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 8px;
    background: transparent; color: var(--text-secondary); font-size: 13px; cursor: pointer; transition: all 0.15s;
  }
  .state-btn:hover { border-color: var(--text-muted); color: var(--text-primary); }
  .state-btn.primary {
    border-color: var(--accent-ring, rgba(232,85,127,0.4));
    background: var(--accent-lo, rgba(232,85,127,0.12));
    color: var(--accent);
  }
  .state-btn.primary:hover { background: var(--accent); color: #fff; }

  /* Comments Panel */
  .comments-panel {
    width: 320px; flex-shrink: 0;
    display: flex; flex-direction: column;
    background: #111318;
    border-left: 1px solid rgba(255,255,255,0.06);
    animation: slide-in-right 0.2s ease;
  }
  .comments-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid rgba(255,255,255,0.06);
  }
  .comments-title {
    font-size: 13px; font-weight: 600; color: var(--text-primary);
  }
  .comments-close {
    background: none; border: none; color: var(--text-muted);
    cursor: pointer; padding: 4px; border-radius: 4px;
    transition: color 0.15s;
  }
  .comments-close:hover { color: var(--text-primary); }
  .comments-body {
    flex: 1; overflow-y: auto; padding: 8px 12px;
  }
  .comments-loading, .comments-empty {
    display: flex; align-items: center; justify-content: center; gap: 8px;
    padding: 32px 0; color: var(--text-muted); font-size: 13px;
  }
  .comment-card {
    padding: 10px 0;
    border-bottom: 1px solid rgba(255,255,255,0.04);
  }
  .comment-card:last-child { border-bottom: none; }
  .comment-header {
    display: flex; align-items: center; gap: 8px; margin-bottom: 6px;
  }
  .comment-avatar {
    width: 28px; height: 28px; border-radius: 50%; object-fit: cover;
    flex-shrink: 0;
  }
  .comment-avatar-placeholder {
    width: 28px; height: 28px; border-radius: 50%;
    background: rgba(232,85,127,0.15); color: var(--accent);
    display: flex; align-items: center; justify-content: center;
    font-size: 12px; font-weight: 600; flex-shrink: 0;
  }
  .comment-user-info { display: flex; flex-direction: column; min-width: 0; }
  .comment-username {
    font-size: 12px; font-weight: 600; color: var(--text-primary);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .comment-date {
    font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);
  }
  .comment-text {
    font-size: 12.5px; color: var(--text-secondary); line-height: 1.6;
    word-break: break-word;
  }

  /* Episodes Panel */
  .episodes-panel {
    width: 320px; flex-shrink: 0;
    display: flex; flex-direction: column;
    background: #111318;
    border-left: 1px solid rgba(255,255,255,0.06);
    animation: slide-in-right 0.2s ease;
  }
  .ep-road-tabs {
    display: flex; gap: 6px; flex-wrap: wrap;
    padding: 10px 14px 4px;
    flex-shrink: 0;
  }
  .ep-road-tab {
    padding: 4px 12px; border: 1px solid rgba(255,255,255,0.12);
    border-radius: 14px; background: transparent;
    color: var(--text-muted); font-size: 12px; font-weight: 500;
    cursor: pointer; transition: all 0.15s;
  }
  .ep-road-tab:hover { border-color: var(--accent-ring, rgba(232,85,127,0.4)); color: var(--text-primary); }
  .ep-road-tab.active {
    background: var(--accent-lo, rgba(232,85,127,0.15));
    border-color: var(--accent); color: var(--accent);
  }
  .episodes-panel-body {
    flex: 1; overflow-y: auto; padding: 10px 14px 14px;
  }
  .ep-panel-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
    gap: 8px;
  }
  .ep-panel-btn {
    padding: 10px 6px; border: 1px solid rgba(255,255,255,0.08);
    border-radius: 8px; background: rgba(255,255,255,0.03);
    color: var(--text-muted); font-size: 12.5px; font-weight: 500;
    cursor: pointer; transition: all 0.15s; text-align: center;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .ep-panel-btn:hover {
    border-color: var(--accent-ring, rgba(232,85,127,0.5));
    background: var(--accent-lo, rgba(232,85,127,0.1));
    color: var(--text-primary);
  }
  .ep-panel-btn.current {
    border-color: var(--accent); background: var(--accent-lo, rgba(232,85,127,0.15));
    color: var(--accent); font-weight: 700;
  }
  .episodes-toggle { gap: 4px; font-size: 12px; }
  .episodes-toggle.active {
    border-color: var(--accent-ring, rgba(232,85,127,0.4));
    background: var(--accent-lo, rgba(232,85,127,0.12));
    color: var(--accent);
  }

  /* 倍速控制 */
  .speed-control {
    position: relative;
  }
  .speed-btn {
    font-family: var(--font-mono);
    font-size: 11px;
    min-width: 45px;
    justify-content: center;
  }
  .speed-btn.active {
    border-color: var(--accent-ring, rgba(232,85,127,0.4));
    background: var(--accent-lo, rgba(232,85,127,0.12));
    color: var(--accent);
  }
  .speed-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: rgba(20, 22, 28, 0.95);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 8px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    z-index: 100;
    backdrop-filter: blur(12px);
    animation: fade-in 0.15s ease;
  }
  .speed-option {
    padding: 6px 16px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-family: var(--font-mono);
    cursor: pointer;
    transition: all 0.15s;
    text-align: center;
  }
  .speed-option:hover {
    background: rgba(255,255,255,0.08);
    color: var(--text-primary);
  }
  .speed-option.current {
    background: var(--accent-lo, rgba(232,85,127,0.15));
    color: var(--accent);
    font-weight: 600;
  }

  /* 手势提示 */
  .gesture-hint, .long-press-hint {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: rgba(0,0,0,0.7);
    color: #fff;
    padding: 12px 20px;
    border-radius: 12px;
    font-size: 14px;
    font-weight: 600;
    pointer-events: none;
    animation: fade-in 0.1s ease;
    z-index: 50;
  }
  .long-press-hint {
    background: var(--accent-lo, rgba(232,85,127,0.8));
    font-size: 16px;
  }

  @keyframes slide-in-right {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }

  .player-bottom {
    flex-shrink: 0;
    display: flex; justify-content: center; gap: 12px;
    padding: 12px 16px;
    background: rgba(10, 12, 18, 0.9);
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }
  .bottom-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 8px 18px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255, 255, 255, 0.04); color: var(--text-muted); font-size: 13px; cursor: pointer; transition: all 0.15s;
  }
  .bottom-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .bottom-btn.close { border-color: var(--accent-ring, rgba(232,85,127,0.3)); color: var(--accent); }
  .bottom-btn:not(:disabled):hover {
    border-color: var(--accent); background: var(--accent-lo, rgba(232,85,127,0.1)); color: var(--accent);
  }
  @keyframes extract-ring {
    0% { transform: scale(0.8); opacity: 0.6; }
    100% { transform: scale(1.4); opacity: 0; }
  }
  .spinner {
    width: 36px; height: 36px; border: 3px solid rgba(255, 255, 255, 0.08);
    border-top-color: var(--accent); border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  .spinner-sm {
    width: 16px; height: 16px; border: 2px solid rgba(255,255,255,0.1);
    border-top-color: var(--accent); border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* 换源自愈进度条 */
  .failover-progress {
    width: min(240px, 70vw);
    height: 4px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
    overflow: hidden;
    margin-top: 4px;
  }
  .failover-bar {
    height: 100%;
    background: var(--accent, #e8557f);
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  @media (prefers-reduced-motion: reduce) {
    .player-overlay, .player-overlay *, .spinner, .spinner-sm { animation: none !important; transition: none !important; }
  }
  :global([data-motion="reduce"]) .player-overlay,
  :global([data-motion="reduce"]) .player-overlay * { animation: none !important; transition: none !important; }
</style>
