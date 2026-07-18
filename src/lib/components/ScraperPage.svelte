<script lang="ts">
  import EmptyState from "./EmptyState.svelte";
  import { spotlight } from "../actions/spotlight";
  import { scrapeGame, type ScrapeResult, type ScrapeSourceOptions, type ScrapeSourceStatus } from "../api";
  import { i18n } from "../stores/i18n.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { Button } from "./ui";
  import { PageShell, PageHeader, AsyncState, type ViewState } from "./ui-v2";

  const sources: { key: keyof ScrapeSourceOptions | "vndb" | "bangumi"; label: string; settingKey: string }[] = [
    { key: "vndb", label: "VNDB", settingKey: "vndb_enabled" },
    { key: "bangumi", label: "Bangumi", settingKey: "bangumi_enabled" },
    { key: "dlsite", label: "DLsite", settingKey: "dlsite_enabled" },
    { key: "kungal", label: "Kungal", settingKey: "kungal_enabled" },
    { key: "steam", label: "Steam", settingKey: "steam_enabled" },
    { key: "pcgw", label: "PCGW", settingKey: "pcgw_enabled" },
    { key: "erogamescape", label: "ErogameScape", settingKey: "erogamescape_enabled" },
    { key: "ymgal", label: "Ymgal", settingKey: "ymgal_enabled" },
  ];

  let query = $state("");
  let strategy = $state("full");
  let loading = $state(false);
  let results = $state<ScrapeResult[]>([]);
  let sourceStatus = $state<ScrapeSourceStatus[]>([]);
  let progress = $state<string[]>([]);
  let selectedIndex = $state(0);
  const selectedResult = $derived(results[selectedIndex] ?? null);

  function isEnabled(key: string): boolean {
    return !!(settingsStore.settings as any)[key];
  }

  async function toggleSource(settingKey: string) {
    const s = settingsStore.settings as any;
    await settingsStore.save({ ...s, [settingKey]: !s[settingKey] });
    uiStore.notify("设置已保存", "success");
  }

  const enabledCount = $derived(sources.filter(s => isEnabled(s.settingKey)).length);

  async function runScrape() {
    if (!query.trim()) return;
    loading = true;
    progress = [`${strategyLabel(strategy)}：${query}`];
    try {
      const s = settingsStore.settings;
      const opts: ScrapeSourceOptions = {
        dlsite: s.dlsite_enabled ?? true,
        touchgal: s.touchgal_enabled ?? true,
        erogamescape: s.erogamescape_enabled ?? true,
        ymgal: s.ymgal_enabled ?? true,
        kungal: s.kungal_enabled ?? true,
        steam: s.steam_enabled ?? true,
        pcgw: s.pcgw_enabled ?? true,
      };
      const resp = await scrapeGame(query, s.vndb_enabled, s.bangumi_enabled, opts);
      results = resp.results;
      sourceStatus = resp.source_status;
      selectedIndex = 0;
      const okCount = sourceStatus.filter(s => s.ok).length;
      const failCount = sourceStatus.filter(s => !s.ok).length;
      progress = [...progress, `完成：${results.length} 条结果（${okCount} 源成功，${failCount} 源失败）`];
    } catch (error) {
      progress = [...progress, `失败：${String(error)}`];
    } finally {
      loading = false;
    }
  }

  function strategyLabel(value: string) {
    return value === "incremental" ? "增量刮削" : value === "patch" ? "补缺刮削" : "完整刮削";
  }

  function matchScore(result: ScrapeResult, index: number) {
    const ratingBase = result.rating ? Math.round(result.rating * 10) : 84;
    return Math.max(58, Math.min(98, ratingBase - index * 4));
  }

  const lastProgress = $derived(progress[progress.length - 1] ?? "");
  // 三态统一：未刮削 idle（progress 为空门闩，未刮削≠无结果）/ 刮削中 loading /
  // 末条进度为“失败：”时 error（主行动重试 = runScrape）/ 有进度与结果 ready。
  const scrapeState = $derived<ViewState>(
    loading
      ? "loading"
      : progress.length === 0
        ? "empty"
        : lastProgress.startsWith("失败：")
          ? "error"
          : "ready",
  );
</script>

