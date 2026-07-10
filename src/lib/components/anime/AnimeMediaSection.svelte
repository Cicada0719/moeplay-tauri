<script lang="ts">
  import type { BangumiSubject } from "../../stores/anime.svelte";
  import { AsyncSection, ContentGrid, MediaCard } from "../ui-v2";
  import type { ViewState } from "../ui-v2";

  let {
    title,
    description,
    subjects,
    state = "ready",
    getImage,
    onOpen,
    onMore,
    moreAvailable = false,
    moreBusy = false,
    accent = "seasonal",
  }: {
    title: string;
    description?: string;
    subjects: BangumiSubject[];
    state?: ViewState;
    getImage: (url: string) => string;
    onOpen: (subject: BangumiSubject, trigger: HTMLElement) => void;
    onMore?: () => void | Promise<void>;
    moreAvailable?: boolean;
    moreBusy?: boolean;
    accent?: "seasonal" | "trending" | "toprated";
  } = $props();
</script>

{#snippet actions()}
  {#if moreAvailable && onMore}
    <button class="section-action" type="button" disabled={moreBusy} onclick={() => void onMore?.()}>
      {moreBusy ? "加载中…" : "显示更多"}
    </button>
  {/if}
{/snippet}

<AsyncSection
  {title}
  {description}
  {state}
  {actions}
  preserveContent={subjects.length > 0}
  loadingRows={5}
  class="anime-media-section anime-media-section--{accent}"
>
  <ContentGrid label={title} minItemWidth="9.5rem" gap="md" busy={state === "loading" || state === "refreshing"}>
    {#each subjects as subject (subject.id)}
      {@const displayTitle = subject.name_cn || subject.name}
      {@const image = subject.image ? getImage(subject.image) : ""}
      <MediaCard
        title={displayTitle}
        subtitle={subject.air_date || (subject.name_cn && subject.name !== subject.name_cn ? subject.name : undefined)}
        description={subject.rating > 0 ? `Bangumi ${subject.rating.toFixed(1)} 分${subject.rank > 0 ? ` · 排名 #${subject.rank}` : ""}` : undefined}
        imageSrc={image || undefined}
        imageAlt={displayTitle}
        variant="poster"
        focusKey={`anime-subject-${subject.id}`}
        ariaLabel={`查看 ${displayTitle} 详情`}
        onActivate={(event) => onOpen(subject, event.currentTarget as HTMLElement)}
      />
    {/each}
  </ContentGrid>
</AsyncSection>

<style>
  :global(.anime-media-section) {
    position: relative;
    padding-inline-start: var(--v2-space-3);
  }
  :global(.anime-media-section)::before {
    content: "";
    position: absolute;
    inset: 0 auto 0 0;
    width: 2px;
    border-radius: 999px;
    background: var(--anime-section-accent, var(--v2-color-accent));
    opacity: .82;
  }
  :global(.anime-media-section--seasonal) { --anime-section-accent: #45b8a0; }
  :global(.anime-media-section--trending) { --anime-section-accent: #df6f83; }
  :global(.anime-media-section--toprated) { --anime-section-accent: #d2a64a; }
  .section-action {
    min-height: 2.5rem;
    padding: .55rem .9rem;
    border: 1px solid var(--v2-color-border);
    border-radius: var(--v2-radius-md);
    background: var(--v2-color-surface-raised);
    color: var(--v2-color-text);
    font: inherit;
    font-weight: 650;
    cursor: pointer;
  }
  .section-action:hover:not(:disabled) { border-color: var(--v2-color-border-strong); transform: translateY(-1px); }
  .section-action:disabled { cursor: wait; opacity: .58; }
  @media (prefers-reduced-motion: reduce) {
    .section-action { transition: none; }
    .section-action:hover:not(:disabled) { transform: none; }
  }
  :global([data-motion="reduce"]) .section-action { transition: none; }
  :global([data-motion="reduce"]) .section-action:hover:not(:disabled) { transform: none; }
</style>
