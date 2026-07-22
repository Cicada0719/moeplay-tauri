<script lang="ts">
  import type { ReturnFocusTarget } from "../../actions/a11y/focusTrap";
  import { comicStore, type ComicChapter } from "../../stores/comic.svelte";
  import ComicCard from "./ComicCard.svelte";
  import Icon from "../Icon.svelte";
  import { Button, Input, Tag } from "../ui";
  import { AsyncSection, ContentGrid, DetailPanel } from "../ui-v2";

  let {
    onclose,
    onopenchapter,
    returnFocus = true,
  }: {
    onclose?: () => void;
    onopenchapter?: (chapter: ComicChapter, event: MouseEvent) => void;
    returnFocus?: ReturnFocusTarget;
  } = $props();

  const comic = $derived(comicStore.currentComic);
  const chapters = $derived(comicStore.chapters);
  const isPicacg = $derived(comicStore.isPicacgDetail);
  const latestChapter = $derived(chapters[chapters.length - 1]);
  let favouriting = $state(false);
  let liking = $state(false);
  let descExpanded = $state(false);
  let commentInput = $state("");
  let postingComment = $state(false);
  let detailTab = $state<"chapters" | "comments" | "recommend">("chapters");

  const subTabs = $derived([
    { value: "chapters" as const, label: `章节 ${chapters.length}` },
    ...(isPicacg
      ? [
          { value: "comments" as const, label: `评论 ${comicStore.commentsTotal || comic?.comments_count || 0}` },
          { value: "recommend" as const, label: `推荐 ${comicStore.recommendations.length}` },
        ]
      : []),
  ]);

  const detailDescription = $derived(comic
    ? `${comic.author || "未知作者"} · ${comic.finished ? "已完结" : "连载中"} · ${chapters.length} 话`
    : "正在加载漫画信息");

  async function handleFavourite() {
    if (!comic) return;
    favouriting = true;
    try { await comicStore.toggleFavourite(comic.id); } finally { favouriting = false; }
  }

  async function handleLike() {
    if (!comic) return;
    liking = true;
    try { await comicStore.toggleLike(comic.id); } finally { liking = false; }
  }

  async function handlePostComment(event: Event) {
    event.preventDefault();
    if (!comic || !commentInput.trim()) return;
    postingComment = true;
    const ok = await comicStore.postComment(comic.id, commentInput.trim());
    if (ok) commentInput = "";
    postingComment = false;
  }

  function closeDetail() {
    if (onclose) onclose();
    else comicStore.closeComic();
  }

  function openChapter(chapter: ComicChapter, event: MouseEvent) {
    if (onopenchapter) onopenchapter(chapter, event);
    else void comicStore.openChapter(chapter.order, chapter.title);
  }

  function openRecommendation(id: string) {
    void comicStore.openComic(id);
  }

  function fmtNum(value: number) {
    return value >= 10000 ? `${(value / 10000).toFixed(1)}w` : value >= 1000 ? `${(value / 1000).toFixed(1)}k` : value.toString();
  }

  function fmtDate(value: string) {
    if (!value) return "";
    const date = new Date(value);
    if (Number.isNaN(date.valueOf())) return value;
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
  }

  function focusTab(index: number) {
    const target = document.querySelector<HTMLElement>(`[data-comic-detail-tab-index="${index}"]`);
    target?.focus();
  }

  function handleTabKeydown(event: KeyboardEvent, index: number) {
    let next = index;
    if (event.key === "ArrowRight") next = (index + 1) % subTabs.length;
    else if (event.key === "ArrowLeft") next = (index - 1 + subTabs.length) % subTabs.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = subTabs.length - 1;
    else return;
    event.preventDefault();
    detailTab = subTabs[next].value;
    queueMicrotask(() => focusTab(next));
  }

  $effect(() => {
    if (!isPicacg && detailTab !== "chapters") detailTab = "chapters";
  });
</script>

<DetailPanel
  open
  title={comic?.title ?? "漫画详情"}
  description={detailDescription}
  onClose={closeDetail}
  side="right"
  size="lg"
  initialFocus=".v2-detail-panel__close"
  {returnFocus}
  class="comic-detail-panel"
