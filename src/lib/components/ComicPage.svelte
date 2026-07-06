<script lang="ts">
  import { onMount } from "svelte";
  import { comicStore, SORT_OPTIONS } from "../stores/comic.svelte";
  import ComicCard from "./comic/ComicCard.svelte";
  import ComicDetail from "./comic/ComicDetail.svelte";
  import ComicReader from "./comic/ComicReader.svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, Input, LoadingSkeleton, SearchInput, SegmentControl, Tag } from "./ui";

  let loginEmail = $state(comicStore.savedEmail);
  let loginPassword = $state("");
  let loginError = $state("");

  async function handleLogin(e: Event) {
    e.preventDefault();
    if (!loginEmail || !loginPassword) { loginError = "请填写账号和密码"; return; }
    loginError = "";
    try {
      await comicStore.login(loginEmail, loginPassword);
      if (comicStore.isLoggedIn) {
        comicStore.loadProfile();
        await comicStore.loadCategories();
      }
    } catch {
      loginError = comicStore.error ?? "登录失败";
    }
  }

  // 搜索
  let searchInput = $state("");

  async function handleSearch(value: string) {
    if (!value.trim()) return;
    await comicStore.search(value.trim());
  }

  function clearSearch() {
    searchInput = "";
    comicStore.searchKeyword = "";
    comicStore.setTab("explore");
  }

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
    return target.isContentEditable;
  }

  function onKeydown(e: KeyboardEvent) {
    if (isTypingTarget(e.target)) return;
    if (e.key === "Escape") {
      if (comicStore.view === "reader") {
        e.stopImmediatePropagation();
        comicStore.closeReader();
      } else if (comicStore.view === "detail") {
        e.stopImmediatePropagation();
        comicStore.closeComic();
      }
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeydown, { capture: true });
    comicStore.rehydrate().then(() => {
      if (comicStore.isLoggedIn) {
        comicStore.loadProfile();
        comicStore.loadCategories();
      }
    });
    return () => window.removeEventListener("keydown", onKeydown, { capture: true });
  });

  const tabs = [
    { value: "explore",   label: "探索" },
    { value: "ranking",   label: "排行榜" },
    { value: "random",    label: "随机" },
    { value: "favorites", label: "收藏" },
    { value: "history",   label: "历史" },
  ];

  let punchingIn = $state(false);
  async function handlePunchIn() {
    punchingIn = true;
    await comicStore.punchIn();
    punchingIn = false;
  }

  function fmtDate(ts: number) {
    const d = new Date(ts);
    const mm = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    const hh = String(d.getHours()).padStart(2, "0");
    const mi = String(d.getMinutes()).padStart(2, "0");
    return `${mm}-${dd} ${hh}:${mi}`;
  }
</script>

