<script lang="ts">
  import { animeStore, COLLECT_TYPES } from '../../stores/anime.svelte';
  import Icon from '../Icon.svelte';
  import { BackgroundLayer, Card, EmptyState, Tag } from '../ui';
  import { DetailPanel } from '../ui-v2';
  import type { ReturnFocusTarget } from '../../actions/a11y/focusTrap';
  import { focusRovingItem, nextRovingIndex } from './a11y';

  let { returnFocus = true }: { returnFocus?: ReturnFocusTarget } = $props();

  const DETAIL_TABS = [
    { id: 'overview', label: '概览' },
    { id: 'comments', label: '吐槽' },
    { id: 'characters', label: '角色' },
    { id: 'staff', label: '制作人员' },
  ] as const;
  type DetailTab = (typeof DETAIL_TABS)[number]['id'];
  let detailTabRefs: Array<HTMLButtonElement | null> = [];

  // ── Reactive data from store ──────────────────────────────────────────
  const subject = $derived(animeStore.detailSubject);
  const rating = $derived(animeStore.detailRating);
  const roads = $derived(animeStore.roads);
  const loading = $derived(animeStore.loading);
  const error = $derived(animeStore.error);
  const detailTab = $derived(animeStore.detailTab);
  const characters = $derived(animeStore.detailCharacters);
  const persons = $derived(animeStore.detailPersons);
  const comments = $derived(animeStore.detailComments);
  const imgCache = $derived(animeStore.imgCache);
  const name = $derived(animeStore.detailName);
  const ruleName = $derived(animeStore.detailRuleName);
  const collectType = $derived(animeStore.getCollectType(name));
  const historyEntry = $derived(animeStore.history.find((h) => h.name === name));
  const bangumiConnected = $derived(animeStore.bangumiConnected);
  const bangumiCollections = $derived(animeStore.bangumiCollections);
  const bangumiMatched = $derived(
    bangumiCollections.find(r => r.subject_name === name || r.subject_name_cn === name)
  );

  // ── Local UI state ────────────────────────────────────────────────────
  let showCollectMenu = $state(false);
  let activeRoad = $state(0);

  // Reset road when roads change
  $effect(() => { roads; activeRoad = 0; });

  // ── Derived values ────────────────────────────────────────────────────
  const posterUrl = $derived(subject?.image ? (imgCache[subject.image] || subject.image) : '');
  const bgPosterUrl = $derived(posterUrl);
  const ratingCount = $derived(rating?.count ?? []);
  const maxRatingCount = $derived(Math.max(...(ratingCount.slice(1) || [1]), 1));
  const currentEpisodes = $derived(roads[activeRoad]?.episodes ?? []);

  // ── Star rating helper ────────────────────────────────────────────────
  function starsArray(score: number): ('full' | 'half' | 'empty')[] {
    const normalized = score / 2; // Convert 10-point to 5-star
    const result: ('full' | 'half' | 'empty')[] = [];
    for (let i = 1; i <= 5; i++) {
      if (normalized >= i) result.push('full');
      else if (normalized >= i - 0.5) result.push('half');
      else result.push('empty');
    }
    return result;
  }

  // ── Collection handlers ───────────────────────────────────────────────
  function setCollect(type: number) {
    animeStore.setCollect(name, type);
    showCollectMenu = false;
  }

  function toggleCollectMenu() {
    showCollectMenu = !showCollectMenu;
  }

  // ── Tab switching with lazy loading ───────────────────────────────────
  function switchTab(tab: DetailTab) {
    animeStore.detailTab = tab;
    if (!subject) return;
    if (tab === 'characters' && characters.length === 0) {
      animeStore.loadBangumiCharacters(subject.id);
    }
    if (tab === 'staff' && persons.length === 0) {
      animeStore.loadBangumiPersons(subject.id);
    }
    if (tab === 'comments' && comments.length === 0) {
      animeStore.loadBangumiComments(subject.id);
    }
  }


  function handleDetailTabKeydown(event: KeyboardEvent, index: number) {
    const next = nextRovingIndex(event.key, index, DETAIL_TABS.length, "horizontal");
    if (next === null) return;
    event.preventDefault();
    switchTab(DETAIL_TABS[next].id);
    focusRovingItem(detailTabRefs, next);
  }

