<script lang="ts">
  import { gsap } from "gsap";
  import { continueStore } from "../stores/continue.svelte";
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import ContinueCard from "./ContinueCard.svelte";
  import Icon from "./Icon.svelte";

  let filter = $state<"all" | "game" | "anime" | "comic">("all");
  let containerEl = $state<HTMLDivElement>();

  const filteredItems = $derived(
    filter === "all" ? continueStore.items : continueStore.items.filter(i => i.type === filter)
  );
  const gameItems = $derived(continueStore.games);
  const animeItems = $derived(continueStore.anime);
  const comicItems = $derived(continueStore.comics);

  function handleSelect(item: { type: string; id: string }) {
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

  function animateIn() {
    if (!containerEl) return;
    gsap.from(containerEl.querySelectorAll(".cc-card"), {
      opacity: 0, y: 12, duration: 0.35, ease: "power3.out", stagger: 0.03,
    });
  }

  $effect(() => {
    filteredItems; // trigger on change
    queueMicrotask(animateIn);
  });
</script>

<div class="hub" bind:this={containerEl}>
  <header class="hub-header">
    <div class="hub-title">
      <Icon name="play" size={24} />
      <h1>继续</h1>
      <span class="hub-count">{continueStore.totalCount} 项进行中</span>
    </div>
    <nav class="hub-filters">
      <button class:active={filter === "all"} onclick={() => (filter = "all")}>
        全部 <span class="hub-badge">{continueStore.totalCount}</span>
      </button>
      <button class:active={filter === "game"} onclick={() => (filter = "game")}>
        🎮 游戏 <span class="hub-badge">{gameItems.length}</span>
      </button>
      <button class:active={filter === "anime"} onclick={() => (filter = "anime")}>
        📺 番剧 <span class="hub-badge">{animeItems.length}</span>
      </button>
      <button class:active={filter === "comic"} onclick={() => (filter = "comic")}>
        📖 漫画 <span class="hub-badge">{comicItems.length}</span>
      </button>
    </nav>
  </header>

  <div class="hub-content">
    {#if filteredItems.length > 0}
      {#if filter === "all"}
        {#if gameItems.length > 0}
          <section class="hub-section">
            <h3 class="hub-section-title">🎮 最近在玩</h3>
            <div class="hub-list">
              {#each gameItems.slice(0, 6) as item (item.id)}
                <ContinueCard {item} onclick={() => handleSelect(item)} />
              {/each}
            </div>
          </section>
        {/if}
        {#if animeItems.length > 0}
          <section class="hub-section">
            <h3 class="hub-section-title">📺 最近在看</h3>
            <div class="hub-list">
              {#each animeItems.slice(0, 6) as item (item.id)}
                <ContinueCard {item} onclick={() => handleSelect(item)} />
              {/each}
            </div>
          </section>
        {/if}
        {#if comicItems.length > 0}
          <section class="hub-section">
            <h3 class="hub-section-title">📖 最近在读</h3>
            <div class="hub-list">
              {#each comicItems.slice(0, 6) as item (item.id)}
                <ContinueCard {item} onclick={() => handleSelect(item)} />
              {/each}
            </div>
          </section>
        {/if}
      {:else}
        <div class="hub-list">
          {#each filteredItems as item (item.id)}
            <ContinueCard {item} onclick={() => handleSelect(item)} />
          {/each}
        </div>
      {/if}
    {:else}
      <div class="hub-empty">
        <Icon name="play" size={48} />
        <h3>没有正在进行的内容</h3>
        <p>开始玩游戏、追番或看漫画，这里会自动聚合你的进度。</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .hub {
    max-width: 800px; margin: 0 auto; padding: 32px 24px;
  }
  .hub-header { margin-bottom: 28px; }
  .hub-title {
    display: flex; align-items: center; gap: 10px; margin-bottom: 16px;
  }
  .hub-title h1 { margin: 0; font-size: 28px; font-weight: 800; color: var(--text-primary); }
  .hub-count { font-size: 14px; color: var(--text-muted); margin-left: 4px; }

  .hub-filters {
    display: flex; gap: 6px; flex-wrap: wrap;
  }
  .hub-filters button {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 7px 14px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.03); color: var(--text-secondary);
    font-size: 13px; cursor: pointer; transition: all 0.15s;
  }
  .hub-filters button:hover { border-color: var(--accent); color: var(--accent); }
  .hub-filters button.active {
    background: var(--accent); border-color: var(--accent); color: #fff;
  }
  .hub-badge {
    font-size: 11px; padding: 1px 6px; border-radius: 10px;
    background: rgba(255,255,255,0.1);
  }
  .hub-filters button.active .hub-badge { background: rgba(255,255,255,0.25); }

  .hub-content { min-height: 200px; }
  .hub-section { margin-bottom: 24px; }
  .hub-section-title {
    margin: 0 0 10px; font-size: 15px; font-weight: 700; color: var(--text-primary);
  }
  .hub-list { display: flex; flex-direction: column; gap: 6px; }

  .hub-empty {
    text-align: center; padding: 60px 20px; color: var(--text-muted);
  }
  .hub-empty h3 { margin: 16px 0 8px; font-size: 18px; color: var(--text-secondary); }
  .hub-empty p { font-size: 14px; }
</style>
