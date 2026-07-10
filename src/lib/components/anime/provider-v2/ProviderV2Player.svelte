<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import Hls from "hls.js";
  import { onMount } from "svelte";
  import type { AnimeEpisode, AnimeResolveResponse } from "../../../features/anime";
  import Icon from "../../Icon.svelte";
  import { AsyncState } from "../../ui-v2";
  import { focusTrap } from "../../../actions/a11y/focusTrap";

  let {
    resolution,
    episode,
    seriesTitle,
    openingFallback = false,
    onClose,
    onFallback,
  }: {
    resolution: AnimeResolveResponse;
    episode: AnimeEpisode;
    seriesTitle: string;
    openingFallback?: boolean;
    onClose: () => void;
    onFallback: () => void | Promise<void>;
  } = $props();

  let videoElement = $state<HTMLVideoElement | null>(null);
  let playbackError = $state("");
  let hls: Hls | null = null;

  const target = $derived(resolution.target);
  const canPlayInternally = $derived(target.mode === "native_hls" || target.mode === "native_file");
  const statusTitle = $derived.by(() => {
    if (target.mode === "webview") return "需要安全网页窗口";
    if (target.mode === "external") return "需要交给外部浏览器";
    if (target.mode === "unsupported") return "当前播放方式不受支持";
    return "播放器准备中";
  });
  const statusMessage = $derived.by(() => {
    if (target.mode === "webview") return "该来源依赖网页环境。MoePlay 会重新向后端确认地址和允许域名后再打开独立窗口。";
    if (target.mode === "external") return target.reason || "该来源需要在系统浏览器中继续。";
    if (target.mode === "unsupported") return target.reason;
    return "正在准备媒体。";
  });

  function destroyPlayback() {
    hls?.destroy();
    hls = null;
    if (videoElement) {
      videoElement.pause();
      videoElement.removeAttribute("src");
      videoElement.load();
    }
  }

  async function attachPlayback() {
    destroyPlayback();
    playbackError = "";
    if (!videoElement) return;
    if (target.mode === "native_file") {
      videoElement.src = convertFileSrc(target.path);
      await videoElement.play().catch(() => undefined);
      return;
    }
    if (target.mode !== "native_hls") return;

    if (Hls.isSupported()) {
      hls = new Hls({
        enableWorker: true,
        lowLatencyMode: true,
        backBufferLength: 90,
      });
      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (data.fatal) playbackError = "视频流无法继续播放，可重试或改用来源提供的安全回退方式。";
      });
      hls.loadSource(target.url);
      hls.attachMedia(videoElement);
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        videoElement?.play().catch(() => undefined);
      });
    } else if (videoElement.canPlayType("application/vnd.apple.mpegurl")) {
      videoElement.src = target.url;
      await videoElement.play().catch(() => undefined);
    } else {
      playbackError = "当前系统 WebView 不支持 HLS 播放。";
    }
  }

  function handleVideoError() {
    playbackError = target.mode === "native_file"
      ? "该文件的封装或编码不受内置播放器支持，可尝试系统播放器。"
      : "视频加载失败，请检查来源状态后重试。";
  }

  onMount(() => {
    queueMicrotask(attachPlayback);
    return destroyPlayback;
  });
</script>

<div
  class="player-backdrop"
  role="dialog"
  aria-modal="true"
  aria-labelledby="provider-player-title"
  aria-describedby="provider-player-description"
  tabindex="-1"
  use:focusTrap={{ initialFocus: '[data-provider-player-close]', returnFocus: true, closeOnEscape: true, onEscape: onClose }}