</script>

<DetailPanel
  open
  title={subject?.name_cn || subject?.name || name || "番剧详情"}
  description={ruleName ? `当前经典来源：${ruleName}` : "Bangumi 元数据详情"}
  onClose={() => animeStore.closeDetail()}
  side="right"
  size="lg"
  initialFocus=".fab-play"
  {returnFocus}
  class="anime-detail-panel"
>
  <div class="detail-overlay">
  <!-- Blurred poster background -->
  {#if bgPosterUrl}
    <BackgroundLayer src={bgPosterUrl} overlay={false} class="bg-blur" />
  {/if}

  <aside class="detail-visual" aria-label="番剧视觉摘要">
    <div class="detail-visual-media" class:no-poster={!posterUrl}>
      {#if posterUrl}<img src={posterUrl} alt="" />{:else}<span>{(subject?.name_cn || subject?.name || name).slice(0, 1)}</span>{/if}
    </div>
    <div class="detail-visual-scrim" aria-hidden="true"></div>
    <div class="detail-visual-register">
      <span>MOEPLAY / ANIME ARCHIVE</span>
      <strong>{ruleName || "BANGUMI METADATA"}</strong>
    </div>
    <div class="detail-visual-copy">
      <span>{historyEntry ? "CONTINUE WATCHING" : "READY TO WATCH"}</span>
      <h2>{subject?.name_cn || subject?.name || name}</h2>
      <p>{subject?.summary || "从封面、放送资料、评分与剧集线路进入这部作品。"}</p>
      <dl>
        {#if subject?.date}<div><dt>放送</dt><dd>{subject.date}</dd></div>{/if}
        {#if rating}<div><dt>评分</dt><dd>{rating.score.toFixed(1)}</dd></div>{/if}
        <div><dt>线路</dt><dd>{roads.length}</dd></div>
        <div><dt>进度</dt><dd>{historyEntry?.lastEpisodeName || (historyEntry ? `第 ${historyEntry.lastEpisode + 1} 集` : "未开始")}</dd></div>
      </dl>
    </div>
    <i aria-hidden="true"></i><i aria-hidden="true"></i><i aria-hidden="true"></i><i aria-hidden="true"></i>
  </aside>

  <!-- Main scrollable content -->
  <div class="detail-scroll">
    {#if loading && !subject}
      <div class="loading-center">
        <div class="spinner"></div>
        <span>加载中...</span>
      </div>
    {:else if error && !subject}
      <EmptyState icon="x" title="加载失败" description={error} class="error-center" />
    {:else}
      <!-- Title section -->
      <section class="title-section">
        <h1 class="detail-title">{subject?.name_cn || subject?.name || name}</h1>
        {#if subject?.name && subject.name !== subject.name_cn}
          <p class="detail-subtitle">{subject.name}</p>
        {/if}
      </section>

      <!-- Poster + metadata row -->
      <section class="info-section">
        <div class="poster-col">
          {#if posterUrl}
            <div class="poster-wrap">
              <img src={posterUrl} alt={name} class="poster-img" />
            </div>
          {/if}
          <!-- Collection button below poster -->
          <div class="collect-wrapper">
            <button
              class="collect-btn"
              class:collected={collectType > 0}
              onclick={toggleCollectMenu}
            >
              <Icon name={collectType > 0 ? "heartFill" : "heart"} size={14} />
              {COLLECT_TYPES[collectType] || "未追"}
              {#if bangumiConnected && bangumiMatched}
                <span class="bangumi-sync-icon" title="已同步至 Bangumi">
                  <Icon name="refresh" size={10} />
                </span>
              {/if}
            </button>
            {#if showCollectMenu}
              <div class="collect-menu">
                {#each [1, 2, 3, 4, 5] as t}
                  <button
                    class="collect-option"
                    class:active={collectType === t}
                    onclick={() => setCollect(t)}
                  >
                    {COLLECT_TYPES[t]}
                  </button>
                {/each}
                {#if collectType > 0}
                  <button class="collect-option remove" onclick={() => setCollect(0)}>
                    取消收藏
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <div class="meta-col">
          <!-- Air date -->
          {#if subject?.date}
            <div class="meta-row">
              <span class="meta-label">放送开始</span>
              <span class="meta-value accent">{subject.date}</span>
            </div>
          {/if}

          <!-- Rating with stars -->
          {#if rating}
            <div class="meta-row">
              <span class="meta-label">{rating.total}人评分</span>
              <div class="rating-display">
                <div class="stars">
                  {#each starsArray(rating.score) as starType}
                    <span class="star" class:full={starType === 'full'} class:half={starType === 'half'}>
                      <Icon name="star" size={14} />
                    </span>
                  {/each}
                </div>
                <span class="score-value">{rating.score.toFixed(1)}</span>
              </div>
            </div>
          {/if}

          <!-- Rank -->
          {#if subject?.rank && subject.rank > 0}
            <div class="meta-row">
              <span class="meta-label">Bangumi Ranked</span>
              <span class="meta-value accent">#{subject.rank}</span>
            </div>
          {/if}

          <!-- Rating distribution chart -->
          {#if rating && ratingCount.length >= 11}
            <div class="rating-chart">
              <span class="chart-title">评分分布</span>
              <div class="chart-bars">
                {#each [10, 9, 8, 7, 6, 5, 4, 3, 2, 1] as n}
                  <div class="chart-bar-col" title="{n} 分 · {ratingCount[n]} 人">
                    <div class="bar-track">
                      <div
                        class="bar-fill"
                        style="height: {ratingCount[n] > 0 ? Math.max((ratingCount[n] / maxRatingCount) * 100, 4) : 0}%"
                      ></div>
                    </div>
                    <span class="bar-label">{n}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </section>

      <!-- Tab navigation -->
      <div class="tab-bar" role="tablist" aria-label="番剧详情分类">
        {#each DETAIL_TABS as tab, index (tab.id)}
          <button
            bind:this={detailTabRefs[index]}
            type="button"
            role="tab"
            id={`anime-detail-tab-${tab.id}`}
            aria-selected={detailTab === tab.id}
            aria-controls={`anime-detail-panel-${tab.id}`}
            tabindex={detailTab === tab.id ? 0 : -1}
            class:active={detailTab === tab.id}
            onclick={() => switchTab(tab.id)}
            onkeydown={(event) => handleDetailTabKeydown(event, index)}
          >{tab.label}</button>
        {/each}
      </div>

      <!-- Tab content -->
      <div
        class="tab-content"
        id={`anime-detail-panel-${detailTab}`}
        role="tabpanel"
        aria-labelledby={`anime-detail-tab-${detailTab}`}
        tabindex="0"
      >
        {#if detailTab === 'overview'}
          <!-- Synopsis -->
          {#if subject?.summary}
            <div class="content-block">
              <h3 class="block-title">简介</h3>
              <p class="synopsis">{subject.summary}</p>
            </div>
          {/if}

          <!-- Tags -->
          {#if subject?.tags && subject.tags.length > 0}
            <Card padding="md" class="content-block">
              <h3 class="block-title">标签</h3>
              <div class="tags-wrap">
                {#each subject.tags as tag}
                  <Tag variant="accent" size="sm">
                    {tag.name}
                    {#if tag.count > 0}
                      <span class="tag-count">{tag.count}</span>
                    {/if}
                  </Tag>
                {/each}
              </div>
            </Card>
          {/if}

          <!-- 线路/分集已移至 SourceSheet (点击"开始观看"后弹出) -->


        {:else if detailTab === 'comments'}
          <div class="comments-list">
            {#if comments.length === 0}
              <EmptyState title="暂无吐槽" class="empty-text" />
            {:else}
              {#each comments as c}
                <Card padding="md" class="comment-card">
                  <div class="comment-header">
                    <span class="comment-user">{c.user}</span>
                    {#if c.rate > 0}
                      <span class="comment-rate">{c.rate}/10</span>
                    {/if}
                    <span class="comment-date">{c.date}</span>
                  </div>
                  <p class="comment-text">{c.comment}</p>
                </Card>
              {/each}
            {/if}
          </div>

        {:else if detailTab === 'characters'}
          <div class="characters-list">
            {#if characters.length === 0}
              <EmptyState title="暂无角色信息" class="empty-text" />
            {:else}
              {#each characters as ch}
                <Card padding="sm" class="character-card">
                  {#if ch.image}
                    <img
                      src={imgCache[ch.image] || ch.image}
                      alt={ch.name}
                      class="char-img"
                    />
                  {:else}
                    <div class="char-img-placeholder">
                      <Icon name="user" size={20} />
                    </div>
                  {/if}
                  <div class="char-info">
                    <span class="char-name">{ch.name_cn || ch.name}</span>
                    {#if ch.actors.length > 0}
                      <span class="char-actor">CV: {ch.actors.map(a => a.name_cn || a.name).join(', ')}</span>
                    {/if}
                  </div>
                </Card>
              {/each}
            {/if}
          </div>

        {:else if detailTab === 'staff'}
          <div class="staff-list">
            {#if persons.length === 0}
              <EmptyState title="暂无制作人员信息" class="empty-text" />
            {:else}
              {#each persons as p}
                <Card padding="sm" class="staff-card">
                  {#if p.image}
                    <img
                      src={imgCache[p.image] || p.image}
                      alt={p.name}
                      class="staff-img"
                    />
                  {:else}
                    <div class="staff-img-placeholder">
                      <Icon name="user" size={20} />
                    </div>
                  {/if}
                  <div class="staff-info">
                    <span class="staff-name">{p.name_cn || p.name}</span>
                    <span class="staff-job">{p.jobs.join(' / ')}</span>
                  </div>
                </Card>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Floating play button → opens SourceSheet（常驻，任意 tab 都能开始观看） -->
  <button
    class="fab-play"
    type="button"
    data-autofocus
    data-anime-source-trigger
    aria-haspopup="dialog"
    aria-expanded={animeStore.sourceSheetOpen}
    onclick={() => animeStore.openSourceSheet()}
  >
    <span class="fab-glow"></span>
    <Icon name="play" size={20} />
    {#if historyEntry}
      <span>继续 · {historyEntry.lastEpisodeName || `第${historyEntry.lastEpisode + 1}集`}</span>
    {:else}
      <span>开始观看</span>
    {/if}
  </button>
  </div>
</DetailPanel>

<style>
  :global(.v2-detail-panel.anime-detail-panel) {
    right: 0;
    left: 0;
    width: 100vw;
    max-width: none;
    background: #0d1117;
  }
  :global(.v2-detail-panel.anime-detail-panel .v2-detail-panel__body) { padding: 0; min-height: 0; overflow: hidden; }
  .tab-bar { display: flex; gap: .35rem; flex-wrap: wrap; padding: .25rem; border: 1px solid rgba(255,255,255,.08); border-radius: .8rem; background: rgba(255,255,255,.025); }
  .tab-bar button { min-height: 2.6rem; padding: .55rem .9rem; border: 0; border-radius: .6rem; background: transparent; color: #9aa4b2; font: inherit; font-weight: 650; cursor: pointer; }
  .tab-bar button.active { background: rgba(232,85,127,.14); color: #fff; box-shadow: inset 0 0 0 1px rgba(232,85,127,.28); }
  .tab-bar button:focus-visible { outline: 2px solid #e8557f; outline-offset: 2px; }

  /* ── Overlay container ──────────────────────────────────────────────── */
  .detail-overlay {
    position: absolute;
    inset: 0;
    background: #0d1117;
    z-index: 20;
    display: grid;
    grid-template-columns: minmax(300px, 42vw) minmax(0, 1fr);
    overflow: hidden;
    animation: slide-in 0.25s ease-out;
  }

  @keyframes slide-in {
    from { transform: translateX(40px); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
  }

  /* ── Blurred background ─────────────────────────────────────────────── */
  :global(.ui-bg-layer.bg-blur) {
    position: absolute;
    inset: -60px;
    background-size: cover;
    background-position: center;
    filter: blur(80px) brightness(0.25);
    z-index: 0;
    pointer-events: none;
  }

  /* ── Header ─────────────────────────────────────────────────────────── */

  :global(.ui-button.header-btn) {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid rgba(232,85,127,0.4);
    border-radius: 50%;
    background: rgba(0,0,0,0.5);
    color: #fff;
    cursor: pointer;
    transition: all 0.15s ease;
    backdrop-filter: blur(8px);
  }

  :global(.ui-button.header-btn:hover) {
    background: rgba(232,85,127,0.3);
    border-color: #e8557f;
    transform: scale(1.05);
  }




  .detail-visual {
    position: relative;
    z-index: 2;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    border-right: 1px solid rgba(255,255,255,.13);
    background: linear-gradient(145deg, #171c24, #090b0f);
    isolation: isolate;
  }
  .detail-visual-media { position:absolute; inset:0; overflow:hidden; }
  .detail-visual-media img { width:100%; height:100%; object-fit:cover; object-position:center 28%; filter:saturate(.84) contrast(1.04); transform:scale(1.015); }
  .detail-visual-media.no-poster { display:grid; place-items:center; background:radial-gradient(circle at 35% 28%,rgba(232,85,127,.24),transparent 34%),linear-gradient(145deg,#202733,#090b0f); }
  .detail-visual-media.no-poster span { color:rgba(255,255,255,.09); font:700 clamp(14rem,32vw,34rem)/1 var(--font-display); }
  .detail-visual-scrim { position:absolute; inset:0; background:linear-gradient(180deg,rgba(3,5,8,.08) 15%,rgba(3,5,8,.28) 48%,rgba(3,5,8,.94) 100%),linear-gradient(90deg,transparent 58%,rgba(3,5,8,.42)); pointer-events:none; }
  .detail-visual-register { position:absolute; z-index:3; top:0; right:0; left:0; display:flex; justify-content:space-between; gap:12px; padding:16px 18px; border-bottom:1px solid rgba(255,255,255,.22); color:rgba(255,255,255,.72); font:650 8px/1 var(--font-mono); letter-spacing:.12em; }
  .detail-visual-register strong { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .detail-visual-copy { position:absolute; z-index:3; right:clamp(24px,4vw,60px); bottom:clamp(26px,5vh,68px); left:clamp(24px,4vw,60px); }
  .detail-visual-copy>span { color:#e8557f; font:700 8px/1 var(--font-mono); letter-spacing:.14em; }
  .detail-visual-copy h2 { max-width:11ch; margin:14px 0 12px; color:#fff; font:680 clamp(2.5rem,5.3vw,6.8rem)/.8 var(--font-display); letter-spacing:-.07em; text-wrap:balance; }
  .detail-visual-copy>p { display:-webkit-box; max-width:50ch; margin:0 0 20px; overflow:hidden; color:rgba(255,255,255,.66); font:500 12px/1.55 var(--font-ui); line-clamp:3; -webkit-line-clamp:3; -webkit-box-orient:vertical; }
  .detail-visual-copy dl { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); margin:0; border-top:1px solid rgba(255,255,255,.24); }
  .detail-visual-copy dl div { min-width:0; padding:10px 10px 9px 0; border-bottom:1px solid rgba(255,255,255,.16); }
  .detail-visual-copy dt { color:rgba(255,255,255,.42); font:650 7px/1 var(--font-mono); letter-spacing:.12em; }
  .detail-visual-copy dd { margin:6px 0 0; overflow:hidden; color:#fff; font:650 10px/1 var(--font-ui); text-overflow:ellipsis; white-space:nowrap; }
  .detail-visual>i { position:absolute; z-index:4; width:16px; height:16px; pointer-events:none; }
  .detail-visual>i:nth-of-type(1) { top:58px; left:14px; border-top:1px solid #e8557f; border-left:1px solid #e8557f; }
  .detail-visual>i:nth-of-type(2) { top:58px; right:14px; border-top:1px solid #e8557f; border-right:1px solid #e8557f; }
  .detail-visual>i:nth-of-type(3) { bottom:14px; left:14px; border-bottom:1px solid #e8557f; border-left:1px solid #e8557f; }
  .detail-visual>i:nth-of-type(4) { right:14px; bottom:14px; border-right:1px solid #e8557f; border-bottom:1px solid #e8557f; }


  /* ── Scrollable content ─────────────────────────────────────────────── */
  .detail-scroll {
    position: relative;
    z-index: 2;
    flex: 1;
    overflow-y: auto;
    padding: 24px clamp(24px, 3.2vw, 54px) 120px;
    background: linear-gradient(180deg, rgba(13,17,23,.94), rgba(10,13,18,.98));
    scrollbar-width: thin;
    scrollbar-color: rgba(232,85,127,0.3) transparent;
  }

  .detail-scroll::-webkit-scrollbar {
    width: 6px;
  }

  .detail-scroll::-webkit-scrollbar-track {
    background: transparent;
  }

  .detail-scroll::-webkit-scrollbar-thumb {
    background: rgba(232,85,127,0.3);
    border-radius: 3px;
  }

  /* ── Title section ──────────────────────────────────────────────────── */
  .title-section {
    margin-bottom: 20px;
  }

  .detail-title {
    font-size: 28px;
    font-weight: 800;
    color: #ffffff;
    margin: 8px 0 0;
    line-height: 1.35;
    letter-spacing: -0.02em;
  }

  .detail-subtitle {
    font-size: 14px;
    color: #8b949e;
    margin: 6px 0 0;
    line-height: 1.4;
  }

  /* ── Info section (poster + metadata) ───────────────────────────────── */
  .info-section {
    display: flex;
    gap: 20px;
    margin-bottom: 24px;
  }

  .poster-col {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }

  .poster-wrap {
    width: 160px;
    aspect-ratio: 3/4;
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0,0,0,0.6);
    position: relative;
  }

  .poster-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  /* ── Collection button ──────────────────────────────────────────────── */
  .collect-wrapper {
    position: relative;
    width: 100%;
  }

  .collect-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 16px;
    border: 1px solid #e8557f;
    border-radius: 24px;
    background: transparent;
    color: #e8557f;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .collect-btn:hover {
    background: rgba(232,85,127,0.15);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(232,85,127,0.3);
  }

  .collect-btn.collected {
    background: linear-gradient(135deg, #e8557f, #c7446a);
    color: #ffffff;
    border-color: transparent;
  }

  .collect-btn.collected:hover {
    background: linear-gradient(135deg, #f06b95, #d55a82);
    box-shadow: 0 4px 16px rgba(232,85,127,0.5);
  }

  .bangumi-sync-icon {
    display: inline-flex;
    align-items: center;
    color: #58d68d;
    opacity: 0.8;
    margin-left: 2px;
  }

  /* ── Collect dropdown menu ──────────────────────────────────────────── */
  .collect-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    right: 0;
    background: #161b22;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 12px;
    padding: 6px;
    z-index: 20;
    box-shadow: 0 12px 40px rgba(0,0,0,0.7);
    animation: menu-appear 0.15s ease-out;
  }

  @keyframes menu-appear {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .collect-option {
    display: block;
    width: 100%;
    padding: 10px 14px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #8b949e;
    font-size: 14px;
    cursor: pointer;
    text-align: center;
    transition: all 0.12s ease;
  }

  .collect-option:hover {
    background: rgba(255,255,255,0.06);
    color: #ffffff;
  }

  .collect-option.active {
    color: #e8557f;
    font-weight: 600;
    background: rgba(232,85,127,0.1);
  }

  .collect-option.remove {
    color: #f87171;
    border-top: 1px solid rgba(255,255,255,0.08);
    margin-top: 4px;
    padding-top: 12px;
  }

  /* ── Metadata column ────────────────────────────────────────────────── */
  .meta-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }

  .meta-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .meta-label {
    font-size: 12px;
    color: #6e7681;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .meta-value {
    font-size: 16px;
    color: #ffffff;
    font-weight: 600;
  }

  .meta-value.accent {
    color: #e8557f;
  }

  /* ── Stars rating ───────────────────────────────────────────────────── */
  .rating-display {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .stars {
    display: flex;
    gap: 2px;
  }

  .star {
    color: #2d333b;
    position: relative;
  }

  .star.full {
    color: #fbbf24;
  }

  .star.half {
    color: #2d333b;
    background: linear-gradient(90deg, #fbbf24 50%, #2d333b 50%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .score-value {
    font-size: 22px;
    font-weight: 700;
    color: #ffffff;
  }

  /* ── Rating chart ───────────────────────────────────────────────────── */
  .rating-chart {
    margin-top: 4px;
    max-width: 320px;
  }

  .chart-title {
    font-size: 12px;
    color: #6e7681;
    font-weight: 500;
    margin-bottom: 10px;
    display: block;
  }

  .chart-bars {
    display: flex;
    align-items: flex-end;
    gap: 5px;
    height: 52px;
  }

  .chart-bar-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    height: 100%;
    cursor: default;
  }

  .bar-track {
    flex: 1;
    width: 100%;
    background: rgba(255,255,255,0.04);
    border-radius: 3px;
    overflow: hidden;
    display: flex;
    align-items: flex-end;
  }

  .bar-fill {
    width: 100%;
    background: linear-gradient(180deg, #e8557f, #c7446a);
    border-radius: 3px 3px 0 0;
    transition: height 0.4s ease, background 0.15s ease;
  }

  .chart-bar-col:hover .bar-fill {
    background: linear-gradient(180deg, #f06b95, #e8557f);
  }

  .bar-label {
    font-size: 10px;
    color: #6e7681;
    font-weight: 500;
  }

  /* ── Tab navigation ─────────────────────────────────────────────────── */
  :global(.ui-segment.tab-bar) {
    margin-bottom: 20px;
    overflow-x: auto;
    scrollbar-width: none;
  }

  :global(.ui-segment.tab-bar::-webkit-scrollbar) {
    display: none;
  }

  /* ── Tab content ────────────────────────────────────────────────────── */
  .tab-content {
    min-height: 200px;
    animation: tab-enter 0.2s ease;
  }

  /* ── Overview content ───────────────────────────────────────────────── */
  .content-block {
    margin-bottom: 24px;
  }

  .block-title {
    font-size: 16px;
    font-weight: 700;
    color: #ffffff;
    margin: 0 0 12px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .block-title::before {
    content: '';
    display: block;
    width: 3px;
    height: 16px;
    background: #e8557f;
    border-radius: 2px;
  }

  .synopsis {
    font-size: 14px;
    line-height: 1.8;
    color: #8b949e;
    margin: 0;
  }

  .tags-wrap {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .tag-count {
    font-size: 11px;
    color: #6e7681;
    font-weight: 400;
  }

  /* ── Comments ───────────────────────────────────────────────────────── */
  .comments-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  :global(.ui-card.comment-card:hover) {
    background: rgba(255,255,255,0.05);
    border-color: rgba(255,255,255,0.1);
  }

  .comment-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }

  .comment-user {
    font-size: 14px;
    color: #e8557f;
    font-weight: 600;
  }

  .comment-rate {
    font-size: 12px;
    color: #fbbf24;
    background: rgba(251,191,36,0.1);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .comment-date {
    font-size: 12px;
    color: #6e7681;
    margin-left: auto;
  }

  .comment-text {
    font-size: 14px;
    color: #8b949e;
    margin: 0;
    line-height: 1.7;
  }

  /* ── Characters & Staff ─────────────────────────────────────────────── */
  .characters-list,
  .staff-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  :global(.ui-card.character-card),
  :global(.ui-card.staff-card) {
    display: flex;
    gap: 14px;
    align-items: center;
  }

  :global(.ui-card.character-card:hover),
  :global(.ui-card.staff-card:hover) {
    background: rgba(255,255,255,0.05);
    border-color: rgba(255,255,255,0.1);
    transform: translateX(4px);
  }

  .char-img,
  .staff-img {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    border: 2px solid rgba(255,255,255,0.1);
  }

  .char-img-placeholder,
  .staff-img-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: rgba(255,255,255,0.06);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #6e7681;
    flex-shrink: 0;
  }

  .char-info,
  .staff-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .char-name,
  .staff-name {
    font-size: 14px;
    color: #ffffff;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .char-actor,
  .staff-job {
    font-size: 12px;
    color: #6e7681;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.ui-empty.empty-text) {
    color: #6e7681;
    font-size: 14px;
    text-align: center;
    padding: 60px 0;
    animation: fade-in 0.3s ease;
  }

  @keyframes tab-enter {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  /* ── Floating action button ─────────────────────────────────────────── */
  .fab-play {
    position: absolute;
    bottom: max(6.5rem, env(safe-area-inset-bottom));
    right: 28px;
    z-index: 15;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 32px;
    border: none;
    border-radius: 30px;
    background: linear-gradient(135deg, #c7446a, #e8557f);
    color: #ffffff;
    font-size: 15px;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 8px 32px rgba(232,85,127,0.5);
    transition: all 0.25s ease;
    animation: fab-appear 0.3s ease-out 0.2s backwards;
    overflow: hidden;
    max-width: min(320px, 70vw);
  }
  .fab-play span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fab-glow {
    position: absolute;
    inset: -2px;
    border-radius: inherit;
    background: linear-gradient(135deg, rgba(232,85,127,0.6), rgba(199,68,106,0.3));
    filter: blur(10px);
    animation: fab-breathe 2.5s ease-in-out infinite;
    pointer-events: none;
    z-index: -1;
  }

  @keyframes fab-appear {
    from { opacity: 0; transform: translateY(20px) scale(0.9); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }

  @keyframes fab-breathe {
    0%, 100% { opacity: 0.4; transform: scale(1); }
    50% { opacity: 0.8; transform: scale(1.08); }
  }

  .fab-play:hover {
    transform: translateY(-3px);
    box-shadow: 0 12px 40px rgba(232,85,127,0.7);
    background: linear-gradient(135deg, #d55a82, #f06b95);
  }

  .fab-play:active {
    transform: translateY(-1px) scale(0.97);
    box-shadow: 0 6px 24px rgba(232,85,127,0.5);
  }

  /* ── Loading & Error states ─────────────────────────────────────────── */
  .loading-center,
  :global(.ui-empty.error-center) {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    color: #6e7681;
    min-height: 300px;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid rgba(255,255,255,0.1);
    border-top-color: #e8557f;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  :global(.ui-empty.error-center p) {
    font-size: 14px;
    margin: 0;
    text-align: center;
    max-width: 300px;
  }


  @media (max-width: 900px) {
    :global(.v2-detail-panel.anime-detail-panel .v2-detail-panel__header) { padding-inline: 16px; }
    .detail-overlay { grid-template-columns: minmax(220px, 34vw) minmax(0,1fr); }
    .detail-visual-copy { right:22px; bottom:28px; left:22px; }
    .detail-visual-copy h2 { font-size:clamp(2rem,5vw,3.8rem); }
    .detail-visual-copy>p { -webkit-line-clamp:2; line-clamp:2; }
  }
  @media (max-width: 680px) {
    .detail-overlay { grid-template-columns:1fr; grid-template-rows:220px minmax(0,1fr); }
    .detail-visual { border-right:0; border-bottom:1px solid rgba(255,255,255,.13); }
    .detail-visual-media img { object-position:center 24%; }
    .detail-visual-copy { bottom:18px; }
    .detail-visual-copy h2 { max-width:16ch; margin:8px 0 0; font-size:clamp(1.7rem,8vw,2.7rem); }
    .detail-visual-copy>p,.detail-visual-copy dl { display:none; }
    .detail-visual-register { padding:10px 12px; }
    .detail-scroll { padding:18px 16px 110px; }
  }

  @media (prefers-reduced-motion: reduce) {
    .detail-overlay, .detail-overlay *, .fab-play, .fab-glow { animation: none !important; transition: none !important; scroll-behavior: auto !important; }
    .fab-play:hover { transform: none; }
  }
  :global([data-motion="reduce"]) .detail-overlay,
  :global([data-motion="reduce"]) .detail-overlay *,
  :global([data-motion="reduce"]) .fab-play,
  :global([data-motion="reduce"]) .fab-glow { animation: none !important; transition: none !important; scroll-behavior: auto !important; }
</style>
