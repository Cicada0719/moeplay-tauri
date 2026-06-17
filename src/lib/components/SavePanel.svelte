<script lang="ts">
  import { gsap } from "gsap";
  import {
    createSaveSnapshot,
    detectSaveCandidates,
    listSaveSnapshots,
    restoreSaveSnapshot,
    deleteSaveSnapshot,
    type SaveCandidateDir,
    type SaveSnapshot,
  } from "../api";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";
  import Button from "./ui/Button.svelte";

  let {
    gameId,
    saveDir = "",
    compact = false,
  }: {
    gameId: string;
    saveDir?: string;
    compact?: boolean;
  } = $props();

  let candidates = $state<SaveCandidateDir[]>([]);
  let snapshots = $state<SaveSnapshot[]>([]);
  let loading = $state(false);
  let error = $state("");
  let expanded = $state(false);
  let lastGameId = "";

  const latestSnapshot = $derived(snapshots[0] ?? null);
  const displaySnapshots = $derived(expanded ? snapshots : snapshots.slice(0, 5));

  $effect(() => {
    if (!gameId || gameId === lastGameId) return;
    lastGameId = gameId;
    void refresh();
  });

  async function refresh() {
    if (!gameId) return;
    loading = true;
    error = "";
    try {
      const [c, s] = await Promise.all([
        detectSaveCandidates(gameId),
        listSaveSnapshots(gameId),
      ]);
      candidates = c;
      snapshots = s;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function createSnap(path?: string) {
    try {
      await createSaveSnapshot(gameId, path || saveDir || null, "详情页快照");
      await refresh();
      uiStore.notify("存档快照已创建", "success");
    } catch (e) {
      error = String(e);
      uiStore.notify(`快照失败：${e}`, "error");
    }
  }

  async function restoreSnap(snapshot: SaveSnapshot) {
    try {
      await restoreSaveSnapshot(gameId, snapshot.file_path, saveDir || null);
      uiStore.notify("存档已恢复", "success");
    } catch (e) {
      error = String(e);
      uiStore.notify(`恢复失败：${e}`, "error");
    }
  }

  async function deleteSnap(snapshot: SaveSnapshot) {
    try {
      await deleteSaveSnapshot(snapshot.file_path);
      await refresh();
      uiStore.notify("快照已删除", "success");
    } catch (e) {
      uiStore.notify(`删除失败：${e}`, "error");
    }
  }

  function formatSnapshotDate(dateStr: string): string {
    try {
      const d = new Date(dateStr);
      const now = new Date();
      const diffMs = now.getTime() - d.getTime();
      const diffMin = Math.floor(diffMs / 60000);
      const diffHr = Math.floor(diffMs / 3600000);
      const diffDay = Math.floor(diffMs / 86400000);
      if (diffMin < 1) return "刚刚";
      if (diffMin < 60) return `${diffMin} 分钟前`;
      if (diffHr < 24) return `${diffHr} 小时前`;
      if (diffDay < 7) return `${diffDay} 天前`;
      return d.toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" });
    } catch { return dateStr; }
  }
</script>

<div class="save-panel" class:compact>
  <div class="sp-header">
    <div class="sp-title">
      <Icon name="save" size={16} />
      <h4>存档管理</h4>
    </div>
    <div class="sp-actions">
      <button class="sp-btn primary" onclick={() => createSnap(candidates[0]?.path)} title="创建快照">
        <Icon name="plus" size={14} /> 创建快照
      </button>
      <button class="sp-btn" onclick={refresh} title="刷新">
        <Icon name="refresh" size={14} />
      </button>
    </div>
  </div>

  {#if loading}
    <div class="sp-loading">扫描存档目录中...</div>
  {:else}
    <div class="sp-info">
      <div class="sp-stat">
        <span class="sp-stat-label">存档目录</span>
        <span class="sp-stat-value" title={saveDir || candidates[0]?.path || ""}>
          {saveDir || candidates[0]?.path || "未配置"}
        </span>
      </div>
      <div class="sp-stat">
        <span class="sp-stat-label">候选目录</span>
        <span class="sp-stat-value">{candidates.length} 个</span>
      </div>
      <div class="sp-stat">
        <span class="sp-stat-label">快照数</span>
        <span class="sp-stat-value">{snapshots.length} 份</span>
      </div>
    </div>

    {#if error}
      <p class="sp-error">{error}</p>
    {/if}

    {#if snapshots.length > 0}
      <div class="sp-timeline">
        <div class="sp-timeline-header">
          <span>快照时间线</span>
          {#if snapshots.length > 5}
            <button class="sp-link" onclick={() => expanded = !expanded}>
              {expanded ? "收起" : `展开全部 (${snapshots.length})`}
            </button>
          {/if}
        </div>
        <div class="sp-timeline-list">
          {#each displaySnapshots as snap, i}
            <div class="sp-snap" class:latest={i === 0}>
              <div class="sp-snap-dot"></div>
              <div class="sp-snap-content">
                <div class="sp-snap-meta">
                  <span class="sp-snap-name">{snap.file_name}</span>
                  <span class="sp-snap-time">{formatSnapshotDate(snap.created_at)}</span>
                </div>
                <div class="sp-snap-detail">
                  <span>{snap.file_count} 个文件</span>
                  {#if snap.note}<span class="sp-snap-note">{snap.note}</span>{/if}
                </div>
                <div class="sp-snap-actions">
                  <button class="sp-link" onclick={() => restoreSnap(snap)}>恢复</button>
                  <button class="sp-link danger" onclick={() => deleteSnap(snap)}>删除</button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <p class="sp-empty">暂无快照，点击上方按钮创建第一份安全快照。</p>
    {/if}

    {#if !compact && candidates.length > 1}
      <div class="sp-candidates">
        <span class="sp-section-title">候选目录</span>
        {#each candidates as cand}
          <div class="sp-candidate">
            <div class="sp-cand-info">
              <span class="sp-cand-path" title={cand.path}>{cand.path}</span>
              <span class="sp-cand-meta">{cand.category} · 置信度 {(cand.score * 100).toFixed(0)}% · {cand.file_count} 文件</span>
            </div>
            <button class="sp-btn sm" onclick={() => createSnap(cand.path)}>备份</button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .save-panel {
    background: var(--bg-elev, rgba(255,255,255,0.03));
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 16px;
  }
  .save-panel.compact { padding: 12px; }

  .sp-header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 12px;
  }
  .sp-title { display: flex; align-items: center; gap: 8px; }
  .sp-title h4 { margin: 0; font-size: 14px; font-weight: 700; color: var(--text-primary); }
  .sp-actions { display: flex; gap: 6px; }

  .sp-btn {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 6px 12px; border: 1px solid var(--border); border-radius: 6px;
    background: rgba(255,255,255,0.04); color: var(--text-secondary);
    font-size: 12px; cursor: pointer; transition: all 0.15s;
  }
  .sp-btn:hover { border-color: var(--accent); color: var(--accent); }
  .sp-btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
  .sp-btn.primary:hover { filter: brightness(1.1); }
  .sp-btn.sm { padding: 4px 8px; font-size: 11px; }

  .sp-loading { padding: 12px 0; font-size: 13px; color: var(--text-muted); }

  .sp-info {
    display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 8px;
    margin-bottom: 12px;
  }
  .sp-stat { display: flex; flex-direction: column; gap: 2px; }
  .sp-stat-label { font-size: 10px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .sp-stat-value {
    font-size: 12px; color: var(--text-secondary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  .sp-error { font-size: 12px; color: #f87171; margin: 8px 0; }

  .sp-timeline { margin-top: 8px; }
  .sp-timeline-header {
    display: flex; align-items: center; justify-content: space-between;
    font-size: 12px; color: var(--text-muted); margin-bottom: 8px;
  }
  .sp-timeline-list { display: flex; flex-direction: column; gap: 0; }
  .sp-snap {
    display: flex; gap: 10px; align-items: flex-start;
    padding: 8px 0; border-left: 2px solid var(--border);
    margin-left: 5px; padding-left: 14px; position: relative;
  }
  .sp-snap-dot {
    position: absolute; left: -6px; top: 12px;
    width: 10px; height: 10px; border-radius: 50%;
    background: var(--border); border: 2px solid var(--bg-elev, #1a1d28);
  }
  .sp-snap.latest .sp-snap-dot { background: var(--accent); }
  .sp-snap-content { flex: 1; min-width: 0; }
  .sp-snap-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 2px; }
  .sp-snap-name {
    font-size: 12px; font-weight: 600; color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .sp-snap-time { font-size: 11px; color: var(--text-muted); white-space: nowrap; }
  .sp-snap-detail { font-size: 11px; color: var(--text-muted); margin-bottom: 4px; }
  .sp-snap-note { margin-left: 8px; font-style: italic; }
  .sp-snap-actions { display: flex; gap: 12px; }

  .sp-link {
    background: none; border: none; padding: 0; cursor: pointer;
    font-size: 11px; color: var(--accent); text-decoration: underline;
    text-underline-offset: 2px;
  }
  .sp-link:hover { color: var(--text-primary); }
  .sp-link.danger { color: #f87171; }
  .sp-link.danger:hover { color: #fca5a5; }

  .sp-empty { font-size: 12px; color: var(--text-muted); padding: 8px 0; }

  .sp-candidates { margin-top: 12px; }
  .sp-section-title { display: block; font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 6px; }
  .sp-candidate {
    display: flex; align-items: center; justify-content: space-between; gap: 8px;
    padding: 6px 0; border-top: 1px solid var(--border);
  }
  .sp-cand-info { flex: 1; min-width: 0; }
  .sp-cand-path {
    display: block; font-size: 12px; color: var(--text-primary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .sp-cand-meta { font-size: 10px; color: var(--text-muted); }
</style>
