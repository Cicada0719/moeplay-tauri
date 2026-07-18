<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import "../../../styles/anime-player.css";

  type PlaybackVariant = "classic" | "provider";

  interface Props {
    title: string;
    episodeTitle?: string;
    artworkUrl?: string | null;
    sourceLabel?: string;
    episodePosition?: string;
    nextEpisodeTitle?: string;
    aspectRatio?: number;
    fullscreen?: boolean;
    chromeVisible?: boolean;
    variant?: PlaybackVariant;
    panelOpen?: boolean;
    stageLabel?: string;
    headerActions?: Snippet;
    toolbar?: Snippet;
    media: Snippet;
    panel?: Snippet;
    footer?: Snippet;
  }

  let {
    title,
    episodeTitle = "",
    artworkUrl = null,
    sourceLabel = "",
    episodePosition = "",
    nextEpisodeTitle = "",
    aspectRatio = 16 / 9,
    fullscreen = false,
    chromeVisible = true,
    variant = "classic",
    panelOpen = false,
    stageLabel = "视频播放区域",
    headerActions,
    toolbar,
    media,
    panel,
    footer,
  }: Props = $props();

  let bodyHost = $state<HTMLDivElement | null>(null);
  let stageHost = $state<HTMLDivElement | null>(null);
  let bodyWidth = $state(0);
  let bodyHeight = $state(0);
  let frameWidth = $state(0);
  let frameHeight = $state(0);
  let resizeObserver: ResizeObserver | null = null;

  const safeRatio = $derived(Number.isFinite(aspectRatio) && aspectRatio > 0 ? aspectRatio : 16 / 9);
  const showInfoRail = $derived(Boolean(artworkUrl) && !fullscreen && !panelOpen && bodyWidth >= 1040 && bodyHeight > 0 && bodyWidth / bodyHeight > safeRatio + .24);
  const frameStyle = $derived(frameWidth > 0 && frameHeight > 0
    ? `width:${frameWidth}px;height:${frameHeight}px;aspect-ratio:${safeRatio};`
    : `width:100%;height:100%;aspect-ratio:${safeRatio};`);

  function measureFrame() {
    if (bodyHost) {
      const bodyRect = bodyHost.getBoundingClientRect();
      bodyWidth = Math.floor(bodyRect.width);
      bodyHeight = Math.floor(bodyRect.height);
    }
    if (!stageHost) return;
    const { width, height } = stageHost.getBoundingClientRect();
    if (width <= 0 || height <= 0) return;
    const hostRatio = width / height;
    if (hostRatio > safeRatio) {
      frameHeight = Math.floor(height);
      frameWidth = Math.floor(height * safeRatio);
    } else {
      frameWidth = Math.floor(width);
      frameHeight = Math.floor(width / safeRatio);
    }
  }

  $effect(() => {
    safeRatio;
    panelOpen;
    showInfoRail;
    if (stageHost) requestAnimationFrame(measureFrame);
  });

  onMount(() => {
    if (!stageHost) return;
    resizeObserver = new ResizeObserver(measureFrame);
    resizeObserver.observe(stageHost);
    if (bodyHost) resizeObserver.observe(bodyHost);
    measureFrame();
    return () => resizeObserver?.disconnect();
  });
</script>

<section
  class="anime-playback-shell anime-playback-shell--{variant}"
  class:anime-playback-shell--fullscreen={fullscreen}
  class:anime-playback-shell--chrome-hidden={fullscreen && !chromeVisible}
  class:anime-playback-shell--with-panel={panelOpen}
  data-testid="anime-playback-shell"
  data-playback-variant={variant}
>
  <div class="anime-playback-shell__ambient" aria-hidden="true">
    {#if artworkUrl}<img src={artworkUrl} alt="" />{/if}
    <span></span>
  </div>

  <header class="anime-playback-shell__context">
    <div class="anime-playback-shell__identity">
      <div class="anime-playback-shell__eyebrow">
        {#if sourceLabel}<span>{sourceLabel}</span>{/if}
        {#if episodePosition}<span>{episodePosition}</span>{/if}
      </div>
      <h2>{title || "番剧播放"}</h2>
      <div class="anime-playback-shell__episode-line">
        <p>{episodeTitle || "正在准备剧集"}</p>
        {#if nextEpisodeTitle}<small>下一集：{nextEpisodeTitle}</small>{/if}
      </div>
    </div>
    {#if headerActions}<div class="anime-playback-shell__header-actions">{@render headerActions()}</div>{/if}
  </header>

  {#if toolbar}<div class="anime-playback-shell__toolbar">{@render toolbar()}</div>{/if}

  <div class="anime-playback-shell__body" bind:this={bodyHost} class:anime-playback-shell__body--with-info={showInfoRail}>
    {#if showInfoRail}
      <aside class="anime-playback-shell__info" aria-label="当前番剧信息">
        <div class="anime-playback-shell__poster"><img src={artworkUrl ?? ""} alt="" /></div>
        <span class="anime-playback-shell__info-index">NOW / PLAYING</span>
        <strong>{title || "番剧播放"}</strong>
        <p>{episodeTitle || "正在准备剧集"}</p>
        <dl>
          {#if episodePosition}<div><dt>进度</dt><dd>{episodePosition}</dd></div>{/if}
          {#if sourceLabel}<div><dt>来源</dt><dd>{sourceLabel}</dd></div>{/if}
          {#if nextEpisodeTitle}<div><dt>下一集</dt><dd>{nextEpisodeTitle}</dd></div>{/if}
        </dl>
        <i aria-hidden="true"></i><i aria-hidden="true"></i><i aria-hidden="true"></i><i aria-hidden="true"></i>
      </aside>
    {/if}
    <div class="anime-playback-shell__stage" bind:this={stageHost} role="region" aria-label={stageLabel}>
      <div class="anime-playback-shell__media-frame" style={frameStyle}>
        {@render media()}
      </div>
    </div>
    {#if panelOpen && panel}<aside class="anime-playback-shell__panel">{@render panel()}</aside>{/if}
  </div>

  {#if footer}<footer class="anime-playback-shell__footer">{@render footer()}</footer>{/if}
</section>
