<script lang="ts">
  import { animeStore, COLLECT_TYPES } from '../../stores/anime.svelte';
  import Icon from '../Icon.svelte';

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
  function switchTab(tab: 'overview' | 'comments' | 'characters' | 'staff') {
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

  // ── Close dropdown on outside click ───────────────────────────────────
  function handleOverlayClick(e: MouseEvent) {
    if (showCollectMenu) {
      const target = e.target as HTMLElement;
      if (!target.closest('.collect-wrapper')) {
        showCollectMenu = false;
      }
    }
  }
</script>

<div class="detail-overlay" role="dialog" tabindex="-1" onclick={handleOverlayClick} onkeydown={(e) => e.key === 'Escape' && animeStore.closeDetail()}>
  <!-- Blurred poster background -->
  {#if bgPosterUrl}
    <div class="bg-blur" style="background-image: url('{bgPosterUrl}')"></div>
  {/if}

  <!-- Top header -->
  <header class="detail-header">
    <button class="header-btn" onclick={() => animeStore.closeDetail()} aria-label="返回">
      <Icon name="chevronLeft" size={20} />
    </button>
    <div class="header-center">
      {#if ruleName}
        <span class="source-badge">{ruleName}</span>
      {/if}
    </div>
    <div class="header-right"></div>
  </header>

  <!-- Main scrollable content -->
  <main class="detail-scroll">
    {#if loading && !subject}
      <div class="loading-center">
        <div class="spinner"></div>
        <span>加载中...</span>
      </div>
    {:else if error && !subject}
      <div class="error-center">
        <Icon name="x" size={32} />
        <p>{error}</p>
      </div>
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
      <nav class="tab-bar">
        {#each [
          ['overview', '概览'],
          ['comments', '吐槽'],
          ['characters', '角色'],
          ['staff', '制作人员']
        ] as [tab, label] (tab)}
          <button
            class="tab-btn"
            class:active={detailTab === tab}
            onclick={() => switchTab(tab as 'overview' | 'comments' | 'characters' | 'staff')}
          >
            {label}
            {#if detailTab === tab}
              <span class="tab-indicator"></span>
            {/if}
          </button>
        {/each}
      </nav>

      <!-- Tab content -->
      <div class="tab-content">
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
            <div class="content-block">
              <h3 class="block-title">标签</h3>
              <div class="tags-wrap">
                {#each subject.tags as tag}
                  <span class="tag-pill">
                    {tag.name}
                    {#if tag.count > 0}
                      <span class="tag-count">{tag.count}</span>
                    {/if}
                  </span>
                {/each}
              </div>
            </div>
          {/if}

          <!-- 线路/分集已移至 SourceSheet (点击"开始观看"后弹出) -->

        {:else if detailTab === 'comments'}
          <div class="comments-list">
            {#if comments.length === 0}
              <p class="empty-text">暂无吐槽</p>
            {:else}
              {#each comments as c}
                <div class="comment-card">
                  <div class="comment-header">
                    <span class="comment-user">{c.user}</span>
                    {#if c.rate > 0}
                      <span class="comment-rate">{c.rate}/10</span>
                    {/if}
                    <span class="comment-date">{c.date}</span>
                  </div>
                  <p class="comment-text">{c.comment}</p>
                </div>
              {/each}
            {/if}
          </div>

        {:else if detailTab === 'characters'}
          <div class="characters-list">
            {#if characters.length === 0}
              <p class="empty-text">暂无角色信息</p>
            {:else}
              {#each characters as ch}
                <div class="character-card">
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
                </div>
              {/each}
            {/if}
          </div>

        {:else if detailTab === 'staff'}
          <div class="staff-list">
            {#if persons.length === 0}
              <p class="empty-text">暂无制作人员信息</p>
            {:else}
              {#each persons as p}
                <div class="staff-card">
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
                </div>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </main>

  <!-- Floating play button → opens SourceSheet（常驻，任意 tab 都能开始观看） -->
  <button
    class="fab-play"
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

<style>
  /* ── Overlay container ──────────────────────────────────────────────── */
  .detail-overlay {
    position: absolute;
    inset: 0;
    background: #0d1117;
    z-index: 20;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: slide-in 0.25s ease-out;
  }

  @keyframes slide-in {
    from { transform: translateX(40px); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
  }

  /* ── Blurred background ─────────────────────────────────────────────── */
  .bg-blur {
    position: absolute;
    inset: -60px;
    background-size: cover;
    background-position: center;
    filter: blur(80px) brightness(0.25);
    z-index: 0;
    pointer-events: none;
  }

  /* ── Header ─────────────────────────────────────────────────────────── */
  .detail-header {
    position: relative;
    z-index: 10;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: linear-gradient(180deg, rgba(13,17,23,0.95) 0%, rgba(13,17,23,0.7) 70%, transparent 100%);
  }

  .header-btn {
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

  .header-btn:hover {
    background: rgba(232,85,127,0.3);
    border-color: #e8557f;
    transform: scale(1.05);
  }

  .header-center {
    flex: 1;
    display: flex;
    justify-content: center;
  }

  .source-badge {
    padding: 4px 12px;
    background: rgba(232,85,127,0.15);
    border: 1px solid rgba(232,85,127,0.3);
    border-radius: 20px;
    color: #e8557f;
    font-size: 12px;
    font-weight: 500;
    backdrop-filter: blur(8px);
  }

  .header-right {
    display: flex;
    gap: 8px;
  }


  /* ── Scrollable content ─────────────────────────────────────────────── */
  .detail-scroll {
    position: relative;
    z-index: 2;
    flex: 1;
    overflow-y: auto;
    padding: 0 20px 120px;
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
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid rgba(255,255,255,0.08);
    margin-bottom: 20px;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .tab-bar::-webkit-scrollbar {
    display: none;
  }

  .tab-btn {
    position: relative;
    padding: 12px 18px;
    border: none;
    background: transparent;
    color: #6e7681;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tab-btn:hover {
    color: #8b949e;
  }

  .tab-btn.active {
    color: #e8557f;
    font-weight: 600;
  }

  .tab-indicator {
    position: absolute;
    bottom: -1px;
    left: 18px;
    right: 18px;
    height: 2px;
    background: #e8557f;
    border-radius: 1px;
    animation: indicator-appear 0.2s ease-out;
  }

  @keyframes indicator-appear {
    from { transform: scaleX(0); }
    to   { transform: scaleX(1); }
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

  .tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border: 1px solid rgba(232,85,127,0.3);
    border-radius: 20px;
    background: rgba(232,85,127,0.06);
    color: #e8557f;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.15s ease;
    cursor: default;
  }

  .tag-pill:hover {
    background: rgba(232,85,127,0.15);
    border-color: rgba(232,85,127,0.5);
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

  .comment-card {
    padding: 16px;
    background: rgba(255,255,255,0.03);
    border-radius: 12px;
    border: 1px solid rgba(255,255,255,0.06);
    transition: all 0.15s ease;
  }

  .comment-card:hover {
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

  .character-card,
  .staff-card {
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 12px;
    background: rgba(255,255,255,0.03);
    border-radius: 12px;
    border: 1px solid rgba(255,255,255,0.06);
    transition: all 0.15s ease;
  }

  .character-card:hover,
  .staff-card:hover {
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

  .empty-text {
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
    bottom: 28px;
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
  .error-center {
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

  .error-center p {
    font-size: 14px;
    margin: 0;
    text-align: center;
    max-width: 300px;
  }
</style>
