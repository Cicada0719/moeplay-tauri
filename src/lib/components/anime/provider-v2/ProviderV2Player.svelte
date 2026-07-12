<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import Hls from "hls.js";
  import { onMount } from "svelte";
  import type { AnimeEpisode, AnimeResolveResponse } from "../../../features/anime";
  import Icon from "../../Icon.svelte";
  import { AsyncState } from "../../ui-v2";
  import { focusTrap } from "../../../actions/a11y/focusTrap";
  import { AnimePlaybackShell } from "../playback";

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
  let mediaAspectRatio = $state(16 / 9);
  let hls: Hls | null = null;

  const target = $derived(resolution.target);
  const canPlayInternally = $derived(target.mode === "native_hls" || target.mode === "native_file");
  const sourceLabel = $derived(`Provider v2 · ${episode.identity.providerId}`);
  const episodePosition = $derived(episode.number === null ? "" : `EP ${String(episode.number).padStart(2, "0")}`);
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

  function updateMediaRatio() {
    if (!videoElement?.videoWidth || !videoElement.videoHeight) return;
    mediaAspectRatio = videoElement.videoWidth / videoElement.videoHeight;
  }

  async function attachPlayback() {
    destroyPlayback();
    playbackError = "";
    mediaAspectRatio = 16 / 9;
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
  <span class="sr-only" id="provider-player-title">{seriesTitle}</span>
  <span class="sr-only" id="provider-player-description">{episode.title}</span>

  <AnimePlaybackShell
    title={seriesTitle}
    episodeTitle={episode.title}
    artworkUrl={episode.artworkUrl}
    {sourceLabel}
    {episodePosition}
    aspectRatio={mediaAspectRatio}
    variant="provider"
    stageLabel={`${seriesTitle} ${episode.title} 播放区域`}
  >
    {#snippet headerActions()}
      <button class="icon-button" data-provider-player-close type="button" aria-label="关闭播放器并返回剧集" onclick={onClose}>
        <Icon name="x" size={18} />
      </button>
    {/snippet}

    {#snippet media()}
      <div class="player-stage" class:handoff={!canPlayInternally}>
        {#if canPlayInternally}
          <video
            bind:this={videoElement}
            controls
            autoplay
            playsinline
            preload="metadata"
            onloadedmetadata={updateMediaRatio}
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
    {/snippet}
  </AnimePlaybackShell>
</div>

<style>
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0; }
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
    width: 100%;
    height: 100%;
    min-height: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    background: #000;
  }
  .player-stage.handoff { padding: clamp(20px, 5vw, 48px); }
  video { width: 100%; height: 100%; object-fit: contain; background: transparent; }
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
    padding: 7px 10px;
    border: 0;
    border-radius: 9px;
    background: #58ad98;
    color: #06110e;
    font: inherit;
    font-size: 11px;
    font-weight: 750;
    cursor: pointer;
  }
  button:disabled { opacity: 0.55; cursor: wait; }
  .handoff-card {
    width: min(620px, 100%);
    display: grid;
    gap: 18px;
    padding: 28px;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 16px;
    background: rgba(255,255,255,0.035);
  }
  @media (max-width: 700px) {
    .player-backdrop { padding: 0; }
    .handoff-card { padding: 20px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .player-backdrop, .player-backdrop * { animation: none !important; transition: none !important; }
  }
  :global([data-motion="reduce"]) .player-backdrop,
  :global([data-motion="reduce"]) .player-backdrop * { animation: none !important; transition: none !important; }
</style>
