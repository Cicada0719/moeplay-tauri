<script lang="ts">
  import { onMount } from "svelte";
  import { animeStore } from "../../stores/anime.svelte";
  import type { AnimeHistory, BangumiSubject, SearchItem } from "../../stores/anime.svelte";
  import { attachGamepad, type GamepadAttachment } from "../../components/switch/useGamepad.svelte";
  import Icon from "../../components/Icon.svelte";
  import HandheldStatePanel from "./HandheldStatePanel.svelte";

  type AnimeTab = "recommend" | "calendar" | "my" | "rules";
  type HubItem =
    | { id: string; title: string; cover: string; meta: string; kind: "subject"; subject: BangumiSubject }
    | { id: string; title: string; cover: string; meta: string; kind: "history"; history: AnimeHistory }
    | { id: string; title: string; cover: string; meta: string; kind: "collection"; collection: { name: string; image?: string; sourceUrl?: string; ruleSource?: string } };
  type HubLane = { id: string; kicker: string; title: string; hint: string; items: HubItem[] };

  interface Props {
    onSearch: (keyword: string) => Promise<void>;
    onOpenResult: (ruleName: string, item: SearchItem, trigger?: HTMLElement) => void;
    onOpenSubject: (subject: BangumiSubject, trigger?: HTMLElement) => void;
    onResumeHistory: (item: AnimeHistory, trigger?: HTMLElement) => void;
    onOpenRules: () => void;
    onOpenProvider: () => void;
    onBack: () => void;
  }

  let {
    onSearch,
    onOpenResult,
    onOpenSubject,
    onResumeHistory,
    onOpenRules,
    onOpenProvider,
    onBack,
  }: Props = $props();

  const tabs: Array<{ id: AnimeTab; label: string; icon: string }> = [
    { id: "recommend", label: "推荐", icon: "star" },
    { id: "calendar", label: "时间表", icon: "calendar" },
    { id: "my", label: "我的", icon: "user" },
    { id: "rules", label: "规则", icon: "settings" },
  ];

  let searchInput = $state("");
  let searchBusy = $state(false);
  let activeLane = $state(0);
  let activeIndex = $state(0);
  let laneHosts: Array<HTMLElement | null> = [];
  let pad: GamepadAttachment | null = null;

  const currentTab = $derived(animeStore.activeTab as AnimeTab);
  const history = $derived(animeStore.history.slice(0, 12));
  const todayWeekday = $derived(new Date().getDay() || 7);
  const today = $derived((animeStore.calendar ?? []).find((day) => day.weekday === todayWeekday)?.items ?? []);
  const collection = $derived(animeStore.collection.slice(0, 12));
  const searchResults = $derived(animeStore.mergedSearchResults);

  function imageFor(url: string | undefined): string {
    if (!url) return "";
    return animeStore.getImg(url) || url;
  }

  function historyItem(item: AnimeHistory): HubItem {
    return {
      id: `history:${item.key}`,
      title: item.name,
      cover: imageFor(item.image),
      meta: item.lastEpisodeName || `第 ${item.lastEpisode + 1} 集`,
      kind: "history",
      history: item,
    };
  }

  function subjectItem(subject: BangumiSubject, prefix: string): HubItem {
    return {
      id: `${prefix}:${subject.id}`,
      title: subject.name_cn || subject.name,
      cover: imageFor(subject.image),
      meta: [subject.air_date, subject.rating > 0 ? `评分 ${subject.rating.toFixed(1)}` : "", subject.eps_count > 0 ? `${subject.eps_count} 话` : "连载"]
        .filter(Boolean)
        .join(" · "),
      kind: "subject",
      subject,
    };
  }

  function collectionItem(item: (typeof collection)[number]): HubItem {
    return {
      id: `collection:${item.key}`,
      title: item.name,
      cover: imageFor(item.image),
      meta: item.ruleSource || "已收藏",
      kind: "collection",
      collection: item,
    };
  }

  const lanes = $derived.by<HubLane[]>(() => {
    const result: HubLane[] = [];
    if (history.length) result.push({ id: "continue", kicker: "CONTINUE", title: "继续观看", hint: "A 打开 · ← → 浏览", items: history.map(historyItem) });
    if (currentTab === "calendar") {
      const calendarItems = (animeStore.calendar ?? []).flatMap((day) => day.items.map((item) => subjectItem(item, `calendar:${day.weekday}`))).slice(0, 24);
      if (calendarItems.length) result.push({ id: "calendar", kicker: "SCHEDULE", title: "一周放送", hint: "选择节目查看详情", items: calendarItems });
    } else {
      if (today.length) result.push({ id: "today", kicker: "TODAY", title: "今日放送", hint: "本地时间表", items: today.slice(0, 12).map((item) => subjectItem(item, "today")) });
      if (animeStore.recTrending.length) result.push({ id: "trending", kicker: "HOT", title: "热门节目", hint: "Bangumi 热度", items: animeStore.recTrending.slice(0, 12).map((item) => subjectItem(item, "hot")) });
      if (animeStore.recSeasonal.length) result.push({ id: "seasonal", kicker: "SEASON", title: "本季新番", hint: "正在播出", items: animeStore.recSeasonal.slice(0, 12).map((item) => subjectItem(item, "season")) });
      if (animeStore.recTopRated.length) result.push({ id: "top-rated", kicker: "RANK", title: "高分节目", hint: "评分优先", items: animeStore.recTopRated.slice(0, 12).map((item) => subjectItem(item, "rank")) });
    }
    if (currentTab === "my" && collection.length) result.push({ id: "collection", kicker: "LIBRARY", title: "我的收藏", hint: "追番档案", items: collection.map(collectionItem) });
    return result;
  });

  const lead = $derived.by<HubItem | null>(() => lanes[0]?.items[0] ?? null);
  const leadTitle = $derived(lead?.title ?? "从这里开始看番");
  const leadCover = $derived(lead?.cover ?? "");
  const leadMeta = $derived(lead?.meta ?? "搜索节目、选择来源，然后从第一集开始");
  const partialError = $derived(Boolean(animeStore.recError && (history.length || animeStore.recTrending.length || animeStore.recSeasonal.length || animeStore.recTopRated.length)));
  const loading = $derived(animeStore.loading || animeStore.recTrendingLoading || animeStore.recSeasonalLoading || animeStore.recTopRatedLoading);

  function setTab(tab: AnimeTab) {
    animeStore.setTab(tab);
    activeLane = 0;
    activeIndex = 0;
    requestAnimationFrame(() => focusCurrent());
  }

  function cycleTab(delta: number) {
    const current = Math.max(0, tabs.findIndex((tab) => tab.id === currentTab));
    const next = (current + delta + tabs.length) % tabs.length;
    if (animeStore.view === "search") animeStore.goHome();
    setTab(tabs[next].id);
  }

  async function submitSearch(event: Event) {
    event.preventDefault();
    const keyword = searchInput.trim();
    if (!keyword || searchBusy) return;
    searchBusy = true;
    try {
      await onSearch(keyword);
    } finally {
      searchBusy = false;
    }
  }

  function searchItemCover(key: string): string {
    return animeStore.getSearchCover(key);
  }

  function focusableModal(): HTMLElement[] {
    const root = document.activeElement?.closest("[role=dialog], .v2-detail-panel");
    if (!root) return [];
    return Array.from(root.querySelectorAll<HTMLElement>("button:not([disabled]), a[href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex='0']"));
  }

  function moveModalFocus(delta: number): boolean {
    const items = focusableModal();
    if (!items.length) return false;
    const current = Math.max(0, items.indexOf(document.activeElement as HTMLElement));
    items[(current + delta + items.length) % items.length]?.focus({ preventScroll: true });
    return true;
  }

  function focusCurrent() {
    if (animeStore.view !== "home" || animeStore.sourceSheetOpen) return;
    const lane = lanes[activeLane];
    const host = laneHosts[activeLane];
    host?.querySelector<HTMLElement>(`[data-lane-index="${activeIndex}"]`)?.focus({ preventScroll: true });
  }

  function moveItem(delta: number) {
    if (animeStore.view !== "home") {
      moveModalFocus(delta);
      return;
    }
    const lane = lanes[activeLane];
    if (!lane?.items.length) return;
    activeIndex = (activeIndex + delta + lane.items.length) % lane.items.length;
    requestAnimationFrame(() => focusCurrent());
  }

  function moveLane(delta: number) {
    if (animeStore.view !== "home") {
      moveModalFocus(delta);
      return;
    }
    if (!lanes.length) return;
    activeLane = (activeLane + delta + lanes.length) % lanes.length;
    activeIndex = Math.min(activeIndex, Math.max(0, lanes[activeLane].items.length - 1));
    requestAnimationFrame(() => focusCurrent());
  }

  function activateItem(item: HubItem | null = lanes[activeLane]?.items[activeIndex] ?? null, trigger?: HTMLElement) {
    if (!item) {
      if (currentTab === "rules") onOpenRules();
      return;
    }
    if (item.kind === "history") onResumeHistory(item.history, trigger);
    else if (item.kind === "subject") onOpenSubject(item.subject, trigger);
    else if (item.collection.ruleSource && item.collection.sourceUrl) {
      onOpenResult(item.collection.ruleSource, { name: item.collection.name, url: item.collection.sourceUrl }, trigger);
    } else onSearch(item.collection.name);
  }

  function activateFocused() {
    if (animeStore.view !== "home" || animeStore.sourceSheetOpen) {
      const active = document.activeElement as HTMLElement | null;
      if (active?.matches("button, a, [role=button]")) active.click();
      else moveModalFocus(1);
      return;
    }
    activateItem();
  }

  function back() {
    if (animeStore.sourceSheetOpen) {
      animeStore.sourceSheetOpen = false;
    } else if (animeStore.view === "detail") {
      animeStore.closeDetail();
    } else if (animeStore.view === "search") {
      animeStore.goHome();
    } else {
      onBack();
    }
  }

  function hubHandlers() {
    return {
      up: () => moveLane(-1),
      down: () => moveLane(1),
      left: () => moveItem(-1),
      right: () => moveItem(1),
      pageLeft: () => moveItem(-1),
      pageRight: () => moveItem(1),
      categoryLeft: () => animeStore.view === "home" ? cycleTab(-1) : moveModalFocus(-1),
      categoryRight: () => animeStore.view === "home" ? cycleTab(1) : moveModalFocus(1),
      launch: activateFocused,
      activate: () => animeStore.view === "home" ? onOpenRules() : moveModalFocus(1),
      filter: () => animeStore.view === "home" ? setTab("my") : moveModalFocus(1),
      back,
      start: () => animeStore.view === "home" ? onOpenRules() : moveModalFocus(1),
    };
  }

  $effect(() => {
    const nextLane = Math.min(activeLane, Math.max(0, lanes.length - 1));
    if (nextLane !== activeLane) activeLane = nextLane;
    const maxIndex = Math.max(0, (lanes[nextLane]?.items.length ?? 0) - 1);
    const nextIndex = Math.min(Math.max(0, Number.isFinite(activeIndex) ? activeIndex : 0), maxIndex);
    if (nextIndex !== activeIndex) activeIndex = nextIndex;
  });

  onMount(() => {
    pad = attachGamepad(hubHandlers(), { id: "handheld-anime-hub", zone: "content", priority: 95 });
    return () => {
      pad?.();
      pad = null;
    };
  });
