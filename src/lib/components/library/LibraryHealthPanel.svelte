<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import {
    LibraryHealth,
    createLibraryFeatureStore,
    tauriLibraryApi,
    type LibraryApi,
    type LibraryFeatureState,
    type LibraryHealthIssue,
    type LibraryHealthSnapshot,
  } from "../../features/library";
  import Icon from "../Icon.svelte";
  import { Button, Dialog } from "../ui";

  type HealthGame = {
    id: string;
    name: string;
    exe_path?: string | null;
    launch_uri?: string | null;
  };

  let {
    open = false,
    games = [],
    api = tauriLibraryApi,
    onClose,
  }: {
    open?: boolean;
    games?: HealthGame[];
    api?: LibraryApi;
    onClose: () => void;
  } = $props();

  const feature = createLibraryFeatureStore(untrack(() => api));
  let snapshot = $state<LibraryFeatureState>(feature.getSnapshot());
  let loadedForOpen = $state(false);
  const unsubscribe = feature.subscribe((next) => (snapshot = next));

  const localizedHealth = $derived(snapshot.health ? localizeHealth(snapshot.health) : null);
  const gameById = $derived(new Map(games.map((game) => [game.id, game])));
  const missingGames = $derived.by(() => {
    const ids = new Set(
      (snapshot.health?.issues ?? [])
        .filter((issue) => issue.code === "missing_launch_target")
        .flatMap((issue) => issue.gameIds),
    );
    return [...ids].map((id) => gameById.get(id) ?? { id, name: id, exe_path: null, launch_uri: null });
  });

  $effect(() => {
    if (open && !loadedForOpen) {
      loadedForOpen = true;
      void feature.loadHealth();
    }
    if (!open && loadedForOpen) {
      loadedForOpen = false;
      feature.cancelHealth();
    }
  });

  onDestroy(() => {
    feature.cancelAll();
    unsubscribe();
  });

  function close() {
    feature.cancelHealth();
    onClose();
  }

  function retry() {
    if (!snapshot.isLoadingHealth) void feature.loadHealth();
  }

  function localizeIssue(issue: LibraryHealthIssue): LibraryHealthIssue {
    const known: Record<string, string> = {
      missing_launch_target: "游戏缺少可访问的启动路径或启动 URI",
      duplicate_strong_identity: "多个游戏共享同一启动路径或平台 ID",
      title_recall_group: "存在同名游戏；仅作召回提示，不会自动合并",
      unresolved_import_conflicts: "仍有 Library v2 导入冲突等待人工决策",
    };
    return { ...issue, message: known[issue.code] ?? issue.message };
  }

  function localizeHealth(health: LibraryHealthSnapshot): LibraryHealthSnapshot {
    return { ...health, issues: health.issues.map(localizeIssue) };
  }
</script>

