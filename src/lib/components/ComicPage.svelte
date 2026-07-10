<script lang="ts">
  import { onMount } from "svelte";
  import {
    comicStore,
    ORDINARY_SOURCE_OPTIONS,
    SORT_OPTIONS,
    type OrdinaryComicSource,
    type OrdinarySourceKey,
  } from "../stores/comic.svelte";
  import ComicCard from "./comic/ComicCard.svelte";
  import ComicDetail from "./comic/ComicDetail.svelte";
  import ComicReader from "./comic/ComicReader.svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, Input, LoadingSkeleton, SearchInput, SegmentControl, Tag } from "./ui";

  let pageMode = $state<"normal" | "picacg">("normal");
  let searchInput = $state("");
  let adultOpen = $state(false);
  let adultEmail = $state(comicStore.savedEmail);
  let adultPassword = $state("");
  let adultLoginError = $state("");
  let adultBusy = $state(false);
  let picacgSearchInput = $state("");
  let punchingIn = $state(false);

  const sourceOptions = ORDINARY_SOURCE_OPTIONS;

  const activeSourceLabel = $derived(
    sourceOptions.find((source) => source.value === comicStore.ordinarySource)?.label ?? "自动"
  );

  async function searchOrdinary(value = searchInput) {
    const keyword = value.trim();
    if (!keyword) return;
    searchInput = keyword;
    await comicStore.searchOrdinary(keyword);
  }

  async function selectOrdinarySource(source: OrdinaryComicSource) {
    if (comicStore.ordinarySource === source) return;
    comicStore.setOrdinarySource(source);
    if (searchInput.trim()) {
      await searchOrdinary(searchInput);
    }
  }

  async function loginPicacg(e: Event) {
    e.preventDefault();
    if (!adultEmail || !adultPassword) {
      adultLoginError = "请填写账号和密码";
      return;
    }
    adultLoginError = "";
    adultBusy = true;
    try {
      await comicStore.login(adultEmail, adultPassword);
      await comicStore.loadProfile();
      await comicStore.loadCategories();
      adultOpen = false;
      pageMode = "picacg";
    } catch {
      adultLoginError = comicStore.error ?? "登录失败";
    } finally {
      adultBusy = false;
    }
  }

  async function handlePicacgSearch(value: string) {
    if (!value.trim()) return;
    await comicStore.search(value.trim());
  }

  function clearPicacgSearch() {
    picacgSearchInput = "";
    comicStore.searchKeyword = "";
    comicStore.setTab("explore");
  }

  async function enterPicacg() {
    adultOpen = false;
    pageMode = "picacg";
    if (comicStore.isLoggedIn) {
      await comicStore.loadProfile();
      await comicStore.loadCategories();
    }
  }

  function leavePicacg() {
    pageMode = "normal";
    adultOpen = false;
  }

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

  async function handlePicacgSubmit(e: Event) {
    e.preventDefault();
    await handlePicacgSearch(picacgSearchInput);
  }

  const tabs = [
    { value: "explore", label: "探索" },
    { value: "ranking", label: "排行榜" },
    { value: "random", label: "随机" },
    { value: "favorites", label: "收藏" },
    { value: "history", label: "历史" },
  ];

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
      } else if (adultOpen) {
        adultOpen = false;
      }
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeydown, { capture: true });
    comicStore.rehydrate().then(() => {
      if (comicStore.isLoggedIn) {
        comicStore.loadProfile();
      }
    });
    return () => window.removeEventListener("keydown", onKeydown, { capture: true });
  });
</script>