</script>

<div class="handheld-anime-hub" data-testid="handheld-anime-hub">
  <div class="hah-toolbar">
    <div class="hah-tabs" role="tablist" aria-label="番剧掌机频道">
      {#each tabs as tab (tab.id)}
        <button class:active={currentTab === tab.id && animeStore.view !== "search"} type="button" role="tab" aria-selected={currentTab === tab.id && animeStore.view !== "search"} onclick={() => setTab(tab.id)}>
          <Icon name={tab.icon} size={14} />
          <span>{tab.label}</span>
          {#if tab.id === "my" && animeStore.history.length}<b>{animeStore.history.length}</b>{/if}
        </button>
      {/each}
    </div>
    <form class="hah-search" onsubmit={submitSearch}>
      <Icon name="search" size={14} />
      <input bind:value={searchInput} type="search" placeholder="搜索番剧…" aria-label="搜索番剧" />
      <button type="submit" disabled={!searchInput.trim() || searchBusy}>{searchBusy ? "搜索中" : "搜索"}</button>
    </form>
    <button class="hah-tool" type="button" onclick={onOpenRules}><Icon name="settings" size={14} />规则</button>
    <button class="hah-tool hah-provider" type="button" onclick={onOpenProvider}><Icon name="layers" size={14} />源工作台</button>
  </div>

  {#if animeStore.view === "search"}
    <section class="hah-search-view" aria-label="番剧搜索结果">
      <div class="hah-view-heading"><span>SEARCH / {animeStore.searchKeyword}</span><button type="button" onclick={() => animeStore.goHome()}>返回推荐</button></div>
      {#if animeStore.loading && searchResults.length === 0}
        <HandheldStatePanel state="loading" compact title="正在寻找节目" description="逐个检查可用来源，先返回的结果会优先显示。" />
      {:else if searchResults.length === 0}
        <HandheldStatePanel state={animeStore.error ? "error" : "empty"} compact title={animeStore.error ? "搜索没有完成" : "没有匹配节目"} description={animeStore.error || "换一个关键词，或到规则页检查来源。"} primaryAction={{ label: "重新搜索", run: () => void onSearch(animeStore.searchKeyword) }} secondaryAction={{ label: "打开规则", run: onOpenRules }} />
      {:else}
        <div class="hah-search-grid">
          {#each searchResults.slice(0, 24) as result (result.key)}
            <button class="hah-card" type="button" onclick={(event) => onOpenResult(result.sources[0], result.items[0], event.currentTarget)}>
              <span class="hah-card-art">{#if searchItemCover(result.key)}<img src={searchItemCover(result.key)} alt="" loading="lazy" />{:else}<span>{result.name.slice(0, 1)}</span>{/if}</span>
              <strong>{result.name}</strong>
              <small>{result.sources.length} 个来源</small>
            </button>
          {/each}
        </div>
      {/if}
    </section>
  {:else if currentTab === "rules"}
    <section class="hah-rules-view">
      <HandheldStatePanel state={animeStore.rules.length ? "ready" : "empty"} title={animeStore.rules.length ? `${animeStore.rules.length} 个播放来源已就绪` : "还没有播放来源"} description={animeStore.rules.length ? "按 A 进入推荐，按 Y 或 START 管理规则、验证码与来源健康状态。" : "安装或导入一个规则后，搜索与播放入口就会在这里恢复。"} primaryAction={{ label: animeStore.rules.length ? "返回推荐" : "管理规则", run: animeStore.rules.length ? () => setTab("recommend") : onOpenRules }} secondaryAction={{ label: "源工作台", run: onOpenProvider }} />
      {#if animeStore.rules.length}
        <div class="hah-rule-strip">{#each animeStore.rules.slice(0, 8) as rule (rule.name)}<span><i></i>{rule.name}</span>{/each}</div>
      {/if}
    </section>
  {:else}
    {#if partialError}
      <div class="hah-alert" role="status"><Icon name="info" size={14} /><span>{animeStore.recError}</span><button type="button" onclick={() => animeStore.refreshRecommendations()}>重试推荐</button></div>
    {/if}
    <section class="hah-lead" aria-labelledby="hah-lead-title">
      <div class="hah-lead-art" class:empty={!leadCover}>
        {#if leadCover}<img src={leadCover} alt="" loading="eager" />{:else}<div><span>ANIME</span><b>PLAY<br />NEXT</b></div>{/if}
        <span class="hah-lead-art-shade"></span>
        <span class="hah-lead-art-tag">MOEPLAY / ANIME</span>
      </div>
      <div class="hah-lead-copy">
        <span class="hah-eyebrow"><b>{lead?.kind === "history" ? "CONTINUE WATCHING" : "HANDHELD ANIME CENTER"}</b><i></i><small>横屏影院</small></span>
        <h1 id="hah-lead-title">{leadTitle}</h1>
        <p>{lead?.kind === "subject" ? (lead.subject.summary || "从节目详情、播放来源与剧集列表进入观看。") : "搜索、收藏和播放记录集中在同一个横屏入口，手柄和触控都能直接继续。"}</p>
        <div class="hah-lead-meta"><span>{leadMeta}</span>{#if history.length}<span>历史 {history.length} 条</span>{/if}{#if animeStore.rules.length}<span>来源 {animeStore.rules.length}</span>{/if}</div>
        <div class="hah-lead-actions">
          <button class="hah-primary" type="button" onclick={(event) => activateItem(lead, event.currentTarget)} disabled={!lead && loading}><Icon name="play" size={16} />{lead?.kind === "history" ? "继续观看" : lead ? "查看节目" : "等待数据"}<b>A</b></button>
          <button type="button" onclick={onOpenRules}><Icon name="settings" size={15} />管理来源</button>
          <button type="button" onclick={onOpenProvider}><Icon name="layers" size={15} />多源工作台</button>
        </div>
      </div>
      <aside class="hah-lead-status"><span>NOW / READY</span><strong>{String(history.length).padStart(2, "0")}</strong><small>WATCH RECORDS</small><i></i><small>↑ ↓ 切换轨道</small></aside>
    </section>

    {#if lanes.length}
      <div class="hah-lanes" aria-label="番剧内容轨道">
        {#each lanes as lane, laneIndex (lane.id)}
          <section class="hah-lane" aria-label={lane.title} class:is-current={activeLane === laneIndex}>
            <header><span>{lane.kicker}</span><h2>{lane.title}</h2><small>{lane.hint}</small></header>
            <div class="hah-lane-track" bind:this={laneHosts[laneIndex]} role="listbox" aria-label={lane.title}>
              {#each lane.items as item, itemIndex (item.id)}
                <button class="hah-card" class:focused={activeLane === laneIndex && activeIndex === itemIndex} type="button" role="option" aria-selected={activeLane === laneIndex && activeIndex === itemIndex} data-lane-index={itemIndex} onclick={(event) => activateItem(item, event.currentTarget)}>
                  <span class="hah-card-art">{#if item.cover}<img src={item.cover} alt="" loading="lazy" />{:else}<span>{item.title.slice(0, 1)}</span>{/if}<i></i></span>
                  <strong>{item.title}</strong>
                  <small>{item.meta || "番剧节目"}</small>
                </button>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    {:else if loading}
      <HandheldStatePanel state="loading" compact title="正在装配节目轨道" description="推荐、时间表和来源状态会逐步出现。" />
    {:else}
      <HandheldStatePanel state="empty" compact title="暂无番剧内容" description="搜索一个节目，或先在规则页安装可用来源。" primaryAction={{ label: "搜索节目", run: () => document.querySelector<HTMLInputElement>(".hah-search input")?.focus() }} secondaryAction={{ label: "打开规则", run: onOpenRules }} />
    {/if}
  {/if}
</div>

<style>
  .handheld-anime-hub { --hah-accent: #ef6d86; --hah-line: rgba(255,255,255,.13); position: relative; display: flex; flex-direction: column; gap: 8px; height: 100%; min-height: 0; padding: 8px max(14px, env(safe-area-inset-left)) 8px max(14px, env(safe-area-inset-right)); overflow: hidden; color: #f6f1f2; background: linear-gradient(125deg, rgba(13, 12, 20, .84), rgba(25, 13, 23, .64)); }
  .hah-toolbar { display: flex; align-items: center; gap: 6px; min-height: 40px; flex: 0 0 auto; }
  .hah-tabs { display: flex; min-width: 0; gap: 4px; }
  .hah-tabs button, .hah-tool, .hah-search button, .hah-view-heading button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 38px; border: 1px solid var(--hah-line); background: rgba(255,255,255,.045); color: rgba(255,255,255,.65); font: 700 11px/1 var(--font-ui, system-ui); cursor: pointer; }
  .hah-tabs button { min-width: 72px; padding: 0 9px; }
  .hah-tabs button.active { border-color: var(--hah-accent); background: rgba(239,109,134,.18); color: #fff; box-shadow: inset 0 -2px var(--hah-accent); }
  .hah-tabs button b { min-width: 16px; padding: 3px 4px; border-radius: 4px; background: rgba(255,255,255,.1); color: #fff; font: 800 9px/1 var(--font-mono, monospace); }
  .hah-search { display: flex; align-items: center; gap: 7px; min-width: 0; flex: 1; height: 38px; padding-left: 10px; border: 1px solid var(--hah-line); background: rgba(5,6,10,.55); color: rgba(255,255,255,.5); }
  .hah-search input { min-width: 0; flex: 1; height: 100%; border: 0; outline: 0; background: transparent; color: #fff; font: 600 11px/1 var(--font-ui, system-ui); }
  .hah-search input::placeholder { color: rgba(255,255,255,.4); }
  .hah-search button { min-width: 54px; align-self: stretch; border-width: 0 0 0 1px; border-color: var(--hah-line); background: var(--hah-accent); color: #16090d; }
  .hah-search button:disabled { opacity: .45; }
  .hah-tool { padding: 0 10px; white-space: nowrap; }
  .hah-tool:hover, .hah-tabs button:hover, .hah-view-heading button:hover { border-color: var(--hah-accent); color: #fff; }
  .hah-lead { display: grid; grid-template-columns: minmax(150px, 24%) minmax(0, 1fr) 112px; gap: 16px; min-height: 150px; flex: 0 0 min(42%, 178px); overflow: hidden; border: 1px solid var(--hah-line); background: linear-gradient(110deg, rgba(10,10,17,.88), rgba(68,25,42,.35)); box-shadow: 0 18px 40px rgba(0,0,0,.22); }
  .hah-lead-art { position: relative; min-width: 0; overflow: hidden; background: linear-gradient(140deg, #3c1c2c, #0c0e15); }
  .hah-lead-art img { width: 100%; height: 100%; display: block; object-fit: cover; object-position: center 25%; filter: saturate(.86) contrast(1.06); }
  .hah-lead-art.empty div { display: grid; align-content: space-between; height: 100%; padding: 20px; color: rgba(255,255,255,.48); font: 700 9px/1 var(--font-mono, monospace); letter-spacing: .16em; }
  .hah-lead-art.empty b { color: rgba(255,255,255,.2); font: 900 3.2rem/.8 var(--font-display, system-ui); letter-spacing: -.08em; }
  .hah-lead-art-shade { position: absolute; inset: 0; background: linear-gradient(90deg, rgba(5,6,10,.06), rgba(5,6,10,.72)), linear-gradient(0deg, rgba(5,6,10,.65), transparent 64%); pointer-events: none; }
  .hah-lead-art-tag { position: absolute; right: 10px; bottom: 9px; color: rgba(255,255,255,.72); font: 700 8px/1 var(--font-mono, monospace); letter-spacing: .11em; }
  .hah-lead-copy { display: flex; min-width: 0; flex-direction: column; justify-content: center; gap: 8px; padding: 12px 0; }
  .hah-eyebrow { display: flex; align-items: center; gap: 8px; color: var(--hah-accent); font: 800 9px/1 var(--font-mono, monospace); letter-spacing: .12em; }
  .hah-eyebrow i { width: 22px; height: 1px; background: var(--hah-accent); }
  .hah-eyebrow small { color: rgba(255,255,255,.45); letter-spacing: .05em; }
  .hah-lead-copy h1 { max-width: 18ch; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font: 850 clamp(1.35rem, 3vw, 2.5rem)/1 var(--font-display, system-ui); letter-spacing: -.06em; }
  .hah-lead-copy p { max-width: 56ch; margin: 0; overflow: hidden; color: rgba(255,255,255,.58); font-size: 11px; line-height: 1.5; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .hah-lead-meta { display: flex; flex-wrap: wrap; gap: 6px; color: rgba(255,255,255,.55); font: 700 9px/1.25 var(--font-mono, monospace); }
  .hah-lead-meta span + span { padding-left: 7px; border-left: 1px solid var(--hah-line); }
  .hah-lead-actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .hah-lead-actions button { display: inline-flex; align-items: center; gap: 6px; min-height: 34px; padding: 0 10px; border: 1px solid var(--hah-line); background: rgba(255,255,255,.055); color: rgba(255,255,255,.78); font: 750 10px/1 var(--font-ui, system-ui); cursor: pointer; }
  .hah-lead-actions button:hover, .hah-lead-actions button:focus-visible { border-color: var(--hah-accent); color: #fff; }
  .hah-lead-actions .hah-primary { border-color: var(--hah-accent); background: var(--hah-accent); color: #19090e; }
  .hah-lead-actions button b { display: grid; place-items: center; min-width: 17px; height: 17px; margin-left: 3px; border: 1px solid currentColor; border-radius: 50%; font: 800 8px/1 var(--font-mono, monospace); }
  .hah-lead-status { display: grid; align-content: center; justify-items: end; gap: 5px; padding: 12px; border-left: 1px solid var(--hah-line); text-align: right; }
  .hah-lead-status span, .hah-lead-status small { color: rgba(255,255,255,.44); font: 700 8px/1.25 var(--font-mono, monospace); letter-spacing: .1em; }
  .hah-lead-status strong { color: var(--hah-accent); font: 900 2.5rem/.8 var(--font-display, system-ui); }
  .hah-lead-status i { width: 30px; height: 2px; background: var(--hah-accent); }
  .hah-lanes { display: flex; flex-direction: column; gap: 8px; min-height: 0; flex: 1; overflow-x: hidden; overflow-y: auto; scrollbar-width: thin; scrollbar-color: rgba(239,109,134,.5) transparent; }
  .hah-lane { min-height: 84px; flex: 0 0 clamp(84px, 17dvh, 118px); overflow: hidden; }
  .hah-lane header { display: flex; align-items: baseline; gap: 8px; min-height: 22px; border-bottom: 1px solid var(--hah-line); }
  .hah-lane header span { color: var(--hah-accent); font: 800 8px/1 var(--font-mono, monospace); letter-spacing: .14em; }
  .hah-lane header h2 { margin: 0; font: 800 13px/1 var(--font-display, system-ui); }
  .hah-lane header small { margin-left: auto; color: rgba(255,255,255,.38); font: 700 8px/1 var(--font-mono, monospace); }
  .hah-lane-track { display: flex; gap: 9px; height: calc(100% - 22px); padding: 7px 2px 2px; overflow-x: auto; scrollbar-width: none; }
  .hah-lane-track::-webkit-scrollbar { display: none; }
  .hah-card { display: grid; flex: 0 0 clamp(78px, 10vw, 108px); align-content: start; gap: 4px; min-width: 0; padding: 0; border: 0; background: transparent; color: rgba(255,255,255,.65); text-align: left; cursor: pointer; }
  .hah-card-art { position: relative; display: grid; place-items: center; aspect-ratio: 16 / 10; overflow: hidden; border: 1px solid transparent; background: linear-gradient(140deg, rgba(255,255,255,.1), rgba(0,0,0,.35)); color: rgba(255,255,255,.28); font: 900 1.5rem/1 var(--font-display, system-ui); }
  .hah-card-art img { width: 100%; height: 100%; display: block; object-fit: cover; }
  .hah-card-art i { position: absolute; right: 4px; bottom: 4px; left: 4px; height: 2px; background: var(--hah-accent); opacity: .9; }
  .hah-card strong, .hah-card small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hah-card strong { font-size: 10px; line-height: 1.2; }
  .hah-card small { color: rgba(255,255,255,.4); font: 700 8px/1.2 var(--font-mono, monospace); }
  .hah-card:hover .hah-card-art, .hah-card:focus-visible .hah-card-art, .hah-card.focused .hah-card-art { border-color: var(--hah-accent); box-shadow: 0 0 0 2px rgba(239,109,134,.2), 0 8px 18px rgba(0,0,0,.28); }
  .hah-card.focused { color: #fff; transform: translateY(-2px); }
  .hah-alert { display: flex; align-items: center; gap: 8px; min-height: 30px; padding: 0 9px; border: 1px solid rgba(244,161,91,.35); background: rgba(111,61,36,.3); color: rgba(255,236,217,.8); font-size: 10px; }
  .hah-alert span { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hah-alert button { min-height: 24px; padding: 0 8px; border: 1px solid rgba(255,255,255,.25); background: transparent; color: #fff; cursor: pointer; font-size: 9px; }
  .hah-search-view, .hah-rules-view { min-height: 0; flex: 1; overflow: auto; }
  .hah-view-heading { display: flex; align-items: center; justify-content: space-between; min-height: 28px; margin-bottom: 8px; border-bottom: 1px solid var(--hah-line); color: var(--hah-accent); font: 800 9px/1 var(--font-mono, monospace); letter-spacing: .1em; }
  .hah-view-heading button { min-height: 26px; padding: 0 8px; font-size: 9px; }
  .hah-search-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(82px, 1fr)); gap: 12px 8px; }
  .hah-search-grid .hah-card { width: 100%; }
  .hah-search-grid .hah-card-art { aspect-ratio: 3 / 4; }
  .hah-rules-view { display: grid; align-content: center; gap: 12px; }
  .hah-rule-strip { display: flex; flex-wrap: wrap; gap: 7px; }
  .hah-rule-strip span { display: inline-flex; align-items: center; gap: 6px; min-height: 28px; padding: 0 8px; border: 1px solid var(--hah-line); color: rgba(255,255,255,.62); font: 700 9px/1 var(--font-mono, monospace); }
  .hah-rule-strip i { width: 6px; height: 6px; border-radius: 50%; background: #62d99b; box-shadow: 0 0 9px rgba(98,217,155,.6); }
  :global(.handheld-anime-hub .hh-state-panel) { min-height: 130px; border-color: var(--hah-line); background: linear-gradient(110deg, rgba(12,13,19,.9), rgba(72,27,43,.32)); }
  :global(.handheld-anime-hub .hh-state-panel__art img) { max-height: 98px; }
  button:focus-visible { outline: 2px solid var(--hah-accent); outline-offset: 2px; }
  @media (max-height: 560px) and (orientation: landscape) {
    .handheld-anime-hub { gap: 5px; padding-block: 5px; }
    .hah-toolbar { min-height: 34px; }
    .hah-tabs button, .hah-tool, .hah-search, .hah-search button { min-height: 32px; height: 32px; }
    .hah-tabs button { min-width: 62px; }
    .hah-lead { grid-template-columns: 108px minmax(0,1fr) 90px; min-height: 122px; flex-basis: min(42%, 154px); gap: 11px; }
    .hah-lead-copy { gap: 5px; }
    .hah-lead-copy h1 { font-size: 1.45rem; }
    .hah-lead-copy p { font-size: 9px; -webkit-line-clamp: 1; line-clamp: 1; }
    .hah-lead-status strong { font-size: 2rem; }
    .hah-lead-actions button { min-height: 28px; font-size: 9px; padding-inline: 7px; }
    .hah-lanes { gap: 4px; }
    .hah-lane header { min-height: 18px; }
    .hah-lane header h2 { font-size: 11px; }
    .hah-lane { flex-basis: 78px; min-height: 78px; }
    .hah-lane-track { height: calc(100% - 18px); padding-top: 4px; gap: 7px; }
    .hah-card { flex-basis: 76px; gap: 3px; }
    .hah-card strong { font-size: 8px; }
    .hah-card small { font-size: 7px; }
  }
  @media (max-width: 690px) {
    .hah-provider { display: none; }
    .hah-tool { padding-inline: 7px; }
    .hah-tabs button { min-width: 58px; padding-inline: 5px; }
    .hah-tabs button span { display: none; }
    .hah-lead { grid-template-columns: 92px minmax(0, 1fr); }
    .hah-lead-status { display: none; }
  }
  @media (prefers-reduced-motion: reduce) {
    .hah-card.focused { transform: none; }
  }
</style>
