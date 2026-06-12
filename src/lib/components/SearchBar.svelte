<script lang="ts">
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";
</script>

<header class="search-header">
  <div class="search-box">
    <Icon name="search" size={16} className="search-icon" />
    <input
      type="text"
      placeholder="搜索游戏名称或标签..."
      bind:value={gameStore.searchQuery}
    />
    {#if gameStore.searchQuery}
      <button class="clear-btn" onclick={() => (gameStore.searchQuery = "")}>
        <Icon name="x" size={12} />
      </button>
    {/if}
  </div>

  <div class="header-actions">
    <select class="sort-select" value={uiStore.sortBy} onchange={(e) => uiStore.sortBy = (e.target as HTMLSelectElement).value}>
      <option value="recent">最近添加</option>
      <option value="last_played">最近游玩</option>
      <option value="name">按名称</option>
      <option value="play_time">游玩时长</option>
    </select>

    <button
      class="view-btn"
      class:active={uiStore.viewMode === "grid"}
      onclick={() => (uiStore.viewMode = "grid")}
      title="网格视图"
    >
      <Icon name="collection" size={16} />
    </button>
    <button
      class="view-btn"
      class:active={uiStore.viewMode === "list"}
      onclick={() => (uiStore.viewMode = "list")}
      title="列表视图"
    >
      <Icon name="paperclip" size={16} />
    </button>
  </div>
</header>

<style>
  .search-header {
    padding: 20px 24px;
    display: flex; gap: 16px; align-items: center;
  }
  .search-box {
    flex: 1; display: flex; align-items: center; gap: 8px;
    background: var(--bg-surface, var(--bg-secondary));
    border-radius: var(--radius-full);
    padding: 10px 20px;
    border: 1px solid var(--border);
    transition: border-color 0.2s;
  }
  .search-box:focus-within { border-color: var(--accent); }
  .search-box input {
    flex: 1; background: none; border: none; color: var(--text-primary);
    font-size: 14px; outline: none; font-family: var(--font-ui);
  }
  .search-box input::placeholder { color: var(--text-muted); }
  .clear-btn {
    background: rgba(255,255,255,0.08); border: none; color: var(--text-muted);
    width: 24px; height: 24px; border-radius: 50%; cursor: pointer;
    display: flex; align-items: center; justify-content: center;
  }
  .clear-btn:hover { background: rgba(255,255,255,0.16); color: var(--text-primary); }

  .header-actions { display: flex; gap: 8px; align-items: center; }
  .sort-select {
    background: var(--bg-secondary); border: 1px solid var(--border);
    color: var(--text-primary); padding: 8px 12px;
    border-radius: var(--radius-full); font-size: 13px; cursor: pointer; outline: none;
  }
  .view-btn {
    background: var(--bg-secondary); border: 1px solid var(--border);
    color: var(--text-muted); border-radius: var(--radius-md);
    padding: 8px; cursor: pointer; display: flex; transition: all 0.15s;
  }
  .view-btn.active, .view-btn:hover { color: var(--text-primary); border-color: var(--border-hover); background: var(--bg-hover); }
</style>