<Dialog {open} onClose={close} title="库健康">
  <section class="health-dialog" aria-busy={snapshot.isLoadingHealth}>
    <header>
      <div>
        <span class="eyebrow">Library Integrity</span>
        <h2>库健康</h2>
        <p>检查启动目标、强身份重复、同名召回、未决冲突与 provenance 覆盖。</p>
      </div>
      <Button variant="quiet" size="sm" press={close} ariaLabel="关闭库健康面板">
        <Icon name="x" size={16} />关闭
      </Button>
    </header>

    {#if snapshot.isLoadingHealth && !localizedHealth}
      <div class="health-state" role="status">
        <span class="spinner" aria-hidden="true"></span>
        <div><strong>正在检查游戏库</strong><span>读取路径、身份索引与字段来源账本。</span></div>
        <Button variant="secondary" size="sm" press={() => feature.cancelHealth()}>取消</Button>
      </div>
    {:else if snapshot.error && !localizedHealth}
      <div class="health-state error" role="alert">
        <Icon name="x" size={18} />
        <div><strong>库健康读取失败</strong><span>{snapshot.error}</span></div>
        <Button variant="secondary" size="sm" press={retry}>重试</Button>
      </div>
    {:else if localizedHealth}
      <LibraryHealth health={localizedHealth} />

      {#if missingGames.length > 0}
        <section class="path-details" aria-labelledby="missing-path-title">
          <div class="section-head">
            <div>
              <span class="eyebrow">Missing Targets</span>
              <h3 id="missing-path-title">失效路径明细</h3>
            </div>
            <b>{missingGames.length}</b>
          </div>
          <ul>
            {#each missingGames as game (game.id)}
              <li>
                <div><strong>{game.name}</strong><code>{game.id}</code></div>
                <dl>
                  <div><dt>启动路径</dt><dd>{game.exe_path || "未设置"}</dd></div>
                  <div><dt>启动 URI</dt><dd>{game.launch_uri || "未设置"}</dd></div>
                </dl>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      <footer>
        <span>数据为空时 provenance 覆盖按 100% 计算。</span>
        <Button variant="secondary" size="sm" press={retry} loading={snapshot.isLoadingHealth}>
          <Icon name="refresh" size={15} />重新检查
        </Button>
      </footer>
    {:else}
      <div class="health-state empty">
        <Icon name="database" size={20} />
        <div><strong>暂无健康快照</strong><span>点击重试开始检查。</span></div>
        <Button variant="secondary" size="sm" press={retry}>开始检查</Button>
      </div>
    {/if}
  </section>
</Dialog>

<style>
  .health-dialog {
    width: min(760px, calc(100vw - 28px));
    max-height: min(82vh, 760px);
    overflow: auto;
    display: grid;
    gap: 14px;
    padding: 18px;
    border: 1px solid var(--border-hover);
    border-radius: 16px;
    background: var(--bg-panel);
    color: var(--text-primary);
    box-shadow: 0 24px 80px rgba(0, 0, 0, .5);
  }
  header, footer, .section-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  h2, h3 { margin: 0; }
  h2 { font-size: 21px; }
  h3 { font-size: 15px; }
  header p { margin: 5px 0 0; color: var(--text-muted); font-size: 12px; }
  .eyebrow { display: block; margin-bottom: 5px; color: var(--accent); font: 650 10px/1 var(--font-mono); text-transform: uppercase; }
  .health-state {
    min-height: 132px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    border: 1px dashed var(--border-hover);
    border-radius: 13px;
  }
  .health-state > div { display: grid; gap: 4px; }
  .health-state span { color: var(--text-muted); font-size: 12px; }
  .health-state :global(.ui-button) { margin-left: auto; }
  .health-state.error { border-style: solid; border-color: color-mix(in srgb, var(--color-error) 42%, var(--border)); }
  .spinner { width: 18px; height: 18px; border: 2px solid var(--border-hover); border-top-color: var(--accent); border-radius: 50%; animation: spin .8s linear infinite; }
  .path-details { display: grid; gap: 10px; padding: 14px; border: 1px solid var(--border); border-radius: 14px; background: color-mix(in srgb, var(--bg-elev) 70%, transparent); }
  .section-head b { font: 750 22px/1 var(--font-mono); color: var(--color-error); }
  ul { display: grid; gap: 8px; margin: 0; padding: 0; list-style: none; }
  li { display: grid; grid-template-columns: minmax(130px, .7fr) minmax(0, 1.3fr); gap: 12px; padding: 10px 11px; border-left: 3px solid var(--color-error); background: var(--bg-panel); }
  li > div { min-width: 0; display: grid; gap: 4px; align-content: start; }
  li strong { font-size: 12px; }
  code { color: var(--text-muted); font-size: 10px; overflow-wrap: anywhere; }
  dl { min-width: 0; display: grid; gap: 5px; margin: 0; }
  dl div { display: grid; grid-template-columns: 62px minmax(0, 1fr); gap: 8px; }
  dt { color: var(--text-muted); font-size: 10px; }
  dd { min-width: 0; margin: 0; color: var(--text-secondary); font: 500 10px/1.4 var(--font-mono); overflow-wrap: anywhere; }
  footer { align-items: center; color: var(--text-muted); font-size: 10px; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 620px) {
    .health-dialog { padding: 14px; }
    header, footer { flex-direction: column; align-items: stretch; }
    li { grid-template-columns: 1fr; }
  }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
