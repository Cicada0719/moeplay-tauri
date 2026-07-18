<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, Input } from "./ui";
  import { i18n } from "../stores/i18n.svelte";
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
  import { PageShell, PageHeader, FilterBar, StateBoundary, type ViewState } from "./ui-v2";

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

  // 三态统一：加载 / 错误 / 无游戏 / 就绪收敛到 StateBoundary。
  const viewState = $derived<ViewState>(
    loading ? "loading" : error ? "error" : games.length === 0 ? "empty" : "ready",
  );

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
      error = i18n.t("backup.no_save_dir");
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

<PageShell as="div" width="full" scrollable={false} class="backup-v2-shell" labelledBy="backup-page-title" ariaLabel={i18n.t("backup.title")}>
  <div class="bk">
    <div class="v2-grain bk-grain" aria-hidden="true"></div>

    <PageHeader
      id="backup-page-title"
      class="bk-header"
      eyebrow="バックアップ / BACKUP"
      title={i18n.t("backup.title")}
      description={i18n.t("backup.subtitle")}
    >
      {#snippet actions()}
        <label class="select-field">
          <span>{i18n.t("backup.game_label")}</span>
          <select bind:value={selectedId} onchange={refresh}>
            {#each games as game}
              <option value={game.id}>{game.name}</option>
            {/each}
          </select>
        </label>
      {/snippet}
    </PageHeader>

    <main class="bk-content">
      <FilterBar label={i18n.t("backup.toolbar_aria")} class="bk-toolbar">
        <Input class="note-field" bind:value={note} placeholder={i18n.t("backup.note_placeholder")} ariaLabel={i18n.t("backup.note_aria")} />
        {#snippet actions()}
          <Button variant="primary" disabled={!selectedId} press={() => createSnapshot(candidates[0]?.path)}>
            <Icon name="save" size={16} />
            <span>{i18n.t("backup.create")}</span>
          </Button>
        {/snippet}
      </FilterBar>

      <StateBoundary
        state={viewState}
        onRetry={load}
        retryLabel={i18n.t("button.retry")}
        title={viewState === "error" ? i18n.t("backup.error_title") : i18n.t("backup.empty_title")}
        description={viewState === "error" ? (error ?? undefined) : i18n.t("backup.empty_desc")}
        loadingRows={4}
      >
        <div class="bk-panels">
          <Card class="panel">
            <div class="panel-head">
              <h2>{i18n.t("backup.panel_candidates")}</h2>
              <span>{selected?.name ?? i18n.t("backup.no_selection")}</span>
            </div>
            {#if candidates.length}
              <div class="row-list">
                {#each candidates as item}
                  <article class="data-row">
                    <div class="row-copy">
                      <strong>{item.path}</strong>
                      <span>{i18n.t("backup.candidate_meta", { category: item.category, score: item.score, count: item.file_count })}</span>
                    </div>
                    <Button variant="ghost" size="sm" press={() => createSnapshot(item.path)}>
                      <Icon name="save" size={15} />
                      <span>{i18n.t("backup.backup_action")}</span>
                    </Button>
                  </article>
                {/each}
              </div>
            {:else}
              <EmptyState title={i18n.t("backup.candidates_empty_title")} description={i18n.t("backup.candidates_empty_desc")} />
            {/if}
          </Card>

          <Card class="panel">
            <div class="panel-head">
              <h2>{i18n.t("backup.panel_snapshots")}</h2>
              <span class="mono">{snapshots.length}</span>
            </div>
            {#if snapshots.length}
              <div class="timeline-list">
                {#each snapshots as snapshot}
                  <article class="timeline-row">
                    <span class="timeline-node" aria-hidden="true"></span>
                    <div class="timeline-copy">
                      <strong>{snapshot.file_name}</strong>
                      <span>{i18n.t("backup.snapshot_meta", { created: snapshot.created_at, count: snapshot.file_count })}</span>
                    </div>
                    <Button variant="ghost" size="sm" press={() => previewRestore(snapshot)} loading={previewingPath === snapshot.file_path} disabled={previewingPath !== null}>
                      <Icon name="refresh" size={15} />
                      <span>{i18n.t("backup.preview_restore")}</span>
                    </Button>
                  </article>
                {/each}
              </div>
            {:else}
              <EmptyState title={i18n.t("backup.snapshots_empty_title")} description={i18n.t("backup.snapshots_empty_desc")} />
            {/if}
          </Card>
        </div>
      </StateBoundary>
    </main>

    {#if restorePreview}
      {@const summary = summarizeSnapshotDiff(restorePreview.diff)}
      <div class="restore-backdrop" role="presentation" onclick={(event) => { if (event.currentTarget === event.target && !restoring) restorePreview = null; }}>
        <div class="restore-dialog-shell" role="dialog" aria-modal="true" aria-labelledby="restore-title">
          <Card class="restore-dialog">
          <div class="panel-head">
            <div>
              <h2 id="restore-title">{i18n.t("backup.restore_title")}</h2>
              <p>{restorePreview.snapshot.file_name}</p>
            </div>
            <Button variant="quiet" size="sm" press={() => restorePreview = null} disabled={restoring} ariaLabel={i18n.t("backup.close_preview")}>
              <Icon name="x" size={15} />
            </Button>
          </div>
          <div class="restore-summary">
            <strong>{i18n.t("backup.restore_changed_files", { count: summary.changedFiles })}</strong>
            <span>{i18n.t("backup.restore_diff", { added: restorePreview.diff.added.length, changed: restorePreview.diff.changed.length, removed: restorePreview.diff.removed.length, unchanged: restorePreview.diff.unchanged })}</span>
            <span>{i18n.t("backup.restore_checkpoint_note")}</span>
          </div>
          {#if summary.destructive}
            <p class="restore-warning" role="alert">{i18n.t("backup.restore_warning")}</p>
          {/if}
          <div class="diff-columns">
            {#each [
              { label: i18n.t("backup.diff_added"), values: restorePreview.diff.added },
              { label: i18n.t("backup.diff_changed"), values: restorePreview.diff.changed },
              { label: i18n.t("backup.diff_removed"), values: restorePreview.diff.removed },
            ] as group}
              <div>
                <strong>{group.label}</strong>
                {#if group.values.length}
                  {#each group.values.slice(0, 8) as value}<code>{value}</code>{/each}
                  {#if group.values.length > 8}<span>{i18n.t("backup.diff_more", { count: group.values.length - 8 })}</span>{/if}
                {:else}
                  <span>{i18n.t("backup.diff_none")}</span>
                {/if}
              </div>
            {/each}
          </div>
          <div class="restore-actions">
            <Button variant="ghost" press={() => restorePreview = null} disabled={restoring}>{i18n.t("button.cancel")}</Button>
            <Button variant="primary" press={confirmRestore} loading={restoring} disabled={restoring}>{i18n.t("backup.confirm_restore")}</Button>
          </div>
          </Card>
        </div>
      </div>
    {/if}
  </div>
</PageShell>

<style>
  :global(.backup-v2-shell) { height: 100%; }
  :global(.backup-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .bk {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .bk-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.bk-header) {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    flex-shrink: 0;
  }

  .bk-content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    width: 100%;
    max-width: 1180px;
    margin: 0 auto;
    padding: 0 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-behavior: smooth;
  }

  .select-field {
    min-width: min(320px, 100%);
    display: grid;
    gap: 8px;
  }

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

  h2,
  p {
    margin: 0;
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

  .bk-panels {
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
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

  .mono {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
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
    background: var(--border);
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
    border-bottom: 1px solid var(--border);
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
    border: 2px solid var(--accent);
    border-radius: 50%;
    background: var(--bg-card);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 10%, transparent);
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
    .bk-panels {
      grid-template-columns: 1fr;
    }

    .select-field {
      min-width: 0;
    }
  }

  @media (max-width: 560px) {
    .bk-content { padding: 0 16px 36px; }
    :global(.bk-header) { padding: 20px 16px 12px; }

    .data-row {
      grid-template-columns: 1fr;
    }

    .timeline-row {
      grid-template-columns: 18px minmax(0, 1fr);
    }

    .data-row :global(.ui-button),
    .timeline-row :global(.ui-button) {
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

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .bk-content { scroll-behavior: auto; }
  }
  :global([data-motion="reduce"]) .bk-content { scroll-behavior: auto; }
</style>
