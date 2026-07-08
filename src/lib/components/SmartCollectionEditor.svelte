<script lang="ts">
  import { gameStore, type SmartCollection } from "../stores/games.svelte";
  import Icon from "./Icon.svelte";
  import Dialog from "./ui/Dialog.svelte";
  import Input from "./ui/Input.svelte";
  import Tag from "./ui/Tag.svelte";
  import SegmentControl from "./ui/SegmentControl.svelte";
  import Switch from "./ui/Switch.svelte";
  import Button from "./ui/Button.svelte";

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

  const quickFilterOptions = [
    { value: "", label: "无" },
    { value: "favorites", label: "收藏" },
    { value: "recent", label: "最近添加" },
    { value: "unplayed", label: "未玩过" },
    { value: "recently_played", label: "最近玩过" },
  ];

  const statusOptions = [
    { value: "", label: "全部" },
    { value: "playing", label: "游玩中" },
    { value: "completed", label: "已通关" },
    { value: "on_hold", label: "搁置" },
    { value: "dropped", label: "已放弃" },
    { value: "plan_to_play", label: "计划中" },
  ];

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
  <Dialog open={open} onClose={() => (open = false)} title={collection ? "编辑合集" : "新建合集"}>
    <div class="sce-modal">
      <header class="sce-header">
        <h3>{collection ? "编辑合集" : "新建合集"}</h3>
        <button class="sce-close" onclick={() => (open = false)} type="button"><Icon name="x" size={16} /></button>
      </header>

      <div class="sce-body">
        <label class="sce-field">
          <span>名称</span>
          <Input type="text" bind:value={name} placeholder="合集名称" />
        </label>

        <label class="sce-field">
          <span>图标</span>
          <div class="sce-icons">
            {#each ICONS as ic}
              <Tag active={icon === ic} onclick={() => (icon = ic)} size="md" title={ic}>
                <Icon name={ic} size={16} />
              </Tag>
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
                aria-label={c || "无颜色"}
              >
                {#if color === c && !c}✕{/if}
              </button>
            {/each}
          </div>
        </label>

        <div class="sce-divider"></div>

        <label class="sce-field">
          <span>快速筛选</span>
          <SegmentControl options={quickFilterOptions} value={quickFilter} onChange={(v) => quickFilter = v} size="sm" />
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
          <SegmentControl options={statusOptions} value={status} onChange={(v) => status = v} size="sm" />
        </label>

        <label class="sce-field">
          <span>最低评分</span>
          <div class="sce-range">
            <input type="range" min="0" max="10" step="0.5" bind:value={minRating} />
            <span>{minRating > 0 ? minRating.toFixed(1) : "不限"}</span>
          </div>
        </label>

        <div class="sce-checks">
          <div class="sce-check"><Switch checked={installed} onchange={() => installed = !installed} /> 仅已安装</div>
          <div class="sce-check"><Switch checked={hasPlayed} onchange={() => hasPlayed = !hasPlayed} /> 仅已玩过</div>
        </div>
      </div>

      <footer class="sce-footer">
        {#if collection}
          <Button variant="ghost" size="sm" press={remove} class="sce-btn-danger">删除</Button>
        {/if}
        <div class="sce-footer-right">
          <Button variant="ghost" size="sm" press={() => (open = false)}>取消</Button>
          <Button variant="primary" size="sm" press={save} disabled={!name.trim()}>保存</Button>
        </div>
      </footer>
    </div>
  </Dialog>
{/if}

<style>
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
    display: flex; flex-direction: column; gap: 6px; margin-bottom: 14px;
  }
  .sce-field > span { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .sce-field select {
    padding: 8px 10px; border: 1px solid var(--border); border-radius: 6px;
    background: var(--bg-card, #12151e); color: var(--text-primary); font-size: 13px;
  }
  .sce-field select:focus {
    outline: none; border-color: var(--accent);
  }
  .sce-icons { display: flex; gap: 6px; flex-wrap: wrap; }
  .sce-colors { display: flex; gap: 6px; flex-wrap: wrap; }
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
  .sce-check { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-secondary); cursor: pointer; }
  .sce-footer {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 20px; border-top: 1px solid var(--border);
  }
  .sce-footer-right { display: flex; gap: 8px; }
  :global(.sce-btn-danger) {
    color: #f87171 !important;
  }
</style>
