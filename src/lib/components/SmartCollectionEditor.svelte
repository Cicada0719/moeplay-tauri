<script lang="ts">
  import { gameStore, type SmartCollection } from "../stores/games.svelte";
  import Icon from "./Icon.svelte";

  let {
    collection = $bindable(null),
    open = $bindable(false),
    initialFilters,
  }: {
    collection: SmartCollection | null;
    open: boolean;
    initialFilters?: SmartCollection["filters"];
  } = $props();

  let name = $state("");
  let icon = $state("folder");
  let color = $state("");
  let quickFilter = $state("");
  let filterTag = $state("");
  let platform = $state("");
  let status = $state("");
  let minRating = $state(0);
  let installed = $state(false);
  let hasPlayed = $state(false);

  const ICONS = ["folder", "gamepad", "star", "heart", "zap", "shield", "diamond", "clock", "eye", "lightbulb", "compass", "layers"];
  const COLORS = ["", "#e8557f", "#a78bfa", "#60a5fa", "#34d399", "#f59e0b", "#ef4444", "#ec4899"];

  $effect(() => {
    if (collection) {
      name = collection.name;
      icon = collection.icon;
      color = collection.color || "";
      quickFilter = collection.filters.quickFilter || "";
      filterTag = collection.filters.filterTag || "";
      platform = collection.filters.platform || "";
      status = collection.filters.status || "";
      minRating = collection.filters.minRating || 0;
      installed = collection.filters.installed || false;
      hasPlayed = collection.filters.hasPlayed || false;
    } else if (initialFilters) {
      name = "";
      icon = "folder";
      color = "";
      quickFilter = initialFilters.quickFilter || "";
      filterTag = initialFilters.filterTag || "";
      platform = initialFilters.platform || "";
      status = initialFilters.status || "";
      minRating = initialFilters.minRating || 0;
      installed = initialFilters.installed || false;
      hasPlayed = initialFilters.hasPlayed || false;
    } else {
      name = "";
      icon = "folder";
      color = "";
      quickFilter = "";
      filterTag = "";
      platform = "";
      status = "";
      minRating = 0;
      installed = false;
      hasPlayed = false;
    }
  });

  const allTags = $derived(
    [...new Set(gameStore.allGames.flatMap(g => {
      const tags: string[] = [];
      try {
        const t = (g as any).tags;
        if (Array.isArray(t)) tags.push(...t.map((x: any) => typeof x === "string" ? x : x?.name ?? ""));
      } catch {}
      return tags;
    }))].filter(Boolean).sort().slice(0, 50)
  );

  const allPlatforms = $derived(
    [...new Set(gameStore.allGames.map(g => (g as any).platform).filter(Boolean))].sort()
  );

  function save() {
    if (!name.trim()) return;
    const filters: SmartCollection["filters"] = {};
    if (quickFilter) filters.quickFilter = quickFilter;
    if (filterTag) filters.filterTag = filterTag;
    if (platform) filters.platform = platform;
    if (status) filters.status = status;
    if (minRating > 0) filters.minRating = minRating;
    if (installed) filters.installed = true;
    if (hasPlayed) filters.hasPlayed = true;

    if (collection) {
      gameStore.updateCollection(collection.id, { name: name.trim(), icon, color: color || undefined, filters });
    } else {
      gameStore.addCollection(name.trim(), icon, filters);
    }
    open = false;
  }

  function remove() {
    if (!collection) return;
    gameStore.removeCollection(collection.id);
    open = false;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="sce-overlay" onclick={() => (open = false)} role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
    <div class="sce-modal" onclick={(e) => e.stopPropagation()} role="document">
      <header class="sce-header">
        <h3>{collection ? "编辑合集" : "新建合集"}</h3>
        <button class="sce-close" onclick={() => (open = false)}><Icon name="x" size={16} /></button>
      </header>

      <div class="sce-body">
        <label class="sce-field">
          <span>名称</span>
          <input type="text" bind:value={name} placeholder="合集名称" />
        </label>

        <label class="sce-field">
          <span>图标</span>
          <div class="sce-icons">
            {#each ICONS as ic}
              <button class="sce-icon-btn" class:active={icon === ic} onclick={() => (icon = ic)} type="button">
                <Icon name={ic} size={16} />
              </button>
            {/each}
          </div>
        </label>

        <label class="sce-field">
          <span>颜色</span>
          <div class="sce-colors">
            {#each COLORS as c}
              <button
                class="sce-color-btn"
                class:active={color === c}
                style={c ? `background:${c}` : "background:var(--border)"}
                onclick={() => (color = c)}
                type="button"
              >
                {#if color === c && !c}✕{/if}
              </button>
            {/each}
          </div>
        </label>

        <div class="sce-divider"></div>

        <label class="sce-field">
          <span>快速筛选</span>
          <select bind:value={quickFilter}>
            <option value="">无</option>
            <option value="favorites">收藏</option>
            <option value="recent">最近添加</option>
            <option value="unplayed">未玩过</option>
            <option value="recently_played">最近玩过</option>
          </select>
        </label>

        <label class="sce-field">
          <span>标签筛选</span>
          <select bind:value={filterTag}>
            <option value="">无</option>
            {#each allTags as tag}
              <option value={tag}>{tag}</option>
            {/each}
          </select>
        </label>

        <label class="sce-field">
          <span>平台</span>
          <select bind:value={platform}>
            <option value="">全部</option>
            {#each allPlatforms as p}
              <option value={p}>{p}</option>
            {/each}
          </select>
        </label>

        <label class="sce-field">
          <span>状态</span>
          <select bind:value={status}>
            <option value="">全部</option>
            <option value="playing">游玩中</option>
            <option value="completed">已通关</option>
            <option value="on_hold">搁置</option>
            <option value="dropped">已放弃</option>
            <option value="plan_to_play">计划中</option>
          </select>
        </label>

        <label class="sce-field">
          <span>最低评分</span>
          <div class="sce-range">
            <input type="range" min="0" max="10" step="0.5" bind:value={minRating} />
            <span>{minRating > 0 ? minRating.toFixed(1) : "不限"}</span>
          </div>
        </label>

        <div class="sce-checks">
          <label><input type="checkbox" bind:checked={installed} /> 仅已安装</label>
          <label><input type="checkbox" bind:checked={hasPlayed} /> 仅已玩过</label>
        </div>
      </div>

      <footer class="sce-footer">
        {#if collection}
          <button class="sce-btn danger" onclick={remove}>删除</button>
        {/if}
        <div class="sce-footer-right">
          <button class="sce-btn" onclick={() => (open = false)}>取消</button>
          <button class="sce-btn primary" onclick={save} disabled={!name.trim()}>保存</button>
        </div>
      </footer>
    </div>
  </div>
{/if}

<style>
  .sce-overlay {
    position: fixed; inset: 0; z-index: 1000;
    background: rgba(0,0,0,0.6); backdrop-filter: blur(6px);
    display: flex; align-items: center; justify-content: center;
    animation: fade-in 0.15s ease;
  }
  .sce-modal {
    width: 420px; max-width: 92vw; max-height: 85vh;
    background: var(--bg-elev, #1a1d28); border: 1px solid var(--border);
    border-radius: 14px; overflow: hidden; display: flex; flex-direction: column;
    box-shadow: 0 24px 48px rgba(0,0,0,0.4);
  }
  .sce-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 16px 20px; border-bottom: 1px solid var(--border);
  }
  .sce-header h3 { margin: 0; font-size: 16px; font-weight: 700; color: var(--text-primary); }
  .sce-close {
    display: flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; border: none; border-radius: 6px;
    background: rgba(255,255,255,0.06); color: var(--text-muted); cursor: pointer;
  }
  .sce-close:hover { background: rgba(255,255,255,0.12); color: var(--text-primary); }
  .sce-body { padding: 16px 20px; overflow-y: auto; flex: 1; }
  .sce-field {
    display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px;
  }
  .sce-field > span { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .sce-field input[type="text"],
  .sce-field select {
    padding: 8px 10px; border: 1px solid var(--border); border-radius: 6px;
    background: var(--bg-card, #12151e); color: var(--text-primary); font-size: 13px;
  }
  .sce-field input:focus, .sce-field select:focus {
    outline: none; border-color: var(--accent);
  }
  .sce-icons { display: flex; gap: 6px; flex-wrap: wrap; }
  .sce-icon-btn {
    width: 32px; height: 32px; display: flex; align-items: center; justify-content: center;
    border: 1px solid var(--border); border-radius: 6px; background: transparent;
    color: var(--text-muted); cursor: pointer; transition: all 0.15s;
  }
  .sce-icon-btn:hover { border-color: var(--accent); color: var(--accent); }
  .sce-icon-btn.active { border-color: var(--accent); background: var(--accent); color: #fff; }
  .sce-colors { display: flex; gap: 6px; }
  .sce-color-btn {
    width: 28px; height: 28px; border-radius: 50%; border: 2px solid transparent;
    cursor: pointer; transition: all 0.15s; display: flex; align-items: center; justify-content: center;
    font-size: 10px; color: var(--text-muted);
  }
  .sce-color-btn:hover { border-color: rgba(255,255,255,0.3); }
  .sce-color-btn.active { border-color: #fff; box-shadow: 0 0 0 2px var(--accent); }
  .sce-divider { border-top: 1px solid var(--border); margin: 12px 0; }
  .sce-range { display: flex; align-items: center; gap: 8px; }
  .sce-range input { flex: 1; }
  .sce-range span { font-size: 12px; color: var(--text-secondary); min-width: 30px; }
  .sce-checks { display: flex; gap: 16px; }
  .sce-checks label { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-secondary); cursor: pointer; }
  .sce-checks input { accent-color: var(--accent); }
  .sce-footer {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 20px; border-top: 1px solid var(--border);
  }
  .sce-footer-right { display: flex; gap: 8px; }
  .sce-btn {
    padding: 8px 16px; border: 1px solid var(--border); border-radius: 6px;
    background: transparent; color: var(--text-secondary); font-size: 13px; cursor: pointer;
  }
  .sce-btn:hover { border-color: var(--accent); color: var(--accent); }
  .sce-btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
  .sce-btn.primary:hover { filter: brightness(1.1); }
  .sce-btn.primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .sce-btn.danger { border-color: rgba(248,113,113,0.3); color: #f87171; }
  .sce-btn.danger:hover { background: rgba(248,113,113,0.1); }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
</style>
