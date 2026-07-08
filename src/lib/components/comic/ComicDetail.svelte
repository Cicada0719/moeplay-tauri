<script lang="ts">
  import { comicStore } from "../../stores/comic.svelte";
  import ComicCard from "./ComicCard.svelte";
  import Icon from "../Icon.svelte";
  import { Button, EmptyState, Input, SegmentControl, Tag } from "../ui";

  const comic = $derived(comicStore.currentComic);
  const chapters = $derived(comicStore.chapters);
  const isPicacg = $derived(comicStore.isPicacgDetail);
  const latestChapter = $derived(chapters[chapters.length - 1]);
  let favouriting = $state(false);
  let liking = $state(false);
  let descExpanded = $state(false);

  // 评论
  let commentInput = $state("");
  let postingComment = $state(false);

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

  async function handlePostComment(e: Event) {
    e.preventDefault();
    if (!comic || !commentInput.trim()) return;
    postingComment = true;
    const ok = await comicStore.postComment(comic.id, commentInput.trim());
    if (ok) commentInput = "";
    postingComment = false;
  }

  function fmtNum(n: number) {
    return n >= 10000 ? (n / 10000).toFixed(1) + "w" : n >= 1000 ? (n / 1000).toFixed(1) + "k" : n.toString();
  }

  function fmtDate(s: string) {
    if (!s) return "";
    try {
      const d = new Date(s);
      return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`;
    } catch { return s; }
  }

  // 评论 tab
  let detailTab = $state<"chapters" | "comments" | "recommend">("chapters");

  const subTabs = [
    { value: "chapters", label: "章节" },
    { value: "comments", label: "评论" },
    { value: "recommend", label: "推荐" },
  ];

  const subTabOptions = $derived(
    subTabs.filter((o) => isPicacg || o.value === "chapters").map((o) => ({
      ...o,
      label:
        o.value === "chapters"
          ? `${o.label} ${chapters.length}`
          : o.value === "comments"
            ? `${o.label} ${comicStore.commentsTotal || comic?.comments_count || 0}`
            : `${o.label} ${comicStore.recommendations.length}`,
    })),
  );

  function handleSubTabChange(value: string) {
    detailTab = value as typeof detailTab;
  }

  $effect(() => {
    if (!isPicacg && detailTab !== "chapters") {
      detailTab = "chapters";
    }
  });
</script>

<div class="detail-overlay" role="dialog">
  <div class="detail-header">
    <Button variant="ghost" size="sm" press={() => comicStore.closeComic()}>
      <Icon name="chevronLeft" size={16} />
      返回
    </Button>
    <span class="detail-hint">Esc 关闭</span>
  </div>

  {#if comicStore.loading && !comic}
    <div class="loading-center"><div class="spinner"></div><span>加载中...</span></div>
  {:else if comic}
    <div class="detail-body">
      <!-- 左：封面 + 操作 -->
      <aside class="detail-aside">
        <img src={comic.thumb_url} alt={comic.title} class="detail-cover"
          onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = "none"; }} />

        {#if isPicacg}
          <div class="action-row">
            <Button variant={comic.is_liked ? "primary" : "ghost"} size="sm" fullWidth
              press={handleLike} disabled={liking} title="喜欢">
              <Icon name={comic.is_liked ? "heartFill" : "heart"} size={15} />
              <span>{fmtNum(comic.likes_count)}</span>
            </Button>
            <Button variant={comic.is_favourite ? "primary" : "ghost"} size="sm" fullWidth
              press={handleFavourite} disabled={favouriting} title="收藏">
              <Icon name={comic.is_favourite ? "star" : "star"} size={15} />
              <span>{comic.is_favourite ? "已收藏" : "收藏"}</span>
            </Button>
          </div>
        {/if}

        <div class="detail-stats-col">
          <div class="stat-item"><Icon name="eye" size={12} /><span>{fmtNum(comic.total_views)}</span></div>
          <div class="stat-item"><Icon name="collection" size={12} /><span>{comic.eps_count} 话 / {comic.pages_count} 页</span></div>
          {#if isPicacg && comic.comments_count > 0}
            <div class="stat-item"><Icon name="paperclip" size={12} /><span>{fmtNum(comic.comments_count)} 评论</span></div>
          {/if}
        </div>
      </aside>

      <!-- 右：信息 + 章节/评论/推荐 -->
      <div class="detail-info">
        <h1 class="detail-title">{comic.title}</h1>
        <div class="detail-meta-row">
          {#if comic.author}
            <Tag variant="muted" size="sm"><Icon name="tag" size={11} /> {comic.author}</Tag>
          {/if}
          {#if comic.chinese_team}
            <Tag variant="muted" size="sm">汉化: {comic.chinese_team}</Tag>
          {/if}
          {#if comic.finished}
            <Tag variant="accent" size="sm">完结</Tag>
          {:else}
            <Tag variant="muted" size="sm">连载中</Tag>
          {/if}
          {#if comic.updated_at}
            <Tag variant="muted" size="sm">{fmtDate(comic.updated_at)}</Tag>
          {/if}
        </div>

        {#if comic.categories.length > 0 || comic.tags.length > 0}
          <div class="tags-row">
            {#each comic.categories as cat}
              <Tag variant="accent" size="sm">{cat}</Tag>
            {/each}
            {#each comic.tags.slice(0, 8) as tag}
              <Tag variant="neutral" size="sm">{tag}</Tag>
            {/each}
          </div>
        {/if}

        {#if comic.description}
          <div class="desc-block">
            <p class="detail-desc" class:expanded={descExpanded}>{comic.description}</p>
            {#if comic.description.length > 72}
              <button class="desc-toggle" type="button" onclick={() => descExpanded = !descExpanded}>
                {descExpanded ? "收起简介" : "展开简介"}
              </button>
            {/if}
          </div>
        {/if}

        {#if !isPicacg}
          <div class="source-facts">
            <div>
              <span>来源</span>
              <b>{comic.categories[0] ?? comic.chinese_team ?? "普通源"}</b>
            </div>
            <div>
              <span>章节</span>
              <b>{chapters.length} 话</b>
            </div>
            <div>
              <span>最新</span>
              <b>{latestChapter?.title ?? "暂无"}</b>
            </div>
          </div>
        {/if}

        <!-- 子 tab 切换 -->
        <div class="sub-tabs">
          <SegmentControl options={subTabOptions} value={detailTab} onChange={handleSubTabChange} size="sm" />
        </div>

        <!-- 章节列表 -->
        {#if detailTab === "chapters"}
          <div class="chapters-section">
            {#if chapters.length === 0}
              <p class="no-data">暂无章节</p>
            {:else}
              <div class="chapters-grid">
                {#each chapters as ch (ch.id)}
                  <Button variant="ghost" size="sm" class="chapter-btn"
                    press={() => comicStore.openChapter(ch.order, ch.title)}>
                    {ch.title || `第 ${ch.order} 话`}
                  </Button>
                {/each}
              </div>
            {/if}
          </div>

        <!-- 评论区 -->
        {:else if detailTab === "comments" && isPicacg}
          <div class="comments-section">
            {#if comic.allow_comment}
              <form class="comment-form" onsubmit={handlePostComment}>
                <Input class="comment-input" bind:value={commentInput}
                  placeholder="写下你的评论..." disabled={postingComment} />
                <Button variant="primary" type="submit" size="sm"
                  disabled={!commentInput.trim() || postingComment}>发送</Button>
              </form>
            {/if}

            {#if comicStore.comments.length > 0}
              <div class="comments-list">
                {#each comicStore.comments as c (c.id)}
                  <div class="comment-item" class:is-top={c.is_top}>
                    <div class="comment-header">
                      <span class="comment-user">
                        <span class="comment-level">Lv.{c.user.level}</span>
                        {c.user.name}
                        {#if c.user.title}
                          <span class="comment-badge">{c.user.title}</span>
                        {/if}
                        {#if c.is_top}
                          <Tag variant="accent" size="sm">置顶</Tag>
                        {/if}
                      </span>
                      <span class="comment-date">{fmtDate(c.created_at)}</span>
                    </div>
                    <p class="comment-content">{c.content}</p>
                    <div class="comment-footer">
                      <Button variant="quiet" size="sm" class="comment-like {c.is_liked ? 'active-like' : ''}"
                        press={() => comicStore.likeComment(c.id)}>
                        <Icon name={c.is_liked ? "heartFill" : "heart"} size={12} />
                        {c.likes_count || ""}
                      </Button>
                      {#if c.comments_count > 0}
                        <span class="comment-replies">{c.comments_count} 回复</span>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
              {#if comicStore.commentsPage < comicStore.commentsPages}
                <Button variant="ghost" size="sm" class="load-more-comments"
                  press={() => comicStore.loadMoreComments()}
                  disabled={comicStore.commentsLoading}>
                  {comicStore.commentsLoading ? "加载中..." : "加载更多评论"}
                </Button>
              {/if}
            {:else if !comicStore.commentsLoading}
              <p class="no-data">暂无评论</p>
            {:else}
              <div class="comments-loading"><div class="spinner-sm"></div></div>
            {/if}
          </div>

        <!-- 推荐 -->
        {:else if detailTab === "recommend" && isPicacg}
          <div class="recommend-section">
            {#if comicStore.recommendations.length > 0}
              <div class="recommend-grid">
                {#each comicStore.recommendations as rec (rec.id)}
                  <ComicCard comic={rec} onclick={() => comicStore.openComic(rec.id)} />
                {/each}
              </div>
            {:else}
              <p class="no-data">暂无推荐</p>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {:else if comicStore.error}
    <EmptyState class="error-center" icon="x" title={comicStore.error} />
  {/if}
</div>

<style>
  .detail-overlay {
    position: absolute; inset: 0; background: var(--bg-deep); z-index: 20;
    display: flex; flex-direction: column; overflow: hidden;
    animation: slide-in 0.22s ease;
  }
  @keyframes slide-in { from { transform: translateX(40px); opacity: 0; } to { transform: translateX(0); opacity: 1; } }

  .detail-header {
    flex-shrink: 0; display: flex; align-items: center; gap: 12px;
    padding: 12px 20px 8px; border-bottom: 1px solid var(--border);
  }
  .detail-hint { font-size: 11px; color: var(--text-dim, var(--text-muted)); font-family: var(--font-mono); }

  .detail-body { flex: 1; display: flex; gap: 24px; padding: 20px 24px; overflow-y: auto; }

  /* ── aside ── */
  .detail-aside { flex-shrink: 0; width: 180px; display: flex; flex-direction: column; gap: 12px; }
  .detail-cover { width: 100%; border-radius: 8px; aspect-ratio: 2/3; object-fit: cover; background: var(--bg); border: 1px solid var(--border); }

  .action-row { display: flex; gap: 6px; }

  .detail-stats-col { display: flex; flex-direction: column; gap: 6px; }
  .stat-item { display: flex; align-items: center; gap: 5px; color: var(--text-muted); font-size: 11.5px; }

  /* ── info ── */
  .detail-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 12px; }
  .detail-title { font-family: var(--font-display); font-size: 22px; font-weight: 750; color: var(--text-primary); margin: 0; line-height: 1.3; }

  .detail-meta-row { display: flex; flex-wrap: wrap; gap: 6px; }

  .tags-row { display: flex; flex-wrap: wrap; gap: 5px; }

  .desc-block {
    display: grid;
    gap: 6px;
  }

  .detail-desc {
    min-width: 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.7;
    margin: 0;
    overflow-wrap: anywhere;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .detail-desc.expanded {
    display: block;
    max-height: 180px;
    overflow-y: auto;
  }

  .desc-toggle {
    justify-self: start;
    border: 0;
    padding: 0;
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }

  .source-facts {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .source-facts > div {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(255,255,255,0.025);
    padding: 8px 10px;
    display: grid;
    gap: 3px;
  }

  .source-facts span {
    color: var(--text-dim, var(--text-muted));
    font-size: 10.5px;
  }

  .source-facts b {
    min-width: 0;
    color: var(--text-primary);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Sub tabs ── */
  .sub-tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border); padding-bottom: 8px; }

  /* ── Chapters ── */
  .chapters-section { display: flex; flex-direction: column; gap: 10px; }
  .chapters-grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 6px; max-height: 400px; overflow-y: auto;
  }
  :global(.chapter-btn) {
    justify-content: flex-start;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Comments ── */
  .comments-section { display: flex; flex-direction: column; gap: 10px; }
  .comment-form { display: flex; gap: 8px; flex-shrink: 0; }
  :global(.ui-input.comment-input) { flex: 1; }

  .comments-list { display: flex; flex-direction: column; gap: 2px; max-height: 400px; overflow-y: auto; }
  .comment-item {
    padding: 10px 12px; border-radius: 8px; background: rgba(255,255,255,0.02);
    border: 1px solid transparent; transition: border-color 0.15s;
  }
  .comment-item:hover { border-color: var(--border); }
  .comment-item.is-top { border-color: var(--accent-ring, rgba(232,85,127,0.2)); background: rgba(232,85,127,0.03); }

  .comment-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 4px; }
  .comment-user { display: flex; align-items: center; gap: 5px; font-size: 12.5px; color: var(--text-primary); font-weight: 600; }
  .comment-level { font-family: var(--font-mono); font-size: 10px; color: var(--accent); font-weight: 700; }
  .comment-badge { font-size: 10px; padding: 1px 5px; border-radius: 3px; background: rgba(255,255,255,0.08); color: var(--text-muted); }
  .comment-date { font-size: 10.5px; color: var(--text-dim, var(--text-muted)); font-family: var(--font-mono); }
  .comment-content { font-size: 13px; color: var(--text-muted); margin: 0; line-height: 1.5; word-break: break-word; }
  .comment-footer { display: flex; align-items: center; gap: 12px; margin-top: 6px; }
  :global(.comment-like) {
    padding: 2px 0;
    min-height: auto;
  }
  :global(.comment-like.active-like) {
    color: var(--accent);
  }

  .comment-replies { font-size: 11px; color: var(--text-dim, var(--text-muted)); }

  :global(.load-more-comments) { align-self: center; }

  .comments-loading { display: flex; justify-content: center; padding: 20px; }

  /* ── Recommend ── */
  .recommend-section { display: flex; flex-direction: column; gap: 10px; }
  .recommend-grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 10px; max-height: 400px; overflow-y: auto;
  }

  .no-data { color: var(--text-muted); font-size: 13px; margin: 0; text-align: center; padding: 20px 0; }

  .loading-center, :global(.error-center) { flex: 1; display: grid; place-items: center; color: var(--text-muted); gap: 10px; }
  .spinner { width: 32px; height: 32px; border: 3px solid rgba(255,255,255,0.1); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.7s linear infinite; }
  .spinner-sm { width: 20px; height: 20px; border: 2px solid rgba(255,255,255,0.1); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.7s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