<section class="comic-page" data-testid="comic-page">
  {#if pageMode === "normal"}
  <div class="comic-shell" class:hidden-by-overlay={comicStore.view !== "home"}>
    <header class="comic-header">
      <div class="header-left">
        <span class="header-kicker">Manga</span>
        <h1 class="header-title"><Icon name="book" size={20} /> 漫画</h1>
      </div>

      <form class="search-form" onsubmit={(e) => { e.preventDefault(); searchOrdinary(); }}>
        <SearchInput class="search-wrap" bind:value={searchInput}
          placeholder="搜索普通漫画..." onclear={() => searchInput = ""} />
        <Button type="submit" variant="secondary" disabled={!searchInput.trim()} loading={comicStore.mangaDexLoading}>
          <Icon name="search" size={14} />
          搜索
        </Button>
      </form>
    </header>

    <div class="source-strip">
      <div class="source-summary">
        <Tag variant="accent" size="sm"><Icon name="globe" size={12} /> {activeSourceLabel}</Tag>
        <span>普通漫画默认入口，可聚合搜索或手动切换备用源</span>
      </div>
      <div class="source-tabs" role="tablist" aria-label="普通漫画源">
        {#each sourceOptions as source}
          <button
            type="button"
            class:active={comicStore.ordinarySource === source.value}
            disabled={comicStore.mangaDexLoading}
            role="tab"
            aria-selected={comicStore.ordinarySource === source.value}
            onclick={() => selectOrdinarySource(source.value)}
          >
            <span>{source.label}</span>
            <small>{source.hint}</small>
          </button>
        {/each}
      </div>
    </div>

    <main class="comic-content">
      {#if comicStore.ordinarySourceSections.length > 0}
        <div class="ordinary-source-results">
          <div class="result-head">
            <span>多源搜索结果</span>
            <Tag variant="muted" size="sm">{comicStore.mangaDexResults.length} 条</Tag>
          </div>
          {#each comicStore.ordinarySourceSections as section (section.source)}
            <section class="ordinary-source-section">
              <div class="ordinary-source-head">
                <div>
                  <h2>{section.label}</h2>
                  <span>{section.loading ? "搜索中" : `${section.docs.length} 条结果`}</span>
                </div>
                {#if section.error}
                  <Button variant="ghost" size="sm" press={() => comicStore.retryOrdinarySource(section.source as OrdinarySourceKey)}>重试</Button>
                {/if}
              </div>
              {#if section.loading && section.docs.length === 0}
                <div class="source-section-loading">
                  <LoadingSkeleton rows={2} columns={4} />
                </div>
              {:else if section.error}
                <div class="source-section-error"><Icon name="x" size={14} /><span>{section.error}</span></div>
              {:else if section.docs.length > 0}
                <div class="comics-grid">
                  {#each section.docs as comic (comic.id)}
                    <ComicCard {comic} onclick={() => comicStore.openOrdinaryComic(comic.id)} />
                  {/each}
                </div>
              {:else}
                <p class="source-section-empty">该图源没有找到结果</p>
              {/if}
            </section>
          {/each}
        </div>
      {:else if comicStore.mangaDexLoading}
        <div class="content-loading">
          <LoadingSkeleton rows={4} columns={4} />
          <span class="loading-hint">正在从普通漫画源检索...</span>
        </div>
      {:else if comicStore.mangaDexError}
        <EmptyState icon="x" title="普通漫画源暂时不可用" description={comicStore.mangaDexError} />
      {:else}
        <div class="ordinary-empty">
          <Icon name="search" size={30} />
          <h2>搜索漫画，直接阅读</h2>
          <p>自动模式会并行搜索所有内置漫画源，单个图源失败不会影响其他结果。</p>
          <div class="quick-searches">
            {#each ["海贼王", "葬送的芙莉莲", "迷宫饭", "电锯人"] as keyword}
              <Button variant="ghost" size="sm" press={() => searchOrdinary(keyword)}>{keyword}</Button>
            {/each}
          </div>
        </div>
      {/if}
    </main>
  </div>

  <div class="adult-entry">
    {#if adultOpen}
      <div class="adult-popover" role="dialog" aria-label="PicACG 18+ 入口">
        <div class="adult-popover-head">
          <div>
            <span class="adult-kicker">18+ Entry</span>
            <h2>PicACG</h2>
          </div>
          <Button variant="quiet" size="sm" press={() => adultOpen = false} ariaLabel="关闭 PicACG 入口">
            <Icon name="x" size={14} />
          </Button>
        </div>

        {#if !comicStore.isLoggedIn}
          <form class="adult-form" onsubmit={loginPicacg}>
            <label>
              <span>邮箱 / 用户名</span>
              <Input type="text" bind:value={adultEmail} placeholder="邮箱或用户名" autocomplete="username" disabled={adultBusy} />
            </label>
            <label>
              <span>密码</span>
              <Input type="password" bind:value={adultPassword} placeholder="••••••••" autocomplete="current-password" disabled={adultBusy} />
            </label>
            {#if adultLoginError}<p class="adult-error">{adultLoginError}</p>{/if}
            <Button type="submit" fullWidth loading={adultBusy}>登录 PicACG</Button>
          </form>
        {:else}
          <div class="adult-profile">
            <Tag variant="muted" size="md">
              <span class="user-level">Lv.{comicStore.profile?.level ?? "-"}</span>
              <span>{comicStore.profile?.name ?? "PicACG"}</span>
            </Tag>
            <Button variant="quiet" size="sm" press={() => comicStore.logout()}>退出</Button>
          </div>

          <p class="adult-note">进入后会切换到独立 PicACG 漫画页，不影响普通漫画搜索。</p>
          <Button variant="primary" fullWidth press={enterPicacg}>进入 PicACG</Button>
        {/if}
      </div>
    {/if}

    <button class="adult-pill" type="button" aria-expanded={adultOpen} onclick={() => adultOpen = !adultOpen}>
      <Icon name="shield" size={15} />
      <span>18+</span>
    </button>
  </div>
  {:else}
    {#if !comicStore.isLoggedIn}
      <div class="login-gate">
        <Card class="login-card" padding="lg">
          <div class="login-logo"><Icon name="book" size={36} /></div>
          <h1 class="login-title">PicACG</h1>
          <p class="login-sub">成人内容入口已独立显示。登录后恢复原来的 PicACG 漫画页。</p>
          <form class="login-form" onsubmit={loginPicacg}>
            <div class="field">
              <label for="picacg-email">邮箱 / 用户名</label>
              <Input id="picacg-email" type="text" bind:value={adultEmail}
                placeholder="邮箱或用户名" autocomplete="username" disabled={adultBusy} />
            </div>
            <div class="field">
              <label for="picacg-pwd">密码</label>
              <Input id="picacg-pwd" type="password" bind:value={adultPassword}
                placeholder="••••••••" autocomplete="current-password" disabled={adultBusy} />
            </div>
            {#if adultLoginError}<p class="login-error">{adultLoginError}</p>{/if}
            <Button type="submit" fullWidth loading={adultBusy}>登录</Button>
          </form>
          <Button variant="ghost" size="sm" press={leavePicacg}>返回普通漫画</Button>
        </Card>
      </div>
    {:else}
      <div class="picacg-shell comic-shell" class:hidden-by-overlay={comicStore.view !== "home"}>
        <header class="comic-header">
          <div class="header-left">
            <span class="header-kicker">PicACG</span>
            <h1 class="header-title"><Icon name="book" size={20} /> 哔咔漫画</h1>
          </div>

          <form class="search-form" onsubmit={handlePicacgSubmit}>
            <SearchInput class="search-wrap" bind:value={picacgSearchInput}
              placeholder="搜索 PicACG..." onclear={clearPicacgSearch} />
            <Button type="submit" variant="secondary" disabled={!picacgSearchInput.trim()}>搜索</Button>
          </form>

          <div class="user-area">
            {#if comicStore.profile}
              <Tag variant="muted" size="md" class="user-chip" title={comicStore.profile.slogan || comicStore.profile.name}>
                <span class="user-level">Lv.{comicStore.profile.level}</span>
                <span class="user-name">{comicStore.profile.name}</span>
              </Tag>
              {#if !comicStore.profile.is_punched}
                <Button variant="ghost" size="sm" class="punch-btn" press={handlePunchIn} disabled={punchingIn}
                  title="每日打卡" ariaLabel="每日打卡">
                  <Icon name="zap" size={13} />
                </Button>
              {:else}
                <Tag variant="accent" size="sm" class="punched-badge" title="今日已打卡"><Icon name="check" size={13} /></Tag>
              {/if}
            {/if}
            <Button variant="ghost" size="sm" class="normal-btn" press={leavePicacg}>普通漫画</Button>
            <Button variant="ghost" size="sm" class="logout-btn" press={() => { comicStore.logout(); pageMode = "normal"; }} title="退出登录" ariaLabel="退出登录">
              <Icon name="x" size={14} />
            </Button>
          </div>
        </header>

        <div class="tab-bar">
          {#if !comicStore.searchKeyword}
            <SegmentControl options={tabs} value={comicStore.activeTab} onChange={(v) => comicStore.setTab(v as any)} size="sm" />

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
            <Button variant="ghost" size="sm" press={clearPicacgSearch}>清除</Button>
          {/if}
        </div>

        <div class="comic-content">
          {#if comicStore.error}
            <EmptyState class="content-error" icon="x" title={comicStore.error}
              action={{ label: "重试", onclick: () => { comicStore.clearError(); comicStore.loadCategories(); } }} />
          {:else if comicStore.loading && comicStore.comicList.length === 0 && comicStore.searchResults.length === 0 && comicStore.ranking.length === 0 && comicStore.randomList.length === 0 && comicStore.favorites.length === 0}
            <div class="content-loading">
              <LoadingSkeleton rows={6} columns={4} />
              <span class="loading-hint">若长时间无响应，请检查网络或代理设置</span>
            </div>
          {:else if comicStore.searchKeyword && comicStore.searchResults.length > 0}
            <div class="comics-grid">
              {#each comicStore.searchResults as comic (comic.id)}
                <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
              {/each}
            </div>
            {#if comicStore.searchPage < comicStore.searchPages}
              <Button variant="ghost" class="load-more" press={() => comicStore.searchNextPage()}
                disabled={comicStore.loading} loading={comicStore.loading}>
                加载更多
              </Button>
            {/if}
          {:else if comicStore.searchKeyword && !comicStore.loading}
            <EmptyState icon="search" title="没有找到相关漫画" />
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
              <Button variant="ghost" class="load-more" press={() => comicStore.loadMoreComics()}
                disabled={comicStore.loading} loading={comicStore.loading}>
                加载更多
              </Button>
            {/if}
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
          {:else if comicStore.activeTab === "random"}
            <div class="random-header">
              <Button variant="ghost" size="sm" press={() => comicStore.loadRandom()}
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
              <Button variant="ghost" class="load-more" press={() => comicStore.loadFavorites(comicStore.favPage + 1)}
                disabled={comicStore.loading} loading={comicStore.loading}>
                加载更多
              </Button>
            {/if}
          {:else if comicStore.activeTab === "history"}
            {#if comicStore.readHistory.length > 0}
              <div class="history-header">
                <span class="history-count">{comicStore.readHistory.length} 条记录</span>
                <Button variant="ghost" size="sm" press={() => comicStore.clearHistory()}>清空</Button>
              </div>
              <div class="history-list">
                {#each comicStore.readHistory as rec (rec.id)}
                  <div class="history-row" role="button" tabindex="0"
                    onclick={() => comicStore.resumeHistory(rec)}
                    onkeydown={(e) => { if (e.key === "Enter") comicStore.resumeHistory(rec); }}>
                    <img src={rec.thumb_url} alt={rec.title} class="history-thumb" loading="lazy" />
                    <div class="history-info">
                      <p class="history-title">{rec.title}</p>
                      <p class="history-meta">{rec.author}</p>
                      <p class="history-progress">读到: {rec.last_title || `第${rec.last_order}话`}</p>
                    </div>
                    <span class="history-time">{fmtDate(rec.ts)}</span>
                    <Button variant="quiet" size="sm" class="history-del" press={(e) => { e.stopPropagation(); comicStore.removeHistory(rec.id); }}
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
    {/if}
  {/if}

  {#if comicStore.view === "detail" || comicStore.view === "reader"}
    <div class="overlays">
      {#if comicStore.view === "reader"}
        <ComicReader />
      {:else}
        <ComicDetail />
      {/if}
    </div>
  {/if}
</section>

<style>
  .comic-page {
    --accent: #e09848;
    --accent-hi: #c78438;
    --accent-lo: rgba(224,152,72,0.12);
    --accent-ring: rgba(224,152,72,0.35);
    height: 100%;
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  .login-gate {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 32px;
  }

  :global(.login-card) {
    width: 100%;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: center;
    text-align: center;
  }

  .login-logo {
    width: 64px;
    height: 64px;
    border-radius: 16px;
    background: var(--accent-lo);
    border: 1px solid var(--accent-ring);
    display: grid;
    place-items: center;
    color: var(--accent);
  }

  .login-title {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 750;
    margin: 0;
  }

  .login-sub {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0;
    line-height: 1.5;
  }

  .login-form {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin-top: 8px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    text-align: left;
  }

  .field label {
    font-size: 12px;
    font-weight: 650;
    color: var(--text-muted);
  }

  .login-error {
    font-size: 12.5px;
    color: #f87171;
    margin: 0;
    text-align: left;
  }

  .comic-shell {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .picacg-shell {
    --accent: #e8557f;
    --accent-hi: #d93f6c;
    --accent-lo: rgba(232,85,127,0.12);
    --accent-ring: rgba(232,85,127,0.35);
  }

  .comic-shell.hidden-by-overlay {
    visibility: hidden;
    pointer-events: none;
  }

  .comic-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 20px 10px;
  }

  .header-left {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .header-kicker,
  .adult-kicker {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent);
  }

  .header-title {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 750;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    line-height: 1;
  }

  .search-form {
    flex: 1;
    display: flex;
    gap: 8px;
    min-width: 260px;
  }

  .user-area {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.user-chip) {
    gap: 5px;
    max-width: 140px;
  }

  .user-name {
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.punch-btn),
  :global(.logout-btn) {
    width: 28px;
    height: 28px;
    min-height: 28px;
    padding: 0;
  }

  :global(.normal-btn) {
    white-space: nowrap;
  }

  :global(.punched-badge) {
    width: 28px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .tab-bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 20px 8px;
    border-bottom: 1px solid var(--border);
  }

  .search-label {
    font-size: 13px;
    color: var(--text-muted);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sort-area {
    margin-left: auto;
  }

  .sort-select {
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: rgba(255,255,255,0.04);
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
    outline: none;
  }

  .sort-select option {
    background: var(--bg-deep);
    color: var(--text-primary);
  }

  :global(.search-wrap) {
    flex: 1;
  }

  .source-strip {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    padding: 8px 20px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .source-summary {
    min-width: 220px;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .source-tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow-x: auto;
    scrollbar-width: thin;
  }

  .source-tabs button {
    min-width: 82px;
    height: 42px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(255,255,255,0.025);
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    cursor: pointer;
  }

  .source-tabs button:hover {
    border-color: var(--border-hover, var(--border));
    color: var(--text-primary);
    background: rgba(255,255,255,0.045);
  }

  .source-tabs button.active {
    border-color: var(--accent-ring);
    background: var(--accent-lo);
    color: var(--text-primary);
  }

  .source-tabs button:disabled {
    opacity: 0.58;
    cursor: wait;
  }

  .source-tabs span {
    font-size: 12px;
    font-weight: 750;
    line-height: 1;
  }

  .source-tabs small {
    max-width: 72px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-dim, var(--text-muted));
    font-size: 10px;
    line-height: 1.1;
  }

  .comic-content {
    flex: 1;
    overflow-y: auto;
    padding: 18px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .cat-chips,
  .random-header,
  .history-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .history-header {
    justify-content: space-between;
  }

  .history-count {
    color: var(--text-muted);
    font-size: 12px;
  }

  .result-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 700;
  }

  .comics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 14px;
  }

  .ordinary-source-results {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .ordinary-source-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-bottom: 18px;
    border-bottom: 1px solid var(--border);
  }

  .ordinary-source-section:last-child {
    border-bottom: 0;
  }

  .ordinary-source-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .ordinary-source-head h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 15px;
  }

  .ordinary-source-head span {
    display: block;
    margin-top: 2px;
    color: var(--text-muted);
    font-size: 10.5px;
  }

  .source-section-loading {
    min-height: 118px;
  }

  .source-section-error,
  .source-section-empty {
    margin: 0;
    min-height: 70px;
    padding: 14px;
    border: 1px dashed var(--border);
    border-radius: 10px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .source-section-error {
    color: #fca5a5;
    background: rgba(248,113,113,0.05);
  }

  .source-section-empty {
    justify-content: center;
  }

  .rank-list,
  .history-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .rank-row,
  .history-row {
    width: 100%;
    border: 1px solid transparent;
    border-radius: 8px;
    background: rgba(255,255,255,0.02);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .rank-row:hover,
  .history-row:hover {
    border-color: var(--border);
    background: rgba(255,255,255,0.04);
  }

  .rank-num {
    width: 24px;
    flex-shrink: 0;
    text-align: center;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-weight: 760;
  }

  .rank-num.top3 {
    color: var(--accent);
  }

  .rank-thumb,
  .history-thumb {
    width: 44px;
    height: 60px;
    object-fit: cover;
    border-radius: 4px;
    flex-shrink: 0;
    background: var(--bg-deep);
  }

  .history-thumb {
    width: 40px;
    height: 54px;
  }

  .rank-info,
  .history-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .rank-title,
  .history-title {
    margin: 0;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rank-meta,
  .history-meta {
    margin: 0;
    color: var(--text-muted);
    font-size: 11px;
  }

  .rank-views,
  .history-time {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    white-space: nowrap;
  }

  .history-progress {
    margin: 0;
    color: var(--accent);
    font-size: 11px;
  }

  :global(.history-del) {
    width: 24px;
    height: 24px;
    min-height: 24px;
    padding: 0;
    opacity: 0;
    flex-shrink: 0;
    transition: opacity 0.15s ease;
  }

  .history-row:hover :global(.history-del) {
    opacity: 1;
  }

  :global(.load-more) {
    align-self: center;
    margin-top: 4px;
  }

  .ordinary-empty,
  .content-loading {
    min-height: 440px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 12px;
    color: var(--text-muted);
  }

  .ordinary-empty h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 22px;
    font-family: var(--font-display);
  }

  .ordinary-empty p,
  .adult-note {
    margin: 0;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-muted);
  }

  .quick-searches {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
  }

  .loading-hint {
    font-size: 11px;
    color: var(--text-dim, var(--text-muted));
  }

  .adult-entry {
    position: absolute;
    right: 22px;
    bottom: 22px;
    z-index: 40;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 10px;
  }

  .adult-pill {
    min-width: 68px;
    height: 38px;
    border: 1px solid rgba(248,113,113,0.36);
    border-radius: 999px;
    background: rgba(31, 10, 14, 0.92);
    color: #fca5a5;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    font-weight: 800;
    cursor: pointer;
    box-shadow: 0 12px 34px rgba(0,0,0,0.28);
  }

  .adult-pill:hover {
    background: rgba(59, 16, 22, 0.96);
  }

  .adult-popover {
    width: min(380px, calc(100vw - 44px));
    max-height: min(620px, calc(100vh - 96px));
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(12, 14, 20, 0.98);
    box-shadow: 0 22px 60px rgba(0,0,0,0.42);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .adult-popover-head,
  .adult-profile {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .adult-popover h2 {
    margin: 2px 0 0;
    font-size: 18px;
    font-family: var(--font-display);
  }

  .adult-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .adult-form label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .adult-error {
    margin: 0;
    font-size: 12px;
    color: #f87171;
  }

  .user-level {
    font-family: var(--font-mono);
    color: var(--accent);
    font-weight: 700;
  }

  .overlays {
    position: absolute;
    inset: 0;
    z-index: 30;
    pointer-events: all;
  }

  @media (max-width: 700px) {
    .comic-header {
      flex-direction: column;
      align-items: stretch;
    }

    .search-form {
      min-width: 0;
    }

    .source-strip {
      align-items: flex-start;
      flex-direction: column;
      gap: 6px;
    }

    .source-summary {
      min-width: 0;
      align-items: flex-start;
      flex-direction: column;
      gap: 6px;
    }

    .source-tabs {
      width: 100%;
    }

    .adult-entry {
      right: 14px;
      bottom: 14px;
    }
  }
</style>
