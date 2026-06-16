<script lang="ts">
  import { onMount } from "svelte";
  import { comicStore, SORT_OPTIONS } from "../stores/comic.svelte";
  import ComicCard from "./comic/ComicCard.svelte";
  import ComicDetail from "./comic/ComicDetail.svelte";
  import ComicReader from "./comic/ComicReader.svelte";
  import Icon from "./Icon.svelte";

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

  async function handleSearch(e: Event) {
    e.preventDefault();
    if (!searchInput.trim()) return;
    await comicStore.search(searchInput.trim());
  }

  function clearSearch() {
    searchInput = "";
    comicStore.searchKeyword = "";
    comicStore.setTab("explore");
  }

  function onKeydown(e: KeyboardEvent) {
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
    { id: "explore",   label: "探索" },
    { id: "ranking",   label: "排行榜" },
    { id: "random",    label: "随机" },
    { id: "favorites", label: "收藏" },
    { id: "history",   label: "历史" },
  ] as const;

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
      <div class="login-card">
        <div class="login-logo"><Icon name="book" size={36} /></div>
        <h1 class="login-title">哔咔漫画</h1>
        <p class="login-sub">使用你的哔咔账号登录，畅读海量漫画</p>
        <form class="login-form" onsubmit={handleLogin}>
          <div class="field">
            <label for="email">邮箱 / 用户名</label>
            <input id="email" type="text" bind:value={loginEmail}
              placeholder="邮箱或用户名" autocomplete="username" disabled={comicStore.loading} />
          </div>
          <div class="field">
            <label for="pwd">密码</label>
            <input id="pwd" type="password" bind:value={loginPassword}
              placeholder="••••••••" autocomplete="current-password" disabled={comicStore.loading} />
          </div>
          {#if loginError}<p class="login-error">{loginError}</p>{/if}
          <button type="submit" class="login-btn" disabled={comicStore.loading}>
            {comicStore.loading ? "登录中..." : "登录"}
          </button>
        </form>
        <p class="login-note">账号为你的哔咔注册账号，萌游不存储密码。</p>
      </div>
    </div>

  {:else}
    <!-- ── 已登录主界面 ── -->
    <div class="comic-shell" class:hidden-by-overlay={comicStore.view !== "home"}>
      <header class="comic-header">
        <div class="header-left">
          <span class="header-kicker">Comic</span>
          <h1 class="header-title"><Icon name="book" size={20} /> 漫画</h1>
        </div>

        <form class="search-form" onsubmit={handleSearch}>
          <div class="search-wrap">
            <Icon name="search" size={15} />
            <input class="search-input" type="text" placeholder="搜索漫画..."
              bind:value={searchInput} disabled={comicStore.loading} />
            {#if searchInput}
              <button type="button" class="search-clear" onclick={clearSearch}>
                <Icon name="x" size={13} />
              </button>
            {/if}
          </div>
          <button type="submit" class="search-btn" disabled={!searchInput.trim()}>搜索</button>
        </form>

        <!-- 用户信息 -->
        <div class="user-area">
          {#if comicStore.profile}
            <div class="user-chip" title={comicStore.profile.slogan || comicStore.profile.name}>
              <span class="user-level">Lv.{comicStore.profile.level}</span>
              <span class="user-name">{comicStore.profile.name}</span>
            </div>
            {#if !comicStore.profile.is_punched}
              <button class="punch-btn" onclick={handlePunchIn} disabled={punchingIn}
                title="每日打卡">
                <Icon name="zap" size={13} />
              </button>
            {:else}
              <span class="punched-badge" title="今日已打卡"><Icon name="check" size={13} /></span>
            {/if}
          {/if}
          <button class="logout-btn" onclick={() => comicStore.logout()} title="退出登录">
            <Icon name="x" size={14} />
          </button>
        </div>
      </header>

      <!-- Tab Bar -->
      <div class="tab-bar">
        {#if !comicStore.searchKeyword}
          {#each tabs as tab (tab.id)}
            <button class="tab-btn" class:active={comicStore.activeTab === tab.id}
              onclick={() => comicStore.setTab(tab.id as any)}>
              {tab.label}
            </button>
          {/each}

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
          <button class="tab-clear" onclick={clearSearch}>清除</button>
        {/if}
      </div>

      <!-- Content -->
      <div class="comic-content">
        {#if comicStore.error}
          <div class="content-error">
            <Icon name="x" size={28} />
            <p>{comicStore.error}</p>
            <button onclick={() => { comicStore.clearError(); comicStore.loadCategories(); }}>重试</button>
          </div>
        {:else if comicStore.loading && comicStore.comicList.length === 0 && comicStore.searchResults.length === 0 && comicStore.ranking.length === 0 && comicStore.randomList.length === 0 && comicStore.favorites.length === 0}
          <div class="content-loading">
            <div class="spinner"></div>
            <span>连接哔咔服务器...</span>
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
            <button class="load-more" onclick={() => comicStore.searchNextPage()}
              disabled={comicStore.loading}>
              {comicStore.loading ? "加载中..." : "加载更多"}
            </button>
          {/if}
        {:else if comicStore.searchKeyword && !comicStore.loading}
          <div class="empty-state"><Icon name="search" size={36} /><p>没有找到相关漫画</p></div>

        <!-- 探索 -->
        {:else if comicStore.activeTab === "explore"}
          {#if comicStore.categories.length > 0}
            <div class="cat-chips">
              <button class="cat-chip" class:active={comicStore.selectedCategory === null}
                onclick={() => comicStore.selectCategory(null)}>全部</button>
              {#each comicStore.categories as cat (cat.id || cat.title)}
                <button class="cat-chip"
                  class:active={comicStore.selectedCategory === cat.title}
                  onclick={() => comicStore.selectCategory(cat.title)}>
                  {cat.title}
                </button>
              {/each}
            </div>
          {/if}
          <div class="comics-grid">
            {#each comicStore.comicList as comic (comic.id)}
              <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
            {/each}
          </div>
          {#if comicStore.comicList.length === 0 && !comicStore.loading}
            <div class="empty-state"><Icon name="book" size={36} /><p>暂无漫画</p></div>
          {/if}
          {#if comicStore.comicPage < comicStore.comicPages}
            <button class="load-more" onclick={() => comicStore.loadMoreComics()}
              disabled={comicStore.loading}>
              {comicStore.loading ? "加载中..." : "加载更多"}
            </button>
          {/if}

        <!-- 排行榜 -->
        {:else if comicStore.activeTab === "ranking"}
          <div class="rank-tabs">
            {#each [{ v: "H24", l: "日榜" }, { v: "D7", l: "周榜" }, { v: "D30", l: "月榜" }] as r (r.v)}
              <button class="rank-tab" class:active={comicStore.rankingType === r.v}
                onclick={() => comicStore.loadRanking(r.v as any)}>{r.l}</button>
            {/each}
          </div>
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
              <div class="empty-state"><Icon name="chart" size={32} /><p>暂无排行数据</p></div>
            {/if}
          </div>

        <!-- 随机本子 -->
        {:else if comicStore.activeTab === "random"}
          <div class="random-header">
            <button class="refresh-btn" onclick={() => comicStore.loadRandom()}
              disabled={comicStore.loading}>
              <Icon name="refresh" size={15} />
              {comicStore.loading ? "加载中..." : "换一批"}
            </button>
          </div>
          <div class="comics-grid">
            {#each comicStore.randomList as comic (comic.id)}
              <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
            {/each}
          </div>
          {#if comicStore.randomList.length === 0 && !comicStore.loading}
            <div class="empty-state"><Icon name="diamond" size={36} /><p>点击"换一批"获取随机漫画</p></div>
          {/if}

        <!-- 收藏 -->
        {:else if comicStore.activeTab === "favorites"}
          <div class="comics-grid">
            {#each comicStore.favorites as comic (comic.id)}
              <ComicCard {comic} onclick={() => comicStore.openComic(comic.id)} />
            {/each}
          </div>
          {#if comicStore.favorites.length === 0 && !comicStore.loading}
            <div class="empty-state"><Icon name="heart" size={36} /><p>还没有收藏的漫画</p></div>
          {/if}
          {#if comicStore.favPage < comicStore.favPages}
            <button class="load-more" onclick={() => comicStore.loadFavorites(comicStore.favPage + 1)}
              disabled={comicStore.loading}>
              {comicStore.loading ? "加载中..." : "加载更多"}
            </button>
          {/if}

        <!-- 阅读历史 -->
        {:else if comicStore.activeTab === "history"}
          {#if comicStore.readHistory.length > 0}
            <div class="history-header">
              <span class="history-count">{comicStore.readHistory.length} 条记录</span>
              <button class="history-clear" onclick={() => comicStore.clearHistory()}>清空</button>
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
                  <button class="history-del" onclick={(e) => { e.stopPropagation(); comicStore.removeHistory(rec.id); }}
                    title="删除记录">
                    <Icon name="x" size={12} />
                  </button>
                </div>
              {/each}
            </div>
          {:else}
            <div class="empty-state"><Icon name="eye" size={36} /><p>暂无阅读记录</p></div>
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
  .login-card {
    width: 100%; max-width: 380px;
    background: rgba(255,255,255,0.03); border: 1px solid var(--border);
    border-radius: 16px; padding: 36px 32px;
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
  .field input {
    padding: 9px 12px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.04); color: var(--text-primary);
    font-size: 14px; transition: border-color 0.15s; outline: none;
  }
  .field input:focus { border-color: var(--accent); }
  .field input:disabled { opacity: 0.5; }
  .login-error { font-size: 12.5px; color: #f87171; margin: 0; text-align: left; }
  .login-btn {
    padding: 11px; border: none; border-radius: 8px;
    background: var(--accent, #e8557f); color: #fff;
    font-size: 14px; font-weight: 700; cursor: pointer;
    transition: opacity 0.15s, transform 0.15s;
  }
  .login-btn:hover:not(:disabled) { opacity: 0.9; transform: translateY(-1px); }
  .login-btn:active:not(:disabled) { transform: translateY(0) scale(0.98); }
  .login-btn:disabled { opacity: 0.5; cursor: not-allowed; }
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
  .search-wrap {
    flex: 1; display: flex; align-items: center; gap: 8px; padding: 0 10px;
    border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.03); color: var(--text-muted); transition: border-color 0.15s;
  }
  .search-wrap:focus-within { border-color: var(--accent); color: var(--text-primary); }
  .search-input { flex: 1; background: transparent; border: none; color: var(--text-primary); font-size: 13px; outline: none; padding: 8px 0; }
  .search-input::placeholder { color: var(--text-muted); }
  .search-clear { background: transparent; border: none; color: var(--text-muted); cursor: pointer; padding: 0; display: flex; align-items: center; }
  .search-btn {
    padding: 0 14px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.04); color: var(--text-muted);
    font-size: 13px; cursor: pointer; transition: all 0.15s; white-space: nowrap;
  }
  .search-btn:not(:disabled):hover { border-color: var(--accent); color: var(--accent); }
  .search-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  /* ── User area ── */
  .user-area { flex-shrink: 0; display: flex; align-items: center; gap: 6px; }
  .user-chip {
    display: flex; align-items: center; gap: 5px;
    padding: 4px 10px; border-radius: 6px;
    background: rgba(255,255,255,0.04); border: 1px solid var(--border);
    font-size: 11.5px; color: var(--text-muted);
  }
  .user-level { font-family: var(--font-mono); font-weight: 700; color: var(--accent); font-size: 10px; }
  .user-name { max-width: 80px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .punch-btn {
    width: 28px; height: 28px; display: grid; place-items: center;
    border: 1px solid var(--accent-ring, rgba(232,85,127,0.3)); border-radius: 6px;
    background: var(--accent-lo, rgba(232,85,127,0.1)); color: var(--accent);
    cursor: pointer; transition: all 0.15s;
  }
  .punch-btn:hover:not(:disabled) { background: var(--accent); color: #fff; }
  .punch-btn:disabled { opacity: 0.5; }
  .punched-badge {
    width: 28px; height: 28px; display: grid; place-items: center;
    border-radius: 6px; background: rgba(74,222,128,0.1);
    border: 1px solid rgba(74,222,128,0.3); color: #4ade80;
  }
  .logout-btn {
    flex-shrink: 0; width: 28px; height: 28px; display: grid; place-items: center;
    border: 1px solid rgba(255,255,255,0.08); border-radius: 6px;
    background: transparent; color: var(--text-muted); cursor: pointer; transition: all 0.15s;
  }
  .logout-btn:hover { border-color: rgba(248,113,113,0.4); color: #f87171; background: rgba(248,113,113,0.08); }

  /* ── Tabs ── */
  .tab-bar { flex-shrink: 0; display: flex; align-items: center; gap: 4px; padding: 4px 20px 8px; border-bottom: 1px solid var(--border); }
  .tab-btn {
    padding: 6px 14px; border: 1px solid transparent; border-radius: 6px;
    background: transparent; color: var(--text-muted);
    font-size: 13px; font-weight: 550; cursor: pointer; transition: all 0.15s;
  }
  .tab-btn.active {
    background: var(--accent-lo, rgba(232,85,127,0.1));
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    color: var(--accent); font-weight: 700;
  }
  .tab-btn:not(.active):hover { background: rgba(255,255,255,0.04); color: var(--text-primary); }
  .search-label { font-size: 13px; color: var(--text-muted); flex: 1; }
  .tab-clear {
    padding: 5px 12px; border: 1px solid rgba(255,255,255,0.1); border-radius: 6px;
    background: transparent; color: var(--text-muted); font-size: 12px; cursor: pointer;
  }
  .tab-clear:hover { color: var(--text-primary); }

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
  .cat-chip {
    padding: 5px 12px; border: 1px solid var(--border); border-radius: 999px;
    background: transparent; color: var(--text-muted);
    font-size: 12px; cursor: pointer; transition: all 0.15s; white-space: nowrap;
  }
  .cat-chip.active {
    background: var(--accent-lo, rgba(232,85,127,0.1));
    border-color: var(--accent-ring, rgba(232,85,127,0.3)); color: var(--accent);
  }
  .cat-chip:not(.active):hover { background: rgba(255,255,255,0.04); color: var(--text-primary); }

  .comics-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 14px; }

  /* ── Random ── */
  .random-header { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .refresh-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 7px 16px; border: 1px solid var(--accent-ring, rgba(232,85,127,0.3));
    border-radius: 8px; background: var(--accent-lo, rgba(232,85,127,0.1));
    color: var(--accent); font-size: 13px; font-weight: 600; cursor: pointer; transition: all 0.15s;
  }
  .refresh-btn:hover:not(:disabled) { background: var(--accent); color: #fff; }
  .refresh-btn:disabled { opacity: 0.5; }

  /* ── Ranking ── */
  .rank-tabs { display: flex; gap: 4px; flex-shrink: 0; }
  .rank-tab {
    padding: 5px 14px; border: 1px solid var(--border); border-radius: 6px;
    background: transparent; color: var(--text-muted); font-size: 12.5px; cursor: pointer; transition: all 0.15s;
  }
  .rank-tab.active {
    background: var(--accent-lo, rgba(232,85,127,0.1));
    border-color: var(--accent-ring, rgba(232,85,127,0.3));
    color: var(--accent); font-weight: 700;
  }
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
  .history-clear {
    padding: 4px 12px; border: 1px solid rgba(248,113,113,0.3); border-radius: 6px;
    background: rgba(248,113,113,0.08); color: #f87171; font-size: 12px; cursor: pointer;
  }
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
  .history-del {
    width: 24px; height: 24px; display: grid; place-items: center;
    border: none; border-radius: 4px; background: transparent;
    color: var(--text-muted); cursor: pointer; opacity: 0; transition: all 0.15s; flex-shrink: 0;
  }
  .history-row:hover .history-del { opacity: 1; }
  .history-del:hover { color: #f87171; background: rgba(248,113,113,0.1); }

  /* ── Misc ── */
  .load-more {
    align-self: center; padding: 9px 24px; border: 1px solid var(--border);
    border-radius: 8px; background: transparent; color: var(--text-muted);
    font-size: 13px; cursor: pointer; transition: all 0.15s; margin-top: 4px;
  }
  .load-more:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .load-more:disabled { opacity: 0.5; cursor: not-allowed; }

  .empty-state {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 12px; color: var(--text-muted); padding: 60px 0;
  }
  .empty-state p { margin: 0; font-size: 14px; }

  .content-loading, .content-error {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 12px; color: var(--text-muted); padding: 60px 0;
  }
  .content-error button {
    margin-top: 8px; padding: 7px 18px; border: 1px solid var(--border);
    border-radius: 6px; background: transparent; color: var(--text-muted); cursor: pointer;
  }
  .loading-hint {
    font-size: 11px; color: var(--text-dim, var(--text-muted));
    opacity: 0; animation: fade-hint 0.5s ease 4s forwards;
  }
  @keyframes fade-hint { to { opacity: 0.7; } }

  .spinner {
    width: 32px; height: 32px;
    border: 3px solid rgba(255,255,255,0.08); border-top-color: var(--accent);
    border-radius: 50%; animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .overlays { position: absolute; inset: 0; z-index: 20; pointer-events: all; }
</style>