>
  <div class="player-shell">
    <header class="player-header">
      <div class="player-heading">
        <span class="eyebrow">Provider v2</span>
        <h2 id="provider-player-title">{seriesTitle}</h2>
        <p id="provider-player-description">{episode.title}</p>
      </div>
      <button class="icon-button" data-provider-player-close type="button" aria-label="关闭播放器并返回剧集" onclick={onClose}>
        <Icon name="x" size={18} />
      </button>
    </header>

    <div class="player-stage" class:handoff={!canPlayInternally}>
      {#if canPlayInternally}
        <video
          bind:this={videoElement}
          controls
          autoplay
          playsinline
          preload="metadata"
          onerror={handleVideoError}
          aria-label={`${seriesTitle} ${episode.title}`}
        ></video>
        {#if playbackError}
          <div class="playback-notice" role="alert">
            <Icon name="info" size={18} />
            <span>{playbackError}</span>
            {#if target.mode === "native_file"}
              <button type="button" onclick={onFallback} disabled={openingFallback}>
                {openingFallback ? "正在打开" : "使用系统播放器"}
              </button>
            {/if}
          </div>
        {/if}
      {:else}
        <div class="handoff-card">
          <AsyncState
            state={target.mode === "unsupported" ? "error" : openingFallback ? "loading" : "partial"}
            title={statusTitle}
            description={statusMessage}
            loadingDelayMs={0}
            primaryAction={target.mode === "webview" || target.mode === "external" ? {
              label: openingFallback ? "正在确认" : target.mode === "webview" ? "打开安全窗口" : "在浏览器中打开",
              onSelect: () => void onFallback(),
              disabled: openingFallback,
              loading: openingFallback,
            } : undefined}
          />
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .player-backdrop {
    position: absolute;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 6, 12, 0.88);
    backdrop-filter: blur(16px);
  }
  .player-shell {
    width: min(1180px, 100%);
    max-height: calc(100% - 12px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 18px;
    background: #090b10;
    box-shadow: 0 28px 80px rgba(0,0,0,0.5);
  }
  .player-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 15px 18px;
    border-bottom: 1px solid rgba(255,255,255,0.08);
  }
  .player-heading { min-width: 0; }
  .eyebrow {
    display: block;
    margin-bottom: 4px;
    color: #77c7b3;
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 750;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  h2, p { margin: 0; }
  h2 { overflow: hidden; color: #f7f8fb; font-size: 17px; text-overflow: ellipsis; white-space: nowrap; }
  .player-heading p { margin-top: 3px; color: #9299a8; font-size: 12px; }
  .icon-button {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 9px;
    background: rgba(255,255,255,0.04);
    color: #cbd0da;
    cursor: pointer;
  }
  .icon-button:hover { background: rgba(255,255,255,0.09); color: white; }
  .player-stage {
    position: relative;
    min-height: 360px;
    display: grid;
    place-items: center;
    background: #030406;
    aspect-ratio: 16 / 9;
  }
  .player-stage.handoff { aspect-ratio: auto; min-height: 470px; padding: 48px; }
  video { width: 100%; height: 100%; object-fit: contain; background: black; }
  .playback-notice {
    position: absolute;
    left: 18px;
    right: 18px;
    bottom: 18px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 13px;
    border: 1px solid rgba(248,113,113,0.28);
    border-radius: 10px;
    background: rgba(23,9,12,0.92);
    color: #f5c2c7;
    font-size: 12px;
  }
  .playback-notice span { flex: 1; }
  .playback-notice button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 0;
    border-radius: 9px;
    background: #58ad98;
    color: #06110e;
    font: inherit;
    font-weight: 750;
    cursor: pointer;
  }
  .playback-notice button { padding: 7px 10px; font-size: 11px; }
  button:disabled { opacity: 0.55; cursor: wait; }
  .handoff-card {
    width: min(620px, 100%);
    display: grid;
    grid-template-columns: 1fr;
    gap: 18px;
    padding: 28px;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 16px;
    background: rgba(255,255,255,0.035);
  }
  @media (max-width: 700px) {
    .player-backdrop { padding: 10px; }
    .player-stage.handoff { min-height: 380px; padding: 20px; }
    .handoff-card { grid-template-columns: 1fr; }
  }

  @media (prefers-reduced-motion: reduce) {
    .player-backdrop, .player-backdrop * { animation: none !important; transition: none !important; }
  }
  :global([data-motion="reduce"]) .player-backdrop,
  :global([data-motion="reduce"]) .player-backdrop * { animation: none !important; transition: none !important; }
</style>
