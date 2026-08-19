<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { MINI_CLOSED_EVENT_KEY, MINI_SESSION_KEY, readMiniSession, type MiniSession } from "../../features/mini/session";
  import Icon from "../Icon.svelte";

  let session = $state<MiniSession | null>(null);
  let videoEl = $state<HTMLVideoElement | null>(null);
  let error = $state("");
  let hls: { destroy(): void } | null = null;

  function onStorage(event: StorageEvent) {
    if (event.key === MINI_SESSION_KEY) loadSession();
  }

  function loadSession() {
    session = readMiniSession();
    if (!session) {
      error = "等待播放会话…";
      return;
    }
    error = "";
    void applySource();
  }

  async function applySource() {
    if (!session || !videoEl) return;
    const video = videoEl;
    if (hls) {
      try { hls.destroy(); } catch { /* ignore */ }
      hls = null;
    }
    if (session.isM3u8) {
      const { default: Hls } = await import("hls.js");
      if (Hls.isSupported()) {
        const instance = new Hls({ enableWorker: true });
        hls = instance;
        instance.loadSource(session.url);
        instance.attachMedia(video);
        instance.on(Hls.Events.MANIFEST_PARSED, () => {
          if (session?.time && session.time > 0) {
            try { video.currentTime = session.time; } catch { /* ignore */ }
          }
          void video.play().catch(() => {});
        });
        return;
      }
    }
    video.src = session.url;
    if (session.time && session.time > 0) {
      try { video.currentTime = session.time; } catch { /* ignore */ }
    }
    void video.play().catch(() => {});
  }

  async function closeMini() {
    try { localStorage.setItem(MINI_CLOSED_EVENT_KEY, String(Date.now())); } catch { /* ignore */ }
    try { await getCurrentWindow().close(); } catch { /* ignore */ }
  }

  $effect(() => {
    if (typeof window === "undefined") return;
    window.addEventListener("storage", onStorage);
    loadSession();
    return () => window.removeEventListener("storage", onStorage);
  });
</script>

<div class="mini-root">
  <header class="mini-bar" data-tauri-drag-region>
    <span class="mini-title" data-tauri-drag-region>{session?.title ?? "迷你播放"}</span>
    <button class="mini-close" type="button" aria-label="关闭迷你播放器" onclick={() => void closeMini()}><Icon name="x" size={14} /></button>
  </header>
  <div class="mini-stage">
    {#if session}
      <video
        bind:this={videoEl}
        class="mini-video"
        controls
        autoplay
        playsinline
        src={session.isM3u8 ? undefined : session.url}
      ><track kind="captions" /></video>
    {:else}
      <p class="mini-empty" role="status">{error || "没有可播放的会话"}</p>
    {/if}
  </div>
</div>

<style>
  .mini-root { height: 100%; display: grid; grid-template-rows: 2.2rem minmax(0, 1fr); background: #0a0d13; color: #fff; }
  .mini-bar { display: flex; align-items: center; justify-content: space-between; gap: .5rem; padding: .2rem .4rem .2rem .7rem; background: #10141d; border-bottom: 1px solid rgba(255,255,255,.08); cursor: move; user-select: none; }
  .mini-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: .78rem; font-weight: 700; }
  .mini-close { display: grid; width: 1.7rem; height: 1.7rem; place-items: center; border: 0; border-radius: 8px; background: transparent; color: rgba(255,255,255,.7); cursor: pointer; }
  .mini-close:hover { background: rgba(255,255,255,.1); color: #fff; }
  .mini-close:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .mini-stage { position: relative; min-height: 0; background: #000; }
  .mini-video { width: 100%; height: 100%; display: block; object-fit: contain; background: #000; }
  .mini-empty { position: absolute; inset: 0; display: grid; place-items: center; margin: 0; color: rgba(255,255,255,.55); font-size: .82rem; }
</style>