>
  {#if comicStore.loading && !comic}
    <AsyncSection title="漫画详情" state="loading" loadingDelayMs={0} />
  {:else if comic}
    <div class="detail-layout">
      <aside class="detail-aside" aria-label="漫画封面与状态">
        <img
          src={comic.thumb_url}
          alt={`${comic.title} 封面`}
          class="detail-cover"
          onerror={(event) => { (event.currentTarget as HTMLImageElement).hidden = true; }}
        />

        {#if isPicacg}
          <div class="action-row" data-gamepad-group>
            <Button variant={comic.is_liked ? "primary" : "ghost"} size="sm" fullWidth press={handleLike} disabled={liking} ariaLabel={comic.is_liked ? "取消喜欢" : "喜欢漫画"} gamepadActivate={comic.is_liked ? "取消喜欢" : "喜欢"}>
              <Icon name={comic.is_liked ? "heartFill" : "heart"} size={15} />
              {fmtNum(comic.likes_count)}
            </Button>
            <Button variant={comic.is_favourite ? "primary" : "ghost"} size="sm" fullWidth press={handleFavourite} disabled={favouriting} ariaLabel={comic.is_favourite ? "取消收藏" : "收藏漫画"} gamepadActivate={comic.is_favourite ? "取消收藏" : "收藏"} gamepadSecondaryAction>
              <Icon name="star" size={15} />
              {comic.is_favourite ? "已收藏" : "收藏"}
            </Button>
          </div>
        {/if}

        <dl class="detail-facts">
          <div><dt>来源</dt><dd>{comic.categories[0] ?? comic.chinese_team ?? "普通源"}</dd></div>
          <div><dt>章节</dt><dd>{chapters.length} 话</dd></div>
          <div><dt>最新</dt><dd>{latestChapter?.title ?? "暂无"}</dd></div>
          {#if comic.total_views > 0}<div><dt>阅读</dt><dd>{fmtNum(comic.total_views)}</dd></div>{/if}
        </dl>
      </aside>

      <div class="detail-content">
        <div class="detail-meta-row">
          {#if comic.author}<Tag variant="muted" size="sm"><Icon name="tag" size={11} /> {comic.author}</Tag>{/if}
          {#if comic.chinese_team}<Tag variant="muted" size="sm">汉化：{comic.chinese_team}</Tag>{/if}
          <Tag variant={comic.finished ? "accent" : "muted"} size="sm">{comic.finished ? "完结" : "连载中"}</Tag>
          {#if comic.updated_at}<Tag variant="muted" size="sm">更新 {fmtDate(comic.updated_at)}</Tag>{/if}
        </div>

        {#if comic.categories.length > 0 || comic.tags.length > 0}
          <div class="tags-row" aria-label="漫画分类与标签">
            {#each comic.categories as category}<Tag variant="accent" size="sm">{category}</Tag>{/each}
            {#each comic.tags.slice(0, 8) as tag}<Tag variant="neutral" size="sm">{tag}</Tag>{/each}
          </div>
        {/if}

        {#if comic.description}
          <div class="description-block">
            <p class:expanded={descExpanded}>{comic.description}</p>
            {#if comic.description.length > 100}
              <button type="button" onclick={() => (descExpanded = !descExpanded)} aria-expanded={descExpanded} data-gamepad-activate={descExpanded ? "收起简介" : "展开简介"}>
                {descExpanded ? "收起简介" : "展开简介"}
              </button>
            {/if}
          </div>
        {/if}

        <div class="detail-tabs" role="tablist" aria-label="漫画详情内容">
          {#each subTabs as tab, index (tab.value)}
            <button
              type="button"
              role="tab"
              aria-selected={detailTab === tab.value}
              aria-controls={`comic-detail-panel-${tab.value}`}
              tabindex={detailTab === tab.value ? 0 : -1}
              data-comic-detail-tab-index={index}
              data-gamepad-activate="切换内容"
              class:active={detailTab === tab.value}
              onclick={() => (detailTab = tab.value)}
              onkeydown={(event) => handleTabKeydown(event, index)}
            >{tab.label}</button>
          {/each}
        </div>

        <div id={`comic-detail-panel-${detailTab}`} role="tabpanel" tabindex="0" class="detail-tabpanel">
          {#if detailTab === "chapters"}
            <AsyncSection
              title="章节"
              description="选择章节后会打开沉浸式阅读器，关闭后焦点返回原章节。"
              state={chapters.length > 0 ? "ready" : "empty"}
              compact
            >
              <div class="chapters-grid" aria-label={`${comic.title} 章节列表`}>
                {#each chapters as chapter (chapter.id)}
                  {@const chapterKey = `${comic.id}:${chapter.order}`}
                  <button
                    type="button"
                    class="chapter-button"
                    data-chapter-focus-key={chapterKey}
                    data-gamepad-label={`阅读 ${chapter.title || `第 ${chapter.order} 话`}`}
                    data-gamepad-activate="开始阅读"
                    onclick={(event) => openChapter(chapter, event)}
                  >
                    <span class="chapter-order">{chapter.order}</span>
                    <span class="chapter-copy">
                      <strong>{chapter.title || `第 ${chapter.order} 话`}</strong>
                      {#if chapter.updated_at}<small>{fmtDate(chapter.updated_at)}</small>{/if}
                    </span>
                    <Icon name="chevronRight" size={15} />
                  </button>
                {/each}
              </div>
            </AsyncSection>
          {:else if detailTab === "comments" && isPicacg}
            <AsyncSection
              title="评论"
              state={comicStore.commentsLoading && comicStore.comments.length === 0 ? "loading" : comicStore.comments.length > 0 ? "ready" : "empty"}
              compact
              loadingDelayMs={0}
            >
              {#if comic.allow_comment}
                <form class="comment-form" onsubmit={handlePostComment}>
                  <Input class="comment-input" bind:value={commentInput} placeholder="写下你的评论..." disabled={postingComment} ariaLabel="漫画评论" />
                  <Button variant="primary" type="submit" size="sm" disabled={!commentInput.trim() || postingComment} loading={postingComment}>发送</Button>
                </form>
              {/if}

              <div class="comments-list">
                {#each comicStore.comments as comment (comment.id)}
                  <article class="comment-item" class:is-top={comment.is_top}>
                    <header>
                      <strong><span class="comment-level">Lv.{comment.user.level}</span> {comment.user.name}</strong>
                      <time datetime={comment.created_at}>{fmtDate(comment.created_at)}</time>
                    </header>
                    <p>{comment.content}</p>
                    <footer>
                      <Button variant="quiet" size="sm" press={() => comicStore.likeComment(comment.id)} ariaLabel={comment.is_liked ? "取消赞同评论" : "赞同评论"}>
                        <Icon name={comment.is_liked ? "heartFill" : "heart"} size={12} />{comment.likes_count || ""}
                      </Button>
                      {#if comment.comments_count > 0}<span>{comment.comments_count} 回复</span>{/if}
                    </footer>
                  </article>
                {/each}
              </div>
              {#if comicStore.commentsPage < comicStore.commentsPages}
                <Button variant="ghost" size="sm" press={() => comicStore.loadMoreComments()} disabled={comicStore.commentsLoading} loading={comicStore.commentsLoading}>加载更多评论</Button>
              {/if}
            </AsyncSection>
          {:else if detailTab === "recommend" && isPicacg}
            <AsyncSection title="相关推荐" state={comicStore.recommendations.length > 0 ? "ready" : "empty"} compact>
              <ContentGrid minItemWidth="9rem" gap="sm" label="相关推荐漫画">
                {#each comicStore.recommendations as recommendation (recommendation.id)}
                  <ComicCard comic={recommendation} focusKey={`recommend:${recommendation.id}`} onclick={() => openRecommendation(recommendation.id)} />
                {/each}
              </ContentGrid>
            </AsyncSection>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <AsyncSection
      title="漫画详情加载失败"
      state="error"
      description={comicStore.error || "无法读取漫画详情，请返回后重试。"}
      primaryAction={{ label: "返回漫画列表", onSelect: closeDetail }}
    />
  {/if}
</DetailPanel>

<style>
  :global(.comic-detail-panel .v2-detail-panel__body) {
    padding: 0;
    background:
      radial-gradient(circle at 4% 8%, color-mix(in srgb, var(--v2-color-accent) 12%, transparent), transparent 24rem),
      linear-gradient(145deg, var(--v2-color-surface), color-mix(in srgb, var(--v2-color-surface-subtle) 92%, var(--v2-color-accent)));
  }

  .detail-layout {
    display: grid;
    grid-template-columns: minmax(10rem, 13rem) minmax(0, 1fr);
    min-height: 100%;
  }

  .detail-aside {
    position: relative;
    display: flex;
    padding: var(--v2-space-5);
    flex-direction: column;
    gap: var(--v2-space-4);
    border-right: 1px solid var(--v2-color-border);
    overflow: hidden;
    background:
      repeating-linear-gradient(125deg, transparent 0 36px, color-mix(in srgb, var(--v2-color-text) 3%, transparent) 36px 37px),
      color-mix(in srgb, var(--v2-color-surface-subtle) 94%, var(--v2-color-accent));
  }

  .detail-cover { position: relative; width: 100%; aspect-ratio: 2 / 3; border: 1px solid color-mix(in srgb, var(--v2-color-border) 75%, transparent); border-radius: var(--v2-radius-lg); object-fit: cover; background: var(--v2-color-surface); box-shadow: 0 .9rem 2rem color-mix(in srgb, #020617 22%, transparent); }
  .action-row { display: grid; grid-template-columns: 1fr 1fr; gap: var(--v2-space-2); }
  .detail-facts { display: grid; gap: var(--v2-space-2); margin: 0; }
  .detail-facts div { display: grid; grid-template-columns: 3.4rem minmax(0, 1fr); gap: var(--v2-space-2); padding-block: var(--v2-space-2); border-bottom: 1px solid var(--v2-color-border); }
  .detail-facts dt { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .detail-facts dd { min-width: 0; margin: 0; overflow: hidden; color: var(--v2-color-text); font-size: var(--v2-text-xs); font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }

  .detail-content { min-width: 0; padding: clamp(var(--v2-space-4), 2.2vw, var(--v2-space-6)); }
  .detail-meta-row, .tags-row { display: flex; flex-wrap: wrap; gap: var(--v2-space-2); }
  .tags-row { margin-top: var(--v2-space-3); }
  .description-block { margin-top: var(--v2-space-4); padding: var(--v2-space-4); border: 1px solid color-mix(in srgb, var(--v2-color-border) 82%, transparent); border-radius: var(--v2-radius-lg); background: color-mix(in srgb, var(--v2-color-surface) 74%, transparent); box-shadow: inset 3px 0 0 color-mix(in srgb, var(--v2-color-accent) 58%, transparent); }
  .description-block p { display: -webkit-box; margin: 0; overflow: hidden; color: var(--v2-color-text-secondary); font-size: var(--v2-text-sm); line-height: 1.65; -webkit-box-orient: vertical; -webkit-line-clamp: 4; line-clamp: 4; }
  .description-block p.expanded { display: block; }
  .description-block button { margin-top: var(--v2-space-2); padding: 0; border: 0; background: none; color: var(--v2-color-accent); font: inherit; cursor: pointer; }
  .description-block button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }

  .detail-tabs { display: flex; margin-top: var(--v2-space-5); gap: var(--v2-space-1); border-bottom: 1px solid var(--v2-color-border); }
  .detail-tabs button { min-height: 2.75rem; padding: 0.65rem 0.9rem; border: 0; border-bottom: 2px solid transparent; background: transparent; color: var(--v2-color-text-secondary); font: inherit; font-weight: 650; cursor: pointer; }
  .detail-tabs button.active { border-bottom-color: var(--v2-color-accent); color: var(--v2-color-text); }
  .detail-tabs button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .detail-tabpanel { min-height: 16rem; padding-top: var(--v2-space-4); outline: none; }

  .chapters-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr)); gap: var(--v2-space-2); }
  .chapter-button { display: grid; grid-template-columns: 2.2rem minmax(0, 1fr) auto; align-items: center; gap: var(--v2-space-2); min-height: 3.25rem; padding: 0.65rem 0.8rem; border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: linear-gradient(145deg, var(--v2-color-surface), color-mix(in srgb, var(--v2-color-surface-subtle) 94%, var(--v2-color-accent))); color: var(--v2-color-text); text-align: left; cursor: pointer; transition: border-color var(--v2-motion-fast) var(--v2-ease-standard), transform var(--v2-motion-fast) var(--v2-ease-standard); }
  .chapter-button:hover { border-color: var(--v2-color-accent); background: var(--v2-color-surface-subtle); transform: translateY(-1px); }
  .chapter-button:focus-visible { outline: none; box-shadow: var(--v2-focus-ring); }
  .chapter-order { display: grid; width: 2rem; height: 2rem; place-items: center; border-radius: var(--v2-radius-sm); background: var(--v2-color-surface-subtle); color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .chapter-copy { min-width: 0; }
  .chapter-copy strong, .chapter-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chapter-copy strong { font-size: var(--v2-text-sm); }
  .chapter-copy small { margin-top: 0.18rem; color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }

  .comment-form { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: var(--v2-space-2); margin-bottom: var(--v2-space-4); }
  .comments-list { display: grid; gap: var(--v2-space-3); }
  .comment-item { padding: var(--v2-space-3); border: 1px solid var(--v2-color-border); border-radius: var(--v2-radius-md); background: var(--v2-color-surface); }
  .comment-item.is-top { border-color: color-mix(in srgb, var(--v2-color-accent) 45%, var(--v2-color-border)); }
  .comment-item header, .comment-item footer { display: flex; align-items: center; justify-content: space-between; gap: var(--v2-space-2); }
  .comment-item time, .comment-item footer { color: var(--v2-color-text-secondary); font-size: var(--v2-text-xs); }
  .comment-item p { color: var(--v2-color-text); font-size: var(--v2-text-sm); line-height: 1.55; }
  .comment-level { color: var(--v2-color-accent); font-size: var(--v2-text-xs); }

  @media (max-width: 48rem) {
    .detail-layout { grid-template-columns: 1fr; }
    .detail-aside { display: grid; grid-template-columns: 8rem minmax(0, 1fr); align-items: start; border-right: 0; border-bottom: 1px solid var(--v2-color-border); }
    .detail-cover { grid-row: 1 / span 3; }
    .action-row { grid-column: 2; }
    .detail-facts { grid-column: 2; }
  }

  @media (max-width: 32rem) {
    .detail-aside { grid-template-columns: 6.5rem minmax(0, 1fr); padding: var(--v2-space-3); }
    .detail-content { padding: var(--v2-space-3); }
    .chapters-grid { grid-template-columns: 1fr; }
    .comment-form { grid-template-columns: 1fr; }
  }

  @media (prefers-reduced-motion: reduce) {
    .chapter-button { transition: none; }
    .chapter-button:hover { transform: none; }
  }

  @media (prefers-contrast: more) {
    :global(.comic-detail-panel .v2-detail-panel__body),
    .detail-aside,
    .description-block,
    .chapter-button { background: var(--v2-color-surface); box-shadow: none; }
  }

  @media (max-height: 520px) and (orientation: landscape) {
    .detail-layout { grid-template-columns: minmax(8rem, 10rem) minmax(0, 1fr); }
    .detail-aside { display:flex; padding:var(--v2-space-3); border-right:1px solid var(--v2-color-border); border-bottom:0; gap:var(--v2-space-2); }
    .detail-cover { max-height:calc(100dvh - 10rem); }
    .detail-content { padding:var(--v2-space-3); }
    .description-block { margin-top:var(--v2-space-2); padding:var(--v2-space-2); }
    .description-block p { -webkit-line-clamp:2; line-clamp:2; }
    .detail-tabs { margin-top:var(--v2-space-3); }
    .detail-tabpanel { padding-top:var(--v2-space-2); }
    .chapters-grid { grid-template-columns:repeat(auto-fill,minmax(10rem,1fr)); }
  }
</style>
