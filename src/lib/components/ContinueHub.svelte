<script lang="ts">
  import { gsap } from "gsap";
  import { continueStore, type ContinueItem } from "../stores/continue.svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import ContinueCard from "./ContinueCard.svelte";
  import Icon from "./Icon.svelte";
  import Card from "./ui/Card.svelte";
  import StatBlock from "./ui/StatBlock.svelte";
  import SegmentControl from "./ui/SegmentControl.svelte";
  import EmptyState from "./ui/EmptyState.svelte";
  import Button from "./ui/Button.svelte";

  let filter = $state<"all" | "game" | "anime" | "comic">("all");
  let containerEl = $state<HTMLDivElement>();
  let animated = $state(false);

  const stats = $derived(continueStore.stats);
  const topItem = $derived(continueStore.topItem);

  const filteredItems = $derived(
    filter === "all" ? continueStore.items : continueStore.items.filter(i => i.type === filter)
  );
  const gameItems = $derived(continueStore.games);
  const animeItems = $derived(continueStore.anime);
  const comicItems = $derived(continueStore.comics);

  const filterOptions = $derived([
    { value: "all", label: `全部 (${stats.totalCount})` },
    { value: "game", label: `游戏 (${stats.gameCount})` },
    { value: "anime", label: `番剧 (${stats.animeCount})` },
    { value: "comic", label: `漫画 (${stats.comicCount})` },
  ]);

  function handleSelect(item: ContinueItem) {
    if (item.type === "game") {
      const gameId = item.id.replace("game-", "");
      gameStore.selectGame(gameId);
      uiStore.currentView = "game-detail";
    } else if (item.type === "anime") {
      uiStore.currentView = "anime";
    } else {
      uiStore.currentView = "comic";
    }
  }

  function goTo(view: string) {
    uiStore.currentView = view;
  }

  function animateIn() {
    if (!containerEl) return;
    gsap.from(containerEl.querySelectorAll(".cc-card, .ui-stat, .top-card"), {
      opacity: 0, y: 12, duration: 0.35, ease: "power3.out", stagger: 0.03,
    });
  }

  $effect(() => {
    if (!animated && containerEl) {
      animated = true;
      queueMicrotask(animateIn);
    }
  });
</script>