<PageShell as="div" width="full" scrollable={false} class="scraper-v2-shell" labelledBy="scraper-page-title" ariaLabel={i18n.t("scraper.title")}>
  <div class="scraper-page">
    <div class="v2-grain sp-grain" aria-hidden="true"></div>

    <PageHeader
      id="scraper-page-title"
      class="sp-header"
      eyebrow="スクレイプ / SCRAPER"
      title={i18n.t("scraper.title")}
      description={i18n.t("scraper.subtitle")}
    >
      {#snippet actions()}
        <div class="head-stats">
          <span>{i18n.t("scraper.stats_sources", { count: enabledCount })}</span>
          <span>{i18n.t("scraper.stats_results", { count: results.length })}</span>
        </div>
        <Button press={runScrape} disabled={loading}>
          {loading ? i18n.t("scraper.running") : i18n.t("scraper.run")}
        </Button>
      {/snippet}
    </PageHeader>

    <div class="sp-content">
      <div class="toolbar">
        <input bind:value={query} placeholder={i18n.t("scraper.query_placeholder")} aria-label={i18n.t("scraper.query_aria")} onkeydown={(event) => event.key === "Enter" && runScrape()} />
        <select bind:value={strategy} aria-label={i18n.t("scraper.strategy_aria")}>
          <option value="full">{i18n.t("scraper.strategy_full")}</option>
          <option value="incremental">{i18n.t("scraper.strategy_incremental")}</option>
          <option value="patch">{i18n.t("scraper.strategy_patch")}</option>
        </select>
      </div>

      <div class="content-grid">
        <section class="panel candidate-panel">
          <div class="panel-head">
            <div>
              <p class="eyebrow">Candidates</p>
              <h2>{i18n.t("scraper.candidates_title")}</h2>
            </div>
            <span>{enabledCount}/{sources.length}</span>
          </div>

          <div class="source-list">
            {#each sources as source}
              <label class="source" class:enabled={isEnabled(source.settingKey)}>
                <input type="checkbox" checked={isEnabled(source.settingKey)} onchange={() => toggleSource(source.settingKey)} />
                <span>{source.label}</span>
                <em>{isEnabled(source.settingKey) ? "ON" : "OFF"}</em>
              </label>
            {/each}
          </div>

          {#if sourceStatus.length}
            <div class="source-status">
              {#each sourceStatus as st}
                <span class="src-st" class:ok={st.ok} class:fail={!st.ok} title={st.error ?? ""}>
                  {st.source.toUpperCase()}
                  {#if st.ok}
                    {st.count}
                  {:else}
                    ✕
                  {/if}
                </span>
              {/each}
            </div>
          {/if}

          {#if results.length}
            <div class="candidate-list" aria-label={i18n.t("scraper.results_aria")}>
              {#each results as result, i}
                <button class="candidate-row" class:active={selectedIndex === i} onclick={() => selectedIndex = i}>
                  <span class="candidate-copy">
                    <strong>{result.title}</strong>
                    <small>{result.source} · {result.release_year ?? i18n.t("scraper.unknown_year")}</small>
                  </span>
                  <span class="match">{matchScore(result, i)}%</span>
                </button>
              {/each}
            </div>
          {/if}

          <AsyncState
            state={scrapeState}
            compact
            title={scrapeState === "error" ? i18n.t("scraper.error_title") : scrapeState === "loading" ? i18n.t("scraper.running") : i18n.t("scraper.idle_title")}
            description={scrapeState === "error" ? lastProgress : i18n.t("scraper.idle_desc")}
            primaryAction={scrapeState === "error" ? { label: i18n.t("button.retry"), onSelect: runScrape } : undefined}
            loadingRows={2}
          >
            <ol class="progress">
              {#each progress as item, i}
                <li><span>{String(i + 1).padStart(2, "0")}</span>{item}</li>
              {/each}
            </ol>
          </AsyncState>
        </section>

        <section class="panel preview-panel" use:spotlight={{ radius: 420 }}>
          <div class="panel-head">
            <div>
              <p class="eyebrow">Preview</p>
              <h2>{i18n.t("scraper.preview_title")}</h2>
            </div>
            <span>{results.length}</span>
          </div>
          {#if selectedResult}
            <article class="preview-card">
              {#if selectedResult.cover}
                <img src={selectedResult.cover} alt={selectedResult.title} />
              {/if}
              <div class="preview-body">
                <div class="preview-title-row">
                  <strong>{selectedResult.title}</strong>
                  <span class="match">{matchScore(selectedResult, selectedIndex)}%</span>
                </div>
                <span class="preview-meta"><code>{selectedResult.source}</code> · <em>{selectedResult.release_year ?? i18n.t("scraper.unknown_year")}</em></span>
                <p>{selectedResult.description ?? i18n.t("scraper.no_description")}</p>
              </div>
            </article>
          {:else}
            <EmptyState title={i18n.t("scraper.empty_title")} description={i18n.t("scraper.empty_desc")} />
          {/if}
        </section>
      </div>
    </div>
  </div>
</PageShell>

<style>
  :global(.scraper-v2-shell) { height: 100%; }
  :global(.scraper-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .scraper-page {
    position: relative;
    isolation: isolate;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--bg-void);
    color: var(--text-primary);
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .sp-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.sp-header) {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    padding: 22px 24px 0;
  }
  .head-stats { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .head-stats span {
    display: inline-flex; align-items: baseline; gap: 5px;
    padding: 7px 10px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--bg-deep); color: var(--text-secondary); font-size: 0.76rem;
  }

  .sp-content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 18px 24px 36px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    scroll-behavior: smooth;
  }

  .eyebrow {
    margin: 0 0 4px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--accent);
    text-transform: uppercase;
  }
  h2 { font-size: 16px; margin: 0; }

  .toolbar { min-width: 0; max-width: 100%; box-sizing: border-box; display: grid; grid-template-columns: minmax(0, 1fr) 180px; gap: 12px; }
  .toolbar input,
  .toolbar select {
    background: var(--bg-deep); color: var(--text-primary);
    border: 1px solid var(--border); border-radius: 8px;
    padding: 12px 14px; outline: none;
  }

  .content-grid { min-width: 0; max-width: 100%; display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(320px, 1fr); gap: 16px; min-height: 0; align-items: start; }
  .panel { min-width: 0; max-width: 100%; box-sizing: border-box; border-radius: 8px; padding: 18px; background: var(--bg-card); border: 1px solid var(--border); box-shadow: var(--shadow-xs); }
  .panel-head { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; margin-bottom: 12px; }
  .panel-head > span { color: var(--accent); font-family: var(--font-mono); font-variant-numeric: tabular-nums; }

  .source-list { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
  .source { display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; gap: 10px; align-items: center; padding: 10px 12px; border-radius: 8px; background: rgba(255,255,255,.05); border: 1px solid rgba(255,255,255,.08); }
  .source span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .source em { font-family: var(--font-mono); font-style: normal; font-size: 0.68rem; color: var(--text-muted); }
  .source.enabled { border-color: rgba(255,183,197,.45); background: rgba(255,183,197,.12); }
  .source.enabled em { color: var(--accent); }

  .source-status { display: flex; flex-wrap: wrap; gap: 6px; padding: 10px 0 4px; }
  .src-st {
    font-size: 10px; font-weight: 700; padding: 2px 8px; border-radius: 6px;
    font-family: var(--font-mono);
  }
  .src-st.ok { background: rgba(76, 175, 80, 0.15); color: #81c784; }
  .src-st.fail { background: rgba(255, 80, 80, 0.15); color: #ff6b6b; }

  .candidate-list { display: grid; border-top: 1px solid var(--border); }
  .candidate-row {
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 11px 0;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    cursor: pointer;
  }
  .candidate-row:last-child { border-bottom: 0; }
  .candidate-row.active,
  .candidate-row:hover { color: var(--text-primary); }
  .candidate-copy { min-width: 0; display: grid; gap: 3px; color: inherit; }
  .candidate-copy strong,
  .candidate-copy small { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .candidate-copy small { color: var(--text-muted); font-size: 0.75rem; }
  .match { color: var(--accent); font-size: 0.82rem; font-family: var(--font-mono); font-variant-numeric: tabular-nums; }

  .progress { list-style: none; padding: 0; color: var(--text-secondary); line-height: 1.7; display: flex; flex-direction: column; gap: 7px; }
  .progress li {
    display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 10px; align-items: start;
    padding: 8px 10px; border-top: 1px solid var(--border);
  }
  .progress li span { color: var(--accent); font-family: var(--font-mono); font-variant-numeric: tabular-nums; }

  .preview-panel { position: relative; overflow: hidden; }
  /* Spotlight border glow driven by use:spotlight pointer tracking (opacity only). */
  .preview-panel::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    padding: 1px;
    background: radial-gradient(220px circle at var(--mx, var(--spotlight-x, 50%)) var(--my, var(--spotlight-y, 0%)), rgba(255, 255, 255, 0.22), transparent 70%);
    -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
    -webkit-mask-composite: xor;
    mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
    mask-composite: exclude;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s ease;
  }
  .preview-panel:hover::before { opacity: 1; }
  .preview-card { display: grid; grid-template-columns: 96px 1fr; gap: 14px; padding: 12px 0; border-top: 1px solid var(--border); background: transparent; }
  .preview-card img { width: 96px; height: 128px; border-radius: 8px; object-fit: cover; }
  .preview-title-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 10px; align-items: start; }
  .preview-meta { display: block; color: var(--accent); font-size: 12px; margin: 4px 0; }
  code, em { font-family: var(--font-mono); font-style: normal; }
  .preview-card p {
    margin: 0; color: var(--text-secondary); line-height: 1.5; font-size: 13px;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
  }

  @media (max-width: 900px) {
    .content-grid, .toolbar { grid-template-columns: minmax(0, 1fr); }
  }
  @media (max-width: 560px) {
    :global(.sp-header) { padding: 18px 16px 0; }
    .sp-content { padding: 16px 16px 32px; }
    .source-list { grid-template-columns: minmax(0, 1fr); }
    .panel-head { flex-direction: column; }
    .head-stats { justify-content: flex-start; }
    .preview-card { grid-template-columns: minmax(0, 1fr); }
    .preview-card img { width: 100%; height: auto; max-height: 220px; aspect-ratio: 3 / 4; }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .sp-content { scroll-behavior: auto; }
    .preview-panel::before { transition: none; }
  }
  :global([data-motion="reduce"]) .sp-content { scroll-behavior: auto; }
  :global([data-motion="reduce"]) .preview-panel::before { transition: none; }
</style>
