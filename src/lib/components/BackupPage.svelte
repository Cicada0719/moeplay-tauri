<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, Input } from "./ui";
  import {
    compareSaveSnapshot,
    createSaveSnapshot,
    detectSaveCandidates,
    getGames,
    listSaveSnapshots,
    restoreSaveSnapshot,
    type Game,
    type SaveCandidateDir,
    type SaveSnapshot,
    type SnapshotDiff,
  } from "../api";
  import { summarizeSnapshotDiff } from "../features/backup/preview";

  let games = $state<Game[]>([]);
  let selectedId = $state("");
  let candidates = $state<SaveCandidateDir[]>([]);
  let snapshots = $state<SaveSnapshot[]>([]);
  let note = $state("");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let restorePreview = $state<{ snapshot: SaveSnapshot; saveDir: string; diff: SnapshotDiff } | null>(null);
  let previewingPath = $state<string | null>(null);
  let restoring = $state(false);

  const selected = $derived(games.find((game) => game.id === selectedId));

  async function load() {
    loading = true;
    error = null;
    try {
      games = await getGames();
      selectedId = selectedId || games[0]?.id || "";
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function refresh() {
    if (!selectedId) return;
    try {
      candidates = await detectSaveCandidates(selectedId);
      snapshots = await listSaveSnapshots(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function createSnapshot(path?: string) {
    if (!selectedId) return;
    await createSaveSnapshot(selectedId, path ?? null, note || null);
    note = "";
    await refresh();
  }

  async function previewRestore(snapshot: SaveSnapshot) {
    const saveDir = selected?.save_data.save_dir ?? candidates[0]?.path;
    if (!saveDir) {
      error = "无法确定当前存档目录，请先在游戏详情中设置存档目录。";
      return;
    }
    previewingPath = snapshot.file_path;
    error = null;
    try {
      const diff = await compareSaveSnapshot(snapshot.file_path, saveDir);
      restorePreview = { snapshot, saveDir, diff };
    } catch (e) {
      error = String(e);
    } finally {
      previewingPath = null;
    }
  }

  async function confirmRestore() {
    if (!restorePreview || !selectedId) return;
    restoring = true;
    error = null;
    try {
      await restoreSaveSnapshot(
        selectedId,
        restorePreview.snapshot.file_path,
        restorePreview.saveDir,
        true,
      );
      restorePreview = null;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      restoring = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="tool-page aura-page" data-aura-echo="BACKUP">
  <header class="tool-head aura-head">
    <div class="head-copy">
      <span class="eyebrow aura-kicker">Save Backup</span>
      <h1 class="aura-title">存档管理</h1>
      <p>自动探测存档目录，创建多版本快照，恢复前会生成安全检查点。</p>
    </div>
    <label class="select-field">
      <span>游戏</span>
      <select bind:value={selectedId} onchange={refresh}>
        {#each games as game}
          <option value={game.id}>{game.name}</option>
        {/each}
      </select>
    </label>
  </header>

  <div class="toolbar">
    <Input class="note-field" bind:value={note} placeholder="快照备注（可选）" />
    <Button disabled={!selectedId} press={() => createSnapshot(candidates[0]?.path)}>
      <Icon name="save" size={16} />
      <span>创建快照</span>
    </Button>
  </div>

  <div class="content-grid">
    {#if loading}
      <Card class="panel full-width">
        <EmptyState title="正在加载存档数据…" />
      </Card>
    {:else if error}
      <Card class="panel full-width">
        <EmptyState title="加载失败" description={error ?? undefined} action={{ label: "重试", onclick: load }} />
      </Card>
    {:else}
      <Card class="panel">
        <div class="panel-head">
          <h2>候选存档目录</h2>
          <span>{selected?.name ?? "未选择"}</span>
        </div>
        {#if candidates.length}
          <div class="row-list">
            {#each candidates as item}
              <article class="data-row">
                <div class="row-copy">
                  <strong>{item.path}</strong>
                  <span>{item.category} · <span class="aura-num">{item.score}</span> 分 · <span class="aura-num">{item.file_count}</span> 文件</span>
                </div>
                <Button variant="ghost" size="sm" press={() => createSnapshot(item.path)}>
                  <Icon name="save" size={15} />
                  <span>备份</span>
                </Button>
              </article>
            {/each}
          </div>
        {:else}
          <EmptyState title="未检测到存档" description="可先在游戏详情里手动设置存档目录。" />
        {/if}
      </Card>

      <Card class="panel">
        <div class="panel-head">
          <h2>快照</h2>
          <span class="aura-num">{snapshots.length}</span>
        </div>
        {#if snapshots.length}
          <div class="timeline-list">
            {#each snapshots as snapshot}
              <article class="timeline-row">
                <span class="timeline-node" aria-hidden="true"></span>
                <div class="timeline-copy">
                  <strong>{snapshot.file_name}</strong>
                  <span><span class="aura-num">{snapshot.created_at}</span> · <span class="aura-num">{snapshot.file_count}</span> 文件</span>
                </div>
                <Button variant="ghost" size="sm" press={() => previewRestore(snapshot)} loading={previewingPath === snapshot.file_path} disabled={previewingPath !== null}>
                  <Icon name="refresh" size={15} />
                  <span>预览恢复</span>
                </Button>
              </article>
            {/each}
          </div>
        {:else}
          <EmptyState title="暂无快照" description="选择一个候选目录并创建第一份快照。" />
        {/if}
      </Card>
    {/if}
  </div>

  {#if restorePreview}
    {@const summary = summarizeSnapshotDiff(restorePreview.diff)}
    <div class="restore-backdrop" role="presentation" onclick={(event) => { if (event.currentTarget === event.target && !restoring) restorePreview = null; }}>
      <div class="restore-dialog-shell" role="dialog" aria-modal="true" aria-labelledby="restore-title">
        <Card class="restore-dialog">
        <div class="panel-head">
          <div>
            <h2 id="restore-title">确认恢复快照</h2>
            <p>{restorePreview.snapshot.file_name}</p>
          </div>
          <Button variant="quiet" size="sm" press={() => restorePreview = null} disabled={restoring} ariaLabel="关闭恢复预览">
            <Icon name="x" size={15} />
          </Button>
        </div>
        <div class="restore-summary">
          <strong>{summary.changedFiles} 个文件将发生变化</strong>
          <span>新增 {restorePreview.diff.added.length} · 覆盖 {restorePreview.diff.changed.length} · 移除 {restorePreview.diff.removed.length} · 不变 {restorePreview.diff.unchanged}</span>
          <span>恢复前会自动创建当前存档的安全检查点。</span>
        </div>
        {#if summary.destructive}
          <p class="restore-warning" role="alert">该操作会覆盖或移除当前存档文件，请确认快照正确。</p>
        {/if}
        <div class="diff-columns">
          {#each [
            { label: "新增", values: restorePreview.diff.added },
            { label: "覆盖", values: restorePreview.diff.changed },
            { label: "移除", values: restorePreview.diff.removed },
          ] as group}
            <div>
              <strong>{group.label}</strong>
              {#if group.values.length}
                {#each group.values.slice(0, 8) as value}<code>{value}</code>{/each}
                {#if group.values.length > 8}<span>另有 {group.values.length - 8} 项</span>{/if}
              {:else}
                <span>无</span>
              {/if}
            </div>
          {/each}
        </div>
        <div class="restore-actions">
          <Button variant="ghost" press={() => restorePreview = null} disabled={restoring}>取消</Button>
          <Button variant="primary" press={confirmRestore} loading={restoring} disabled={restoring}>创建检查点并恢复</Button>
        </div>
        </Card>
      </div>
    </div>
  {/if}

</section>

<style>
  .tool-page {
    min-width: 0;
    height: 100%;
    padding: 24px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: var(--bg-void, var(--bg-base));
  }

  .tool-head,
  .toolbar {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
    box-shadow: none;
  }

  .tool-head,
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
  }

  .head-copy {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .eyebrow,
  .aura-kicker,
  .select-field span,
  .panel-head span,
  .row-copy span,
  .timeline-copy span {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 650;
    line-height: 1.2;
    letter-spacing: 0;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    color: var(--text-primary);
    font-size: 24px;
    font-weight: 750;
    line-height: 1.15;
    letter-spacing: 0;
  }

  h2 {
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 700;
    line-height: 1.2;
    letter-spacing: 0;
  }

  p {
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .select-field {
    min-width: min(320px, 100%);
    display: grid;
    gap: 8px;
  }

  select {
    min-width: 0;
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 11px 12px;
    background: var(--bg-deep);
    color: var(--text-primary);
    font: inherit;
    outline: none;
  }

  select:focus-visible {
    border-color: var(--accent-ring);
    box-shadow: var(--focus-ring);
  }

  :global(.ui-input.note-field) {
    flex: 1;
  }

  :global(.ui-card.full-width) {
    grid-column: 1 / -1;
  }

  .content-grid {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  :global(.ui-card.panel) {
    min-width: 0;
  }

  .panel-head {
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .panel-head span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-list {
    min-width: 0;
    display: grid;
  }

  .timeline-list {
    min-width: 0;
    position: relative;
    display: grid;
    padding-left: 2px;
  }

  .timeline-list::before {
    content: "";
    position: absolute;
    left: 6px;
    top: 16px;
    bottom: 16px;
    width: 1px;
    background: var(--aura-line);
  }

  .data-row {
    min-width: 0;
    padding: 12px 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    border-bottom: 1px solid var(--border);
  }

  .data-row:last-child {
    border-bottom: 0;
    padding-bottom: 0;
  }

  .timeline-row {
    min-width: 0;
    position: relative;
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 12px 0;
    border-bottom: 1px solid var(--aura-line);
  }

  .timeline-row:last-child {
    border-bottom: 0;
    padding-bottom: 0;
  }

  .timeline-node {
    position: relative;
    z-index: 1;
    width: 11px;
    height: 11px;
    border: 2px solid var(--aura-data-a);
    border-radius: 50%;
    background: var(--aura-bg);
    box-shadow: 0 0 0 4px rgba(232, 85, 127, 0.08);
  }

  .timeline-copy,
  .row-copy {
    min-width: 0;
    display: grid;
    gap: 5px;
  }

  .row-copy strong,
  .timeline-copy strong {
    min-width: 0;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 700;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .row-copy span,
  .timeline-copy span {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  @media (max-width: 900px) {
    .content-grid {
      grid-template-columns: 1fr;
    }

    .tool-head,
    .toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .select-field {
      min-width: 0;
    }
  }

  @media (max-width: 560px) {
    .tool-page {
      padding: 18px;
    }

    .data-row {
      grid-template-columns: 1fr;
    }

    .timeline-row {
      grid-template-columns: 18px minmax(0, 1fr);
    }

    .data-row :global(.ui-button),
    .timeline-row :global(.ui-button),
    .toolbar :global(.ui-button) {
      width: 100%;
    }

    .timeline-row :global(.ui-button) {
      grid-column: 2;
      justify-self: start;
      width: auto;
    }
  }

  .restore-backdrop { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 24px; background: rgba(0, 0, 0, 0.66); }
  .restore-dialog-shell { width: min(760px, 100%); }
  :global(.ui-card.restore-dialog) { width: 100%; max-height: min(760px, calc(100vh - 48px)); overflow: auto; display: grid; gap: 16px; padding: 20px; }
  .restore-summary { display: grid; gap: 6px; }
  .restore-summary span, .restore-warning, .diff-columns span { color: var(--text-secondary); }
  .restore-warning { margin: 0; padding: 12px; border: 1px solid color-mix(in srgb, var(--danger, #ef4444) 45%, transparent); border-radius: 8px; background: color-mix(in srgb, var(--danger, #ef4444) 10%, transparent); }
  .diff-columns { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
  .diff-columns > div { min-width: 0; display: grid; align-content: start; gap: 6px; padding: 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-inset, var(--bg-base)); }
  .diff-columns code { overflow: hidden; color: var(--text-secondary); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
  .restore-actions { display: flex; justify-content: flex-end; gap: 10px; }
  @media (max-width: 760px) { .diff-columns { grid-template-columns: 1fr; } }
</style>