<div class="hub" bind:this={containerEl}>
  <header class="hub-header">
    <div class="hub-title">
      <Icon name="play" size={24} />
      <h1>今日中枢</h1>
      <span class="hub-count">{stats.totalCount} 项进行中</span>
    </div>
  </header>

  <!-- Stats Hero -->
  <section class="stats-grid" aria-label="今日概览">
    <StatBlock label="今日活跃" value={stats.todayMinutes} unit="m" />
    <StatBlock label="本周活跃" value={stats.weekMinutes} unit="m" />
    <StatBlock label="连续活跃天" value={stats.streakDays} />
    <StatBlock label="游戏" value={stats.gameCount} />
    <StatBlock label="番剧" value={stats.animeCount} />
    <StatBlock label="漫画" value={stats.comicCount} />
  </section>

  <!-- Top Continue Action -->
  {#if topItem}
    <section class="top-section" aria-label="继续">
      <h2 class="section-title">继续</h2>
      <Card class="top-card" hoverable focusable onclick={() => handleSelect(topItem)} ariaLabel={`继续 ${topItem.title}`}>
        <div class="top-cover">
          {#if topItem.cover}
            <img src={topItem.cover} alt={topItem.title} />
          {:else}
            <div class="top-placeholder">{topItem.title[0]}</div>
          {/if}
        </div>
        <div class="top-info">
          <span class="top-meta">{topItem.actionLabel || "继续"}</span>
          <span class="top-title">{topItem.title}</span>
          {#if topItem.subtitle}
            <span class="top-subtitle">{topItem.subtitle}</span>
          {/if}
          {#if topItem.progress > 0}
            <div class="top-progress">
              <div class="top-progress-bar" style="width: {topItem.progress}%"></div>
            </div>
            <span class="top-progress-label">{topItem.progressLabel}</span>
          {/if}
        </div>
        <div class="top-action" aria-hidden="true">
          <Icon name="play" size={20} />
        </div>
      </Card>
    </section>
  {/if}

  <!-- Filters -->
  <nav class="hub-filters" aria-label="类型筛选">
    <SegmentControl options={filterOptions} value={filter} onChange={(v) => (filter = v as any)} size="sm" />
  </nav>

  <!-- Content -->
  <div class="hub-content">
    {#if filteredItems.length > 0}
      {#if filter === "all"}
        {#if gameItems.length > 0}
          <section class="hub-section">
            <h3 class="hub-section-title">最近在玩</h3>
            <div class="hub-list">
              {#each gameItems.slice(0, 6) as item (item.id)}
                <ContinueCard {item} onclick={() => handleSelect(item)} />
              {/each}
            </div>
          </section>
        {/if}
        {#if animeItems.length > 0}
          <section class="hub-section">
            <h3 class="hub-section-title">最近在看</h3>
            <div class="hub-list">
              {#each animeItems.slice(0, 6) as item (item.id)}
                <ContinueCard {item} onclick={() => handleSelect(item)} />
              {/each}
            </div>
          </section>
        {/if}
        {#if comicItems.length > 0}
          <section class="hub-section">
            <h3 class="hub-section-title">最近在读</h3>
            <div class="hub-list">
              {#each comicItems.slice(0, 6) as item (item.id)}
                <ContinueCard {item} onclick={() => handleSelect(item)} />
              {/each}
            </div>
          </section>
        {/if}
      {:else}
        <section class="hub-section">
          <div class="hub-list">
            {#each filteredItems as item (item.id)}
              <ContinueCard {item} onclick={() => handleSelect(item)} />
            {/each}
          </div>
        </section>
      {/if}
    {:else}
      <EmptyState
        icon="play"
        title="还没有进行中内容"
        description="导入本地游戏、安装番剧规则源或登录哔咔，这里会自动聚合你的进度。"
      />
      <div class="hub-empty-actions">
        <Button variant="secondary" size="sm" onclick={() => goTo("steam-import")}>导入游戏</Button>
        <Button variant="secondary" size="sm" onclick={() => goTo("anime")}>去追番</Button>
        <Button variant="secondary" size="sm" onclick={() => goTo("comic")}>去看漫</Button>
      </div>
    {/if}
  </div>
</div>

<style>
  .hub {
    padding: 28px 32px;
    max-width: 1200px;
    margin: 0 auto;
    overflow-y: auto;
  }
  .hub-header { margin-bottom: 20px; }
  .hub-title {
    display: flex; align-items: center; gap: 10px;
  }
  .hub-title h1 { margin: 0; font-size: 26px; font-weight: 800; color: var(--text-primary); }
  .hub-count { font-size: 14px; color: var(--text-muted); margin-left: 4px; }

  /* Stats grid */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 10px;
    margin-bottom: 24px;
  }

  /* Top card */
  .top-section { margin-bottom: 24px; }
  .section-title {
    margin: 0 0 10px; font-size: 15px; font-weight: 700; color: var(--text-primary);
  }
  :global(.top-card) {
    display: flex; align-items: center; gap: 16px;
    width: 100%; text-align: left;
    padding: 16px;
    background: linear-gradient(135deg, rgba(232,85,127,0.08), rgba(255,255,255,0.02));
  }
  :global(.top-card):hover {
    border-color: var(--accent);
    background: linear-gradient(135deg, rgba(232,85,127,0.12), rgba(255,255,255,0.03));
  }
  .top-cover {
    width: 72px; height: 96px; flex-shrink: 0;
    border-radius: 10px; overflow: hidden;
    background: rgba(255,255,255,0.06);
  }
  .top-cover img { width: 100%; height: 100%; object-fit: cover; }
  .top-placeholder {
    width: 100%; height: 100%;
    display: flex; align-items: center; justify-content: center;
    font-size: 28px; font-weight: 700; color: var(--text-muted);
  }
  .top-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .top-meta { font-size: 12px; color: var(--accent); font-weight: 700; }
  .top-title { font-size: 18px; font-weight: 700; color: var(--text-primary); }
  .top-subtitle { font-size: 13px; color: var(--text-secondary); }
  .top-progress {
    height: 4px; width: 100%; max-width: 240px;
    background: rgba(255,255,255,0.1); border-radius: 2px; margin-top: 4px;
  }
  .top-progress-bar {
    height: 100%; background: var(--accent); border-radius: 2px;
  }
  .top-progress-label { font-size: 12px; color: var(--text-muted); margin-top: 2px; }
  .top-action {
    width: 44px; height: 44px; flex-shrink: 0;
    display: grid; place-items: center;
    border-radius: 50%;
    background: var(--accent); color: #fff;
  }

  /* Filters */
  .hub-filters {
    display: flex; gap: 6px; flex-wrap: wrap;
    margin-bottom: 20px;
  }

  /* Content */
  .hub-content { min-height: 200px; }
  .hub-section { margin-bottom: 22px; }
  .hub-section-title {
    margin: 0 0 10px; font-size: 15px; font-weight: 700; color: var(--text-primary);
  }
  .hub-list {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }

  /* Empty */
  .hub-empty-actions {
    display: flex; gap: 10px; justify-content: center; flex-wrap: wrap;
    margin-top: 16px;
  }

  @media (max-width: 900px) {
    .stats-grid { grid-template-columns: repeat(3, 1fr); }
    .hub-list { grid-template-columns: 1fr; }
  }
  @media (max-width: 560px) {
    .hub { padding: 20px 16px; }
    .stats-grid { grid-template-columns: repeat(2, 1fr); }
    :global(.top-card) { gap: 12px; padding: 12px; }
    .top-cover { width: 56px; height: 75px; }
    .top-title { font-size: 15px; }
  }
</style>
