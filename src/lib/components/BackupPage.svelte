<script lang="ts">
  import { onMount } from "svelte";
  import EmptyState from "./EmptyState.svelte";
  import Icon from "./Icon.svelte";
  import {
    createSaveSnapshot,
    detectSaveCandidates,
    getGames,
    listSaveSnapshots,
    restoreSaveSnapshot,
    type Game,
    type SaveCandidateDir,
    type SaveSnapshot,
  } from "../api";

  let games = $state<Game[]>([]);
  let selectedId = $state("");
  let candidates = $state<SaveCandidateDir[]>([]);
  let snapshots = $state<SaveSnapshot[]>([]);
  let note = $state("");

  const selected = $derived(games.find((game) => game.id === selectedId));

  async function load() {
    games = await getGames();
    selectedId = selectedId || games[0]?.id || "";
    await refresh();
  }

  async function refresh() {
    if (!selectedId) return;
    candidates = await detectSaveCandidates(selectedId);
    snapshots = await listSaveSnapshots(selectedId);
  }

  async function createSnapshot(path?: string) {
    if (!selectedId) return;
    await createSaveSnapshot(selectedId, path ?? null, note || null);
    note = "";
    await refresh();
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
    <input bind:value={note} placeholder="快照备注（可选）" />
    <button class="primary-action" disabled={!selectedId} onclick={() => createSnapshot(candidates[0]?.path)}>
      <Icon name="save" size={16} />
      <span>创建快照</span>
    </button>
  </div>

  <div class="content-grid">
    <section class="panel">
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
              <button class="row-action" onclick={() => createSnapshot(item.path)}>
                <Icon name="save" size={15} />
                <span>备份</span>
              </button>
            </article>
          {/each}
        </div>
      {:else}
        <EmptyState title="未检测到存档" description="可先在游戏详情里手动设置存档目录。" />
      {/if}
    </section>

    <section class="panel">
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
              <button class="row-action" onclick={() => restoreSaveSnapshot(selectedId, snapshot.file_path)}>
                <Icon name="refresh" size={15} />
                <span>恢复</span>
              </button>
            </article>
          {/each}
        </div>
      {:else}
        <EmptyState title="暂无快照" description="选择一个候选目录并创建第一份快照。" />
      {/if}
    </section>
  </div>
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
  .toolbar,
  .panel {
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

  select,
  input {
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

  select:focus-visible,
  input:focus-visible {
    border-color: var(--accent-ring);
    box-shadow: var(--focus-ring);
  }

  input {
    flex: 1;
  }

  button {
    min-width: 0;
    border-radius: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.16s ease, border-color 0.16s ease, color 0.16s ease, transform 0.16s ease;
  }

  button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  button:active:not(:disabled) {
    transform: translateY(1px);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.56;
  }

  .primary-action {
    min-height: 38px;
    border: 1px solid var(--accent-ring);
    padding: 0 14px;
    color: #fff;
    background: var(--accent);
  }

  .primary-action:hover:not(:disabled) {
    background: var(--accent-hi);
    border-color: var(--accent-hi);
  }

  .content-grid {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  .panel {
    padding: 16px;
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

  .row-action {
    min-height: 34px;
    border: 1px solid var(--border);
    padding: 0 12px;
    color: var(--text-secondary);
    background: transparent;
  }

  .row-action:hover {
    border-color: var(--border-hover);
    color: var(--text-primary);
    background: var(--bg-hover);
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

    .data-row .row-action,
    .primary-action {
      width: 100%;
    }

    .timeline-row .row-action {
      grid-column: 2;
      justify-self: start;
      width: auto;
    }
  }
</style>