<section class="comic-page" data-testid="comic-page">
  {#if !comicStore.isLoggedIn}
    <!-- ── 登录 ── -->
    <div class="login-gate">
      <Card class="login-card" padding="lg">
        <div class="login-logo"><Icon name="book" size={36} /></div>
        <h1 class="login-title">哔咔漫画</h1>
        <p class="login-sub">使用你的哔咔账号登录，畅读海量漫画</p>
        <form class="login-form" onsubmit={handleLogin}>
          <div class="field">
            <label for="email">邮箱 / 用户名</label>
            <Input id="email" type="text" bind:value={loginEmail}
              placeholder="邮箱或用户名" autocomplete="username" disabled={comicStore.loading} />
          </div>
          <div class="field">
            <label for="pwd">密码</label>
            <Input id="pwd" type="password" bind:value={loginPassword}
              placeholder="••••••••" autocomplete="current-password" disabled={comicStore.loading} />
          </div>
          {#if loginError}<p class="login-error">{loginError}</p>{/if}
          <Button type="submit" fullWidth loading={comicStore.loading}>
            登录
          </Button>
        </form>
        <p class="login-note">账号为你的哔咔注册账号，萌游不存储密码。</p>
      </Card>
    </div>

  {:else}
    <!-- ── 已登录主界面 ── -->
    <div class="comic-shell" class:hidden-by-overlay={comicStore.view !== "home"}>
      <header class="comic-header">
        <div class="header-left">
          <span class="header-kicker">Comic</span>
          <h1 class="header-title"><Icon name="book" size={20} /> 漫画</h1>
        </div>

        <form class="search-form" onsubmit={(e) => { e.preventDefault(); handleSearch(searchInput); }}>
          <SearchInput class="search-wrap" bind:value={searchInput}
            placeholder="搜索漫画..." onclear={clearSearch} />
          <Button type="submit" variant="secondary" disabled={!searchInput.trim()}>搜索</Button>
        </form>

        <!-- 用户信息 -->
        <div class="user-area">
          {#if comicStore.profile}
            <Tag variant="muted" size="md" class="user-chip" title={comicStore.profile.slogan || comicStore.profile.name}>
              <span class="user-level">Lv.{comicStore.profile.level}</span>
              <span class="user-name">{comicStore.profile.name}</span>
            </Tag>
            {#if !comicStore.profile.is_punched}
              <Button variant="ghost" size="sm" class="punch-btn" onclick={handlePunchIn} disabled={punchingIn}
                title="每日打卡" ariaLabel="每日打卡">
                <Icon name="zap" size={13} />
              </Button>
            {:else}
              <Tag variant="accent" size="sm" class="punched-badge" title="今日已打卡"><Icon name="check" size={13} /></Tag>
            {/if}
          {/if}
          <Button variant="ghost" size="sm" class="logout-btn" onclick={() => comicStore.logout()} title="退出登录" ariaLabel="退出登录">
            <Icon name="x" size={14} />
          </Button>
        </div>
      </header>

      <!-- Tab Bar -->
      <div class="tab-bar">
        {#if !comicStore.searchKeyword}
          <SegmentControl options={tabs} value={comicStore.activeTab} onChange={(v) => comicStore.setTab(v as any)} size="sm" />

          <!-- 排序（仅探索/搜索/收藏） -->
          {#if comicStore.activeTab === "explore" || comicStore.activeTab === "favorites"}
            <div class="sort-area">
              <select class="sort-select" value={comicStore.sort}
                onchange={(e) => comicStore.setSort((e.currentTarget as HTMLSelectElement).value as any)}>
                {#each SORT_OPTIONS as opt (opt.value)}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </div>
          {/if}
        {:else}
          <span class="search-label">搜索："{comicStore.searchKeyword}"</span>
          <div class="sort-area">
            <select class="sort-select" value={comicStore.sort}
              onchange={(e) => { comicStore.setSort((e.currentTarget as HTMLSelectElement).value as any); comicStore.search(comicStore.searchKeyword); }}>
              {#each SORT_OPTIONS as opt (opt.value)}
                <option value={opt.value}>{opt.label}</option>
              {/each}
            </select>
          </div>
          <Button variant="ghost" size="sm" onclick={clearSearch}>清除</Button>
        {/if}
      </div>

      <!-- Content -->
      <div class="comic-content">
        {#if comicStore.error}
          <EmptyState class="content-error" icon="x" title={comicStore.error}
            action={{ label: "重试", onclick: () => { comicStore.clearError(); comicStore.loadCategories(); } }} />
        {:else if comicStore.loading && comicStore.comicList.length === 0 && comicStore.searchResults.length === 0 && comicStore.ranking.length === 0 && comicStore.randomList.length === 0 && comicStore.favorites.length === 0}
          <div class="content-loading">
            <LoadingSkeleton rows={6} columns={4} />
            <span class="loading-hint">若长时间无响应，请检查网络或代理设置</span>
          </div>

        <!-- 搜索结果 -->
        {:else if comicStore.searchKeyword && comicStore.searchResults.length > 0}
          <div class="comics-grid">
            {#each comicStore.searchResults as comic (comic.id)}
              <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
            {/each}
          </div>
          {#if comicStore.searchPage < comicStore.searchPages}
            <Button variant="ghost" class="load-more" onclick={() => comicStore.searchNextPage()}
              disabled={comicStore.loading} loading={comicStore.loading}>
              加载更多
            </Button>
          {/if}
        {:else if comicStore.searchKeyword && !comicStore.loading}
          <EmptyState icon="search" title="没有找到相关漫画" />

        <!-- 探索 -->
        {:else if comicStore.activeTab === "explore"}
          {#if comicStore.categories.length > 0}
            <div class="cat-chips">
              <Tag active={comicStore.selectedCategory === null}
                onclick={() => comicStore.selectCategory(null)}>全部</Tag>
              {#each comicStore.categories as cat (cat.id || cat.title)}
                <Tag active={comicStore.selectedCategory === cat.title}
                  onclick={() => comicStore.selectCategory(cat.title)}>
                  {cat.title}
                </Tag>
              {/each}
            </div>
          {/if}
          <div class="comics-grid">
            {#each comicStore.comicList as comic (comic.id)}
              <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
            {/each}
          </div>
          {#if comicStore.comicList.length === 0 && !comicStore.loading}
            <EmptyState icon="book" title="暂无漫画" />
          {/if}
          {#if comicStore.comicPage < comicStore.comicPages}
            <Button variant="ghost" class="load-more" onclick={() => comicStore.loadMoreComics()}
              disabled={comicStore.loading} loading={comicStore.loading}>
              加载更多
            </Button>
          {/if}

        <!-- 排行榜 -->
        {:else if comicStore.activeTab === "ranking"}
          <SegmentControl options={[{ value: "H24", label: "日榜" }, { value: "D7", label: "周榜" }, { value: "D30", label: "月榜" }]} value={comicStore.rankingType} onChange={(v) => comicStore.loadRanking(v as any)} size="sm" />
          <div class="rank-list">
            {#each comicStore.ranking as comic, i (comic.id)}
              <button class="rank-row" onclick={() => comicStore.openComic(comic.id)}>
                <span class="rank-num" class:top3={i < 3}>{i + 1}</span>
                <img src={comic.thumb_url} alt={comic.title} class="rank-thumb" loading="lazy" />
                <div class="rank-info">
                  <p class="rank-title">{comic.title}</p>
                  <p class="rank-meta">{comic.author} · {comic.eps_count}话</p>
                </div>
                <span class="rank-views">{(comic.total_views / 1000).toFixed(0)}k</span>
              </button>
            {/each}
            {#if comicStore.ranking.length === 0 && !comicStore.loading}
              <EmptyState icon="chart" title="暂无排行数据" />
            {/if}
          </div>

        <!-- 随机本子 -->
        {:else if comicStore.activeTab === "random"}
          <div class="random-header">
            <Button variant="ghost" size="sm" onclick={() => comicStore.loadRandom()}
              disabled={comicStore.loading} loading={comicStore.loading}>
              <Icon name="refresh" size={15} />
              换一批
            </Button>
          </div>
          <div class="comics-grid">
            {#each comicStore.randomList as comic (comic.id)}
              <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
            {/each}
          </div>
          {#if comicStore.randomList.length === 0 && !comicStore.loading}
            <EmptyState icon="diamond" title='点击"换一批"获取随机漫画' />
          {/if}

        <!-- 收藏 -->
        {:else if comicStore.activeTab === "favorites"}
          <div class="comics-grid">
            {#each comicStore.favorites as comic (comic.id)}
              <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
            {/each}
          </div>
          {#if comicStore.favorites.length === 0 && !comicStore.loading}
            <EmptyState icon="heart" title="还没有收藏的漫画" />
          {/if}
          {#if comicStore.favPage < comicStore.favPages}
            <Button variant="ghost" class="load-more" onclick={() => comicStore.loadFavorites(comicStore.favPage + 1)}
              disabled={comicStore.loading} loading={comicStore.loading}>
              加载更多
            </Button>
          {/if}

        <!-- 阅读历史 -->
        {:else if comicStore.activeTab === "history"}
          {#if comicStore.readHistory.length > 0}
            <div class="history-header">
              <span class="history-count">{comicStore.readHistory.length} 条记录</span>
              <Button variant="ghost" size="sm" onclick={() => comicStore.clearHistory()}>清空</Button>
            </div>
            <div class="history-list">
              {#each comicStore.readHistory as rec (rec.id)}
                <div class="history-row" role="button" tabindex="0"
                  onclick={() => comicStore.openComic(rec.id)}
                  onkeydown={(e) => { if (e.key === "Enter") comicStore.openComic(rec.id); }}>
                  <img src={rec.thumb_url} alt={rec.title} class="history-thumb" loading="lazy" />
                  <div class="history-info">
                    <p class="history-title">{rec.title}</p>
                    <p class="history-meta">{rec.author}</p>
                    <p class="history-progress">读到: {rec.last_title || `第${rec.last_order}话`}</p>
                  </div>
                  <span class="history-time">{fmtDate(rec.ts)}</span>
                  <Button variant="quiet" size="sm" class="history-del" onclick={(e) => { e.stopPropagation(); comicStore.removeHistory(rec.id); }}
                    title="删除记录" ariaLabel="删除记录">
                    <Icon name="x" size={12} />
                  </Button>
                </div>
              {/each}
            </div>
          {:else}
            <EmptyState icon="eye" title="暂无阅读记录" />
          {/if}
        {/if}
      </div>
    </div>

    {#if comicStore.view === "detail" || comicStore.view === "reader"}
      <div class="overlays">
        {#if comicStore.view === "reader"}
          <ComicReader />
        {:else}
          <ComicDetail />
        {/if}
      </div>
    {/if}
  {/if}
</section>

<style>
  .comic-page {
    --accent: #e09848;
    --accent-hi: #c78438;
    --accent-lo: rgba(224,152,72,0.12);
    --accent-ring: rgba(224,152,72,0.35);
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    color: var(--text-primary);
  }

  /* ── Login Gate ── */
  .login-gate { flex: 1; display: grid; place-items: center; padding: 32px; }
  :global(.login-card) {
    width: 100%; max-width: 380px;
    display: flex; flex-direction: column; gap: 12px;
    align-items: center; text-align: center;
  }
  .login-logo {
    width: 64px; height: 64px; border-radius: 16px;
    background: var(--accent-lo, rgba(232,85,127,0.12));
    border: 1px solid var(--accent-ring, rgba(232,85,127,0.3));
    display: grid; place-items: center; color: var(--accent); margin-bottom: 4px;
  }
  .login-title { font-family: var(--font-display); font-size: 22px; font-weight: 750; margin: 0; }
  .login-sub { font-size: 13px; color: var(--text-muted); margin: 0; line-height: 1.5; }
  .login-form { width: 100%; display: flex; flex-direction: column; gap: 14px; margin-top: 8px; }
  .field { display: flex; flex-direction: column; gap: 5px; text-align: left; }
  .field label { font-size: 12px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .field :global(.ui-input) { padding: 9px 12px; font-size: 14px; }
  .login-error { font-size: 12.5px; color: #f87171; margin: 0; text-align: left; }
  .login-note { font-size: 11px; color: var(--text-dim, var(--text-muted)); margin: 0; }

  /* ── Shell ── */
  .comic-shell { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
  .comic-shell.hidden-by-overlay { visibility: hidden; pointer-events: none; }

  /* ── Header ── */
  .comic-header { flex-shrink: 0; display: flex; align-items: center; gap: 12px; padding: 14px 20px 8px; }
  .header-left { flex-shrink: 0; display: flex; flex-direction: column; gap: 2px; }
  .header-kicker { font-family: var(--font-mono); font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--accent); }
  .header-title { font-family: var(--font-display); font-size: 20px; font-weight: 750; margin: 0; display: flex; align-items: center; gap: 6px; line-height: 1; }

  .search-form { flex: 1; display: flex; gap: 8px; }
  :global(.search-wrap) { flex: 1; }

  /* ── User area ── */
  .user-area { flex-shrink: 0; display: flex; align-items: center; gap: 6px; }
  :global(.user-chip) {
    gap: 5px;
    max-width: 140px;
  }
  .user-level { font-family: var(--font-mono); font-weight: 700; color: var(--accent); font-size: 10px; }
  .user-name { max-width: 80px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  :global(.punch-btn) {
    width: 28px; height: 28px; min-height: 28px; padding: 0;
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    background: var(--accent-lo, rgba(232,85,127,0.1));
    color: var(--accent);
  }
  :global(.punch-btn:hover:not(:disabled)) { background: var(--accent); color: #fff; }
  :global(.punched-badge) {
    width: 28px; height: 24px;
    display: inline-flex; align-items: center; justify-content: center;
    padding: 0;
  }
  :global(.logout-btn) {
    width: 28px; height: 28px; min-height: 28px; padding: 0;
  }
  :global(.logout-btn:hover) { border-color: rgba(248,113,113,0.4); color: #f87171; background: rgba(248,113,113,0.08); }

  /* ── Tabs ── */
  .tab-bar { flex-shrink: 0; display: flex; align-items: center; gap: 4px; padding: 4px 20px 8px; border-bottom: 1px solid var(--border); }
  .search-label { font-size: 13px; color: var(--text-muted); flex: 1; }

  .sort-area { margin-left: auto; }
  .sort-select {
    padding: 4px 8px; border: 1px solid var(--border); border-radius: 6px;
    background: rgba(255,255,255,0.04); color: var(--text-muted);
    font-size: 12px; cursor: pointer; outline: none;
  }
  .sort-select option { background: var(--bg-deep); color: var(--text-primary); }

  /* ── Content ── */
  .comic-content { flex: 1; overflow-y: auto; padding: 16px 20px 20px; display: flex; flex-direction: column; gap: 12px; }

  .cat-chips { display: flex; flex-wrap: wrap; gap: 6px; flex-shrink: 0; }

  .comics-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 14px; }

  /* ── Random ── */
  .random-header { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }

  /* ── Ranking ── */
  .rank-list { display: flex; flex-direction: column; gap: 4px; }
  .rank-row {
    display: flex; align-items: center; gap: 12px; padding: 8px 12px;
    border: 1px solid transparent; border-radius: 8px; background: rgba(255,255,255,0.02);
    cursor: pointer; transition: all 0.15s; text-align: left; width: 100%;
  }
  .rank-row:hover { border-color: var(--border); background: rgba(255,255,255,0.04); }
  .rank-num { font-family: var(--font-mono); font-size: 14px; font-weight: 700; color: var(--text-muted); width: 24px; text-align: center; flex-shrink: 0; }
  .rank-num.top3 { color: var(--accent); }
  .rank-thumb { width: 44px; height: 60px; object-fit: cover; border-radius: 4px; flex-shrink: 0; background: var(--bg-deep); }
  .rank-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .rank-title { font-size: 13.5px; font-weight: 650; color: var(--text-primary); margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rank-meta { font-size: 11.5px; color: var(--text-muted); margin: 0; }
  .rank-views { font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); flex-shrink: 0; }

  /* ── History ── */
  .history-header { display: flex; align-items: center; justify-content: space-between; flex-shrink: 0; }
  .history-count { font-size: 12px; color: var(--text-muted); }
  .history-list { display: flex; flex-direction: column; gap: 4px; }
  .history-row {
    display: flex; align-items: center; gap: 12px; padding: 8px 12px;
    border: 1px solid transparent; border-radius: 8px; background: rgba(255,255,255,0.02);
    cursor: pointer; transition: all 0.15s;
  }
  .history-row:hover { border-color: var(--border); background: rgba(255,255,255,0.04); }
  .history-thumb { width: 40px; height: 54px; object-fit: cover; border-radius: 4px; flex-shrink: 0; background: var(--bg-deep); }
  .history-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .history-title { font-size: 13px; font-weight: 650; color: var(--text-primary); margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .history-meta { font-size: 11px; color: var(--text-muted); margin: 0; }
  .history-progress { font-size: 11px; color: var(--accent); margin: 0; }
  .history-time { font-family: var(--font-mono); font-size: 10.5px; color: var(--text-dim, var(--text-muted)); flex-shrink: 0; }
  :global(.history-del) {
    width: 24px; height: 24px; min-height: 24px; padding: 0;
    opacity: 0; transition: opacity 0.15s; flex-shrink: 0;
  }
  .history-row:hover :global(.history-del) { opacity: 1; }
  :global(.history-del:hover) { color: #f87171; background: rgba(248,113,113,0.1); }

  /* ── Misc ── */
  :global(.load-more) {
    align-self: center; margin-top: 4px;
  }

  .content-loading {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 12px; color: var(--text-muted); padding: 60px 0;
  }
  :global(.content-error) {
    flex: 1;
  }
  .loading-hint {
    font-size: 11px; color: var(--text-dim, var(--text-muted));
    opacity: 0; animation: fade-hint 0.5s ease 4s forwards;
  }
  @keyframes fade-hint { to { opacity: 0.7; } }

  .overlays { position: absolute; inset: 0; z-index: 20; pointer-events: all; }
</style>
