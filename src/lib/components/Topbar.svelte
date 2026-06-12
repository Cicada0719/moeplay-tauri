<script lang="ts">
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";

  let searchInput = $state("");

  /// Debounced pinyin-aware search
  let debounceTimer: ReturnType<typeof setTimeout>;
  function handleSearch(e: Event) {
    const raw = (e.target as HTMLInputElement).value;
    searchInput = raw;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      if (!raw.trim()) { gameStore.searchQuery = ""; return; }
      gameStore.searchQuery = raw;
    }, 200);
  }

  function clearSearch() {
    searchInput = "";
    gameStore.searchQuery = "";
  }

  const filterLabels: Record<string, string> = {
    favorite: "收藏", completed: "已通关", unplayed: "未玩",
    installed: "已安装", missing_metadata: "待刮削", recent: "最近游玩",
  };

  const viewModes = [
    { id: "grid", label: "网格", icon: "collection" },
    { id: "list", label: "列表", icon: "paperclip" },
    { id: "compact", label: "紧凑", icon: "diamond" },
  ] as const;

  function handleImport() { gameStore.importGame(); }
</script>

<header class="topbar">
  <div class="left">
    <button class="icon-btn" onclick={() => uiStore.sidebarCollapsed = !uiStore.sidebarCollapsed} title="切换侧边栏">
      <Icon name="paperclip" size={20} />
    </button>

    <div class="search-box">
      <Icon name="search" size={16} className="search-icon" />
      <input
        type="text"
        placeholder="搜索游戏... (支持拼音)"
        value={searchInput}
        oninput={handleSearch}
        class="search-input"
      />
      {#if searchInput}
        <button class="clear-btn" onclick={clearSearch}><Icon name="x" size={14} /></button>
      {/if}
    </div>

    {#if gameStore.quickFilter}
      <div class="filter-chip">
        <Icon name="tag" size={12} />
        <span>{filterLabels[gameStore.quickFilter] ?? gameStore.quickFilter}</span>
        <button class="filter-clear" onclick={() => gameStore.quickFilter = null} aria-label="清除筛选">
          <Icon name="x" size={11} />
        </button>
      </div>
    {/if}
  </div>

  <div class="right">
    <div class="view-toggle">
      {#each viewModes as mode}
        <button
          class="view-btn"
          class:active={uiStore.viewMode === mode.id}
          onclick={() => uiStore.viewMode = mode.id}
          title={mode.label}
        >
          <Icon name={mode.icon} size={16} />
        </button>
      {/each}
    </div>

    <button class="btn-add" onclick={handleImport}>
      <Icon name="plus" size={18} />
      <span>添加游戏</span>
    </button>
  </div>
</header>

<style>
  .topbar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 20px; gap: 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }
  .left { display: flex; align-items: center; gap: 12px; flex: 1; }
  .right { display: flex; align-items: center; gap: 12px; }

  .icon-btn {
    background: none; border: none; cursor: pointer; color: var(--text-secondary);
    padding: 6px; border-radius: var(--radius-sm); display: flex;
    transition: all 0.2s;
  }
  .icon-btn:hover { color: var(--text-primary); background: var(--bg-hover); }

  .search-box {
    display: flex; align-items: center; gap: 8px;
    background: var(--bg-card); border: 1px solid var(--border);
    border-radius: var(--radius-full); padding: 8px 16px;
    flex: 1; max-width: 480px; transition: border-color 0.2s;
  }
  .search-box:focus-within { border-color: var(--accent); }
  .search-box :global(.search-icon) { color: var(--text-muted); flex-shrink: 0; }
  .search-input {
    flex: 1; border: none; background: transparent; color: var(--text-primary);
    font-size: 0.9rem; outline: none; font-family: var(--font-ui);
  }
  .clear-btn {
    background: none; border: none; cursor: pointer; color: var(--text-muted);
    padding: 2px; border-radius: 50%; display: flex;
  }
  .clear-btn:hover { color: var(--text-primary); }

  .view-toggle { display: flex; gap: 2px; background: var(--bg-card); border-radius: var(--radius-md); padding: 2px; }
  .view-btn {
    background: none; border: none; cursor: pointer; padding: 6px 8px;
    border-radius: var(--radius-sm); color: var(--text-muted); display: flex;
    transition: all 0.15s;
  }
  .view-btn.active, .view-btn:hover { color: var(--text-primary); }
  .view-btn.active { background: var(--bg-hover); }

  .btn-add {
    display: flex; align-items: center; gap: 6px;
    padding: 8px 16px; border: none; border-radius: var(--radius-full);
    background: var(--accent); color: #fff; font-weight: 600;
    cursor: pointer; font-size: 0.85rem; transition: transform 0.12s ease, background 0.18s ease;
  }
  .btn-add:hover { background: var(--accent-hi); }
  .btn-add:active { transform: translateY(1px); }

  .filter-chip {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 5px 10px; border-radius: var(--radius-full);
    border: 1px solid var(--accent-ring); background: var(--accent-lo);
    color: var(--accent); font-size: 12px; font-weight: 550;
    white-space: nowrap;
  }
  .filter-clear {
    background: none; border: none; cursor: pointer; color: var(--accent);
    padding: 1px; display: flex;
  }
  .filter-clear:hover { color: var(--accent-hi); }
</style>
