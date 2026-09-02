<script lang="ts">
  import type { HandheldMediaKind } from "./mediaTypes";

  interface Props {
    title: string;
    eyebrow?: string;
    subtitle?: string;
    cover?: string | null;
    fallback?: string;
    progress?: number;
    progressLabel?: string;
    kind?: HandheldMediaKind | "game";
    actionLabel: string;
    secondaryLabel?: string;
    onAction: () => void | Promise<void>;
    onSecondary?: () => void | Promise<void>;
  }

  let {
    title,
    eyebrow = "RECENT ACTIVITY",
    subtitle = "",
    cover = null,
    fallback = "",
    progress = 0,
    progressLabel = "",
    kind = "game",
    actionLabel,
    secondaryLabel,
    onAction,
    onSecondary,
  }: Props = $props();

  let imageFailed = $state(false);
  const safeProgress = $derived(Math.min(1, Math.max(0, Number.isFinite(progress) ? progress : 0)));
  const imageSource = $derived(!imageFailed && cover ? cover : fallback);

  function handleImageError() {
    if (fallback && imageSource !== fallback) imageFailed = true;
  }

  $effect(() => {
    cover;
    imageFailed = false;
  });
</script>

<section class="hh-hero hh-hero--{kind}" aria-label="最近活动">
  <div class="hh-hero__copy">
    <span class="hh-hero__eyebrow">{eyebrow}</span>
    <h1>{title}</h1>
    {#if subtitle}<p>{subtitle}</p>{/if}
    {#if progressLabel || safeProgress > 0}
      <div class="hh-hero__progress-copy"><span>{progressLabel || "进度"}</span><b>{Math.round(safeProgress * 100)}%</b></div>
      <div class="hh-hero__progress" aria-label={progressLabel || "进度"}><i style={`width:${safeProgress * 100}%`}></i></div>
    {/if}
    <div class="hh-hero__actions">
      <button type="button" class="hh-hero__action primary" onclick={() => void onAction()}>{actionLabel}</button>
      {#if secondaryLabel && onSecondary}<button type="button" class="hh-hero__action" onclick={() => void onSecondary?.()}>{secondaryLabel}</button>{/if}
    </div>
  </div>
  <div class="hh-hero__art" aria-hidden="true">
    {#if imageSource}<img src={imageSource} alt="" onerror={handleImageError} />{:else}<span>{title.slice(0, 2)}</span>{/if}
    <div class="hh-hero__art-wash"></div>
  </div>
</section>

<style>
  .hh-hero { position: relative; display: grid; grid-template-columns: minmax(0, 1fr) minmax(170px, 34%); min-height: 154px; overflow: hidden; border: 1px solid color-mix(in srgb, var(--accent, #e8557f) 30%, var(--border, #fff)); background: linear-gradient(115deg, color-mix(in srgb, var(--bg-card, #151820) 94%, transparent), color-mix(in srgb, var(--accent, #e8557f) 9%, transparent)); box-shadow: 0 16px 38px rgb(0 0 0 / .2); }
  .hh-hero--anime { --hero-accent: #f06d58; }
  .hh-hero--comic { --hero-accent: #55c4d8; }
  .hh-hero--novel { --hero-accent: #d5a35d; }
  .hh-hero__copy { position: relative; z-index: 1; display: flex; min-width: 0; flex-direction: column; align-items: flex-start; justify-content: center; gap: 7px; padding: 18px 22px; }
  .hh-hero__eyebrow { color: var(--hero-accent, var(--accent)); font: 700 9px/1 var(--font-mono, monospace); letter-spacing: .16em; }
  .hh-hero h1 { max-width: 100%; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font: 850 clamp(1.35rem, 3.2vw, 2rem)/1.05 var(--font-display, system-ui); letter-spacing: -.045em; }
  .hh-hero p { max-width: 560px; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-secondary, #b7bbc5); font-size: .78rem; }
  .hh-hero__progress-copy { display: flex; align-items: center; gap: 10px; color: var(--text-muted, #89909d); font: 700 9px/1 var(--font-mono, monospace); }
  .hh-hero__progress-copy b { color: var(--text-primary, #fff); }
  .hh-hero__progress { width: min(260px, 100%); height: 3px; overflow: hidden; background: rgb(255 255 255 / .12); }
  .hh-hero__progress i { display: block; height: 100%; background: var(--hero-accent, var(--accent)); box-shadow: 0 0 12px color-mix(in srgb, var(--hero-accent, var(--accent)) 60%, transparent); }
  .hh-hero__actions { display: flex; gap: 8px; margin-top: 3px; }
  .hh-hero__action { min-width: 44px; min-height: 38px; padding: 0 14px; border: 1px solid var(--border, rgb(255 255 255 / .16)); background: color-mix(in srgb, var(--bg-elev, #202532) 82%, transparent); color: var(--text-primary, #fff); font: 700 .72rem/1 var(--font-ui, system-ui); cursor: pointer; }
  .hh-hero__action.primary { border-color: var(--hero-accent, var(--accent)); background: var(--hero-accent, var(--accent)); color: #fff; }
  .hh-hero__action:hover, .hh-hero__action:focus-visible { border-color: var(--hero-accent, var(--accent)); }
  .hh-hero__action:focus-visible { outline: 2px solid color-mix(in srgb, var(--hero-accent, var(--accent)) 70%, white); outline-offset: 2px; }
  .hh-hero__art { position: relative; min-width: 0; overflow: hidden; background: linear-gradient(135deg, var(--bg-elev, #202532), var(--bg-card, #151820)); }
  .hh-hero__art img, .hh-hero__art-wash { position: absolute; inset: 0; width: 100%; height: 100%; }
  .hh-hero__art img { object-fit: cover; opacity: .66; filter: saturate(1.08); }
  .hh-hero__art-wash { background: linear-gradient(90deg, color-mix(in srgb, var(--bg-card, #151820) 88%, transparent), transparent 58%), linear-gradient(180deg, transparent 50%, rgb(4 5 8 / .42)); }
  .hh-hero__art > span { position: absolute; inset: 0; display: grid; place-items: center; color: var(--text-dim, #667080); font: 850 2.4rem/1 var(--font-display, system-ui); }
  @media (max-width: 620px) { .hh-hero { grid-template-columns: minmax(0, 1fr) 28%; } .hh-hero__copy { padding-inline: 14px; } .hh-hero__action { padding-inline: 10px; } }
  @media (prefers-reduced-motion: reduce) { .hh-hero__action { transition: none; } }
</style>
