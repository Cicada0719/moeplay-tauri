<script lang="ts">
  import { comicStore } from "../../stores/comic.svelte";
  import ComicCard from "./ComicCard.svelte";
  import Icon from "../Icon.svelte";

  const comic = $derived(comicStore.currentComic);
  const chapters = $derived(comicStore.chapters);
  let favouriting = $state(false);
  let liking = $state(false);

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
</script>

<div class="detail-overlay" role="dialog">
  <div class="detail-header">
    <button class="back-btn" onclick={() => comicStore.closeComic()}>
      <Icon name="chevronLeft" size={16} /> 返回
    </button>
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

        <div class="action-row">
          <button class="action-btn" class:active={comic.is_liked}
            onclick={handleLike} disabled={liking} title="喜欢">
            <Icon name={comic.is_liked ? "heartFill" : "heart"} size={15} />
            <span>{fmtNum(comic.likes_count)}</span>
          </button>
          <button class="action-btn" class:active={comic.is_favourite}
            onclick={handleFavourite} disabled={favouriting} title="收藏">
            <Icon name={comic.is_favourite ? "star" : "star"} size={15} />
            <span>{comic.is_favourite ? "已收藏" : "收藏"}</span>
          </button>
        </div>

        <div class="detail-stats-col">
          <div class="stat-item"><Icon name="eye" size={12} /><span>{fmtNum(comic.total_views)}</span></div>
          <div class="stat-item"><Icon name="collection" size={12} /><span>{comic.eps_count} 话 / {comic.pages_count} 页</span></div>
          {#if comic.comments_count > 0}
            <div class="stat-item"><Icon name="paperclip" size={12} /><span>{fmtNum(comic.comments_count)} 评论</span></div>
          {/if}
        </div>
      </aside>

      <!-- 右：信息 + 章节/评论/推荐 -->
      <div class="detail-info">
        <h1 class="detail-title">{comic.title}</h1>
        <div class="detail-meta-row">
          {#if comic.author}
            <span class="meta-chip"><Icon name="tag" size={11} /> {comic.author}</span>
          {/if}
          {#if comic.chinese_team}
            <span class="meta-chip">汉化: {comic.chinese_team}</span>
          {/if}
          {#if comic.finished}
            <span class="meta-chip accent">完结</span>
          {:else}
            <span class="meta-chip">连载中</span>
          {/if}
          {#if comic.updated_at}
            <span class="meta-chip">{fmtDate(comic.updated_at)}</span>
          {/if}
        </div>

        {#if comic.categories.length > 0 || comic.tags.length > 0}
          <div class="tags-row">
            {#each comic.categories as cat}
              <span class="tag">{cat}</span>
            {/each}
            {#each comic.tags.slice(0, 8) as tag}
              <span class="tag tag-dim">{tag}</span>
            {/each}
          </div>
        {/if}

        {#if comic.description}
          <p class="detail-desc">{comic.description}</p>
        {/if}

        <!-- 子 tab 切换 -->
        <div class="sub-tabs">
          <button class="sub-tab" class:active={detailTab === "chapters"}
            onclick={() => { detailTab = "chapters"; }}>
            章节 <span class="sub-count">{chapters.length}</span>
          </button>
          <button class="sub-tab" class:active={detailTab === "comments"}
            onclick={() => { detailTab = "comments"; }}>
            评论 <span class="sub-count">{comicStore.commentsTotal || comic.comments_count}</span>
          </button>
          <button class="sub-tab" class:active={detailTab === "recommend"}
            onclick={() => { detailTab = "recommend"; }}>
            推荐 <span class="sub-count">{comicStore.recommendations.length}</span>
          </button>
        </div>

        <!-- 章节列表 -->
        {#if detailTab === "chapters"}
          <div class="chapters-section">
            {#if chapters.length === 0}
              <p class="no-data">暂无章节</p>
            {:else}
              <div class="chapters-grid">
                {#each chapters as ch (ch.id)}
                  <button class="chapter-btn"
                    onclick={() => comicStore.openChapter(ch.order, ch.title)}>
                    {ch.title || `第 ${ch.order} 话`}
                  </button>
                {/each}
              </div>
            {/if}
          </div>

        <!-- 评论区 -->
        {:else if detailTab === "comments"}
          <div class="comments-section">
            {#if comic.allow_comment}
              <form class="comment-form" onsubmit={handlePostComment}>
                <input class="comment-input" type="text" bind:value={commentInput}
                  placeholder="写下你的评论..." disabled={postingComment} />
                <button type="submit" class="comment-submit"
                  disabled={!commentInput.trim() || postingComment}>发送</button>
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
                          <span class="comment-pin">置顶</span>
                        {/if}
                      </span>
                      <span class="comment-date">{fmtDate(c.created_at)}</span>
                    </div>
                    <p class="comment-content">{c.content}</p>
                    <div class="comment-footer">
                      <button class="comment-like" class:liked={c.is_liked}
                        onclick={() => comicStore.likeComment(c.id)}>
                        <Icon name={c.is_liked ? "heartFill" : "heart"} size={12} />
                        {c.likes_count || ""}
                      </button>
                      {#if c.comments_count > 0}
                        <span class="comment-replies">{c.comments_count} 回复</span>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
              {#if comicStore.commentsPage < comicStore.commentsPages}
                <button class="load-more-comments" onclick={() => comicStore.loadMoreComments()}
                  disabled={comicStore.commentsLoading}>
                  {comicStore.commentsLoading ? "加载中..." : "加载更多评论"}
                </button>
              {/if}
            {:else if !comicStore.commentsLoading}
              <p class="no-data">暂无评论</p>
            {:else}
              <div class="comments-loading"><div class="spinner-sm"></div></div>
            {/if}
          </div>

        <!-- 推荐 -->
        {:else if detailTab === "recommend"}
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
    <div class="error-center"><Icon name="x" size={24} /><p>{comicStore.error}</p></div>
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
  .back-btn {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 7px 14px 7px 10px;
    border: 1px solid var(--accent-ring, rgba(232,85,127,0.4)); border-radius: 999px;
    background: var(--accent-lo, rgba(232,85,127,0.1)); color: var(--accent, #e8557f);
    font-size: 13px; font-weight: 650; cursor: pointer; transition: background 0.15s, transform 0.15s;
  }
  .back-btn:hover { background: var(--accent, #e8557f); color: #fff; transform: translateX(-2px); }
  .detail-hint { font-size: 11px; color: var(--text-dim, var(--text-muted)); font-family: var(--font-mono); }

  .detail-body { flex: 1; display: flex; gap: 24px; padding: 20px 24px; overflow-y: auto; }

  /* ── aside ── */
  .detail-aside { flex-shrink: 0; width: 180px; display: flex; flex-direction: column; gap: 12px; }
  .detail-cover { width: 100%; border-radius: 8px; aspect-ratio: 2/3; object-fit: cover; background: var(--bg); border: 1px solid var(--border); }

  .action-row { display: flex; gap: 6px; }
  .action-btn {
    flex: 1; display: flex; align-items: center; justify-content: center; gap: 5px;
    padding: 8px 4px; border: 1px solid var(--border); border-radius: 8px;
    background: transparent; color: var(--text-muted); font-size: 12px; cursor: pointer; transition: all 0.15s;
  }
  .action-btn.active { border-color: var(--accent); background: var(--accent-lo, rgba(232,85,127,0.12)); color: var(--accent); }
  .action-btn:not(:disabled):hover { border-color: var(--accent); color: var(--accent); }

  .detail-stats-col { display: flex; flex-direction: column; gap: 6px; }
  .stat-item { display: flex; align-items: center; gap: 5px; color: var(--text-muted); font-size: 11.5px; }

  /* ── info ── */
  .detail-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 12px; }
  .detail-title { font-family: var(--font-display); font-size: 22px; font-weight: 750; color: var(--text-primary); margin: 0; line-height: 1.3; }

  .detail-meta-row { display: flex; flex-wrap: wrap; gap: 6px; }
  .meta-chip {
    font-size: 11.5px; padding: 3px 9px; border-radius: 6px;
    background: rgba(255,255,255,0.06); border: 1px solid var(--border); color: var(--text-muted);
    display: inline-flex; align-items: center; gap: 4px;
  }
  .meta-chip.accent { background: var(--accent-lo, rgba(232,85,127,0.12)); border-color: var(--accent-ring, rgba(232,85,127,0.3)); color: var(--accent, #e8557f); }

  .tags-row { display: flex; flex-wrap: wrap; gap: 5px; }
  .tag { font-size: 11px; padding: 2px 8px; border-radius: 5px; background: rgba(255,255,255,0.04); border: 1px solid var(--border); color: var(--text-muted); }
  .tag-dim { opacity: 0.6; }

  .detail-desc { font-size: 13px; color: var(--text-muted); line-height: 1.7; margin: 0; max-height: 80px; overflow-y: auto; }

  /* ── Sub tabs ── */
  .sub-tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border); padding-bottom: 8px; }
  .sub-tab {
    padding: 6px 14px; border: 1px solid transparent; border-radius: 6px;
    background: transparent; color: var(--text-muted);
    font-size: 13px; font-weight: 550; cursor: pointer; transition: all 0.15s;
    display: inline-flex; align-items: center; gap: 5px;
  }
  .sub-tab.active {
    background: var(--accent-lo, rgba(232,85,127,0.1));
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    color: var(--accent); font-weight: 700;
  }
  .sub-tab:not(.active):hover { background: rgba(255,255,255,0.04); color: var(--text-primary); }
  .sub-count { font-size: 10.5px; font-family: var(--font-mono); opacity: 0.7; }

  /* ── Chapters ── */
  .chapters-section { display: flex; flex-direction: column; gap: 10px; }
  .chapters-grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 6px; max-height: 400px; overflow-y: auto;
  }
  .chapter-btn {
    padding: 7px 10px; border: 1px solid var(--border); border-radius: 6px;
    background: rgba(255,255,255,0.03); color: var(--text-primary);
    font-size: 12px; cursor: pointer; transition: all 0.15s;
    text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .chapter-btn:hover { border-color: var(--accent); background: var(--accent-lo, rgba(232,85,127,0.1)); color: var(--accent); }

  /* ── Comments ── */
  .comments-section { display: flex; flex-direction: column; gap: 10px; }
  .comment-form { display: flex; gap: 8px; flex-shrink: 0; }
  .comment-input {
    flex: 1; padding: 8px 12px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.04); color: var(--text-primary);
    font-size: 13px; outline: none; transition: border-color 0.15s;
  }
  .comment-input:focus { border-color: var(--accent); }
  .comment-submit {
    padding: 0 16px; border: 1px solid var(--accent-ring, rgba(232,85,127,0.3));
    border-radius: 8px; background: var(--accent-lo, rgba(232,85,127,0.1));
    color: var(--accent); font-size: 13px; font-weight: 600; cursor: pointer;
  }
  .comment-submit:disabled { opacity: 0.4; cursor: not-allowed; }

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
  .comment-pin {
    font-size: 9px; padding: 1px 5px; border-radius: 3px;
    background: var(--accent-lo, rgba(232,85,127,0.12)); color: var(--accent); font-weight: 700;
  }
  .comment-date { font-size: 10.5px; color: var(--text-dim, var(--text-muted)); font-family: var(--font-mono); }
  .comment-content { font-size: 13px; color: var(--text-muted); margin: 0; line-height: 1.5; word-break: break-word; }
  .comment-footer { display: flex; align-items: center; gap: 12px; margin-top: 6px; }
  .comment-like {
    display: inline-flex; align-items: center; gap: 4px;
    background: transparent; border: none; color: var(--text-dim, var(--text-muted));
    font-size: 11px; cursor: pointer; padding: 2px 0; transition: color 0.15s;
  }
  .comment-like.liked { color: var(--accent); }
  .comment-like:hover { color: var(--accent); }
  .comment-replies { font-size: 11px; color: var(--text-dim, var(--text-muted)); }

  .load-more-comments {
    align-self: center; padding: 6px 20px; border: 1px solid var(--border); border-radius: 6px;
    background: transparent; color: var(--text-muted); font-size: 12px; cursor: pointer;
  }
  .load-more-comments:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .comments-loading { display: flex; justify-content: center; padding: 20px; }

  /* ── Recommend ── */
  .recommend-section { display: flex; flex-direction: column; gap: 10px; }
  .recommend-grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 10px; max-height: 400px; overflow-y: auto;
  }

  .no-data { color: var(--text-muted); font-size: 13px; margin: 0; text-align: center; padding: 20px 0; }

  .loading-center, .error-center { flex: 1; display: grid; place-items: center; color: var(--text-muted); gap: 10px; }
  .spinner { width: 32px; height: 32px; border: 3px solid rgba(255,255,255,0.1); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.7s linear infinite; }
  .spinner-sm { width: 20px; height: 20px; border: 2px solid rgba(255,255,255,0.1); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.7s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
