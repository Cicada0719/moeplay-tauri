<script lang="ts">
  import EmptyState from "./EmptyState.svelte";
  import { spotlight } from "../actions/spotlight";
  import { scrapeGame, type ScrapeResult, type ScrapeSourceOptions } from "../api";

  const sources: { key: keyof ScrapeSourceOptions | "vndb" | "bangumi"; label: string }[] = [
    { key: "vndb", label: "VNDB" },
    { key: "bangumi", label: "Bangumi" },
    { key: "dlsite", label: "DLsite" },
    { key: "kungal", label: "Kungal" },
    { key: "steam", label: "Steam" },
    { key: "pcgw", label: "PCGW" },
    { key: "erogamescape", label: "ErogameScape" },
    { key: "ymgal", label: "Ymgal" },
  ];

  let query = $state("");
  let strategy = $state("full");
  let enabled = $state<Record<string, boolean>>({
    vndb: true,
    bangumi: true,
    dlsite: false,
    kungal: true,
    steam: false,
    pcgw: false,
    erogamescape: false,
    ymgal: false,
  });
  let loading = $state(false);
  let results = $state<ScrapeResult[]>([]);
  let progress = $state<string[]>([]);
  let selectedIndex = $state(0);
  const selectedResult = $derived(results[selectedIndex] ?? null);

  async function runScrape() {
    if (!query.trim()) return;
    loading = true;
    progress = [`${strategyLabel(strategy)}：${query}`];
    try {
      const opts: ScrapeSourceOptions = {
        dlsite: enabled.dlsite,
        touchgal: false,
        erogamescape: enabled.erogamescape,
        ymgal: enabled.ymgal,
        kungal: enabled.kungal,
        steam: enabled.steam,
        pcgw: enabled.pcgw,
      };
      results = await scrapeGame(query, enabled.vndb, enabled.bangumi, opts);
      selectedIndex = 0;
      progress = [...progress, `完成：${results.length} 条结果`];
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
</script>

<section class="page aura-page" data-aura-echo="SCRAPER">
  <header class="page-head aura-head">
    <div>
      <p class="aura-kicker">Scraper</p>
      <h1 class="aura-title">AI 刮削中心</h1>
      <p>聚合 Galgame 数据源，按策略补齐标题、封面、简介、标签和技术资料。</p>
    </div>
    <div class="head-stats">
      <span><strong class="aura-num">{Object.values(enabled).filter(Boolean).length}</strong> 数据源</span>
      <span><strong class="aura-num">{results.length}</strong> 结果</span>
    </div>
    <button class="primary" disabled={loading} onclick={runScrape}>
      {loading ? "刮削中..." : "开始刮削"}
    </button>
  </header>

  <div class="toolbar">
    <input bind:value={query} placeholder="输入游戏名或 Steam App ID" onkeydown={(event) => event.key === "Enter" && runScrape()} />
    <select bind:value={strategy}>
      <option value="full">完整刮削</option>
      <option value="incremental">增量刮削</option>
      <option value="patch">补缺刮削</option>
    </select>
  </div>

  <div class="content-grid">
    <section class="panel candidate-panel">
      <div class="panel-head">
        <div>
          <p class="aura-kicker">Candidates</p>
          <h2>候选与进度</h2>
        </div>
        <span class="aura-num">{Object.values(enabled).filter(Boolean).length}/{sources.length}</span>
      </div>

      <div class="source-list">
        {#each sources as source}
          <label class="source" class:enabled={enabled[source.key]}>
            <input type="checkbox" bind:checked={enabled[source.key]} />
            <span>{source.label}</span>
            <em>{enabled[source.key] ? "ON" : "OFF"}</em>
          </label>
        {/each}
      </div>

      {#if results.length}
        <div class="candidate-list" aria-label="刮削候选结果">
          {#each results as result, i}
            <button class="candidate-row" class:active={selectedIndex === i} onclick={() => selectedIndex = i}>
              <span class="candidate-copy">
                <strong>{result.title}</strong>
                <small>{result.source} · {result.release_year ?? "年份未知"}</small>
              </span>
              <span class="match aura-num">{matchScore(result, i)}%</span>
            </button>
          {/each}
        </div>
      {/if}

      {#if progress.length}
        <ol class="progress">
          {#each progress as item, i}
            <li><span class="aura-num">{String(i + 1).padStart(2, "0")}</span>{item}</li>
          {/each}
        </ol>
      {:else}
        <EmptyState title="等待任务" description="输入关键词并选择数据源后开始。" />
      {/if}
    </section>

    <section class="panel preview-panel aura-panel--spot" use:spotlight={{ radius: 420 }}>
      <div class="panel-head">
        <div>
          <p class="aura-kicker">Preview</p>
          <h2>结果预览</h2>
        </div>
        <span class="aura-num">{results.length}</span>
      </div>
      {#if selectedResult}
        <article class="preview-card">
          {#if selectedResult.cover}
            <img src={selectedResult.cover} alt={selectedResult.title} />
          {/if}
          <div>
            <div class="preview-title-row">
              <strong>{selectedResult.title}</strong>
              <span class="match aura-num">{matchScore(selectedResult, selectedIndex)}%</span>
            </div>
            <span><code>{selectedResult.source}</code> · <em class="aura-num">{selectedResult.release_year ?? "年份未知"}</em></span>
            <p>{selectedResult.description ?? "暂无简介"}</p>
          </div>
        </article>
      {:else}
        <EmptyState title="暂无结果" description="刮削结果会在这里集中预览。" />
      {/if}
    </section>
  </div>
</section>

<style>
  .page { padding: 24px; overflow: auto; overflow-x: hidden; display: flex; flex-direction: column; gap: 18px; }
  .page-head { display: flex; justify-content: space-between; gap: 16px; align-items: center; }
  .aura-head { align-items: flex-end; }
  .aura-kicker {
    margin: 0 0 6px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--accent);
    text-transform: uppercase;
  }
  .aura-title { margin: 0; }
  .head-stats { margin-left: auto; display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .head-stats span {
    display: inline-flex; align-items: baseline; gap: 5px;
    padding: 7px 10px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--bg-deep); color: var(--text-secondary); font-size: 0.76rem;
  }
  .head-stats strong { color: var(--text-primary); font-size: 0.95rem; }
  h1 { font-size: 24px; }
  p { color: var(--text-secondary); line-height: 1.5; }
  .primary { border: 0; border-radius: 999px; padding: 12px 18px; color: #fff; background: var(--accent); cursor: pointer; font-weight: 600; transition: background 0.18s ease; }
  .primary:hover { background: var(--accent-hi); }
  .toolbar { min-width: 0; max-width: 100%; box-sizing: border-box; display: grid; grid-template-columns: minmax(0, 1fr) 180px; gap: 12px; }
  input, select { background: rgba(255,255,255,.08); color: var(--text-primary); border: 1px solid rgba(255,255,255,.12); border-radius: 12px; padding: 12px 14px; outline: none; }
  .source-list { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
  .source { display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; gap: 10px; align-items: center; padding: 10px 12px; border-radius: 12px; background: rgba(255,255,255,.05); border: 1px solid rgba(255,255,255,.08); }
  .source span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .source em { font-family: var(--font-mono); font-style: normal; font-size: 0.68rem; color: var(--text-muted); }
  .source.enabled { border-color: rgba(255,183,197,.45); background: rgba(255,183,197,.12); }
  .source.enabled em { color: var(--accent); }
  .candidate-list { display: grid; border-top: 1px solid var(--aura-line); }
  .candidate-row {
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 11px 0;
    border: 0;
    border-bottom: 1px solid var(--aura-line);
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
  .match { color: var(--accent); font-size: 0.82rem; }
  .content-grid { min-width: 0; max-width: 100%; display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(320px, 1fr); gap: 16px; min-height: 0; align-items: start; }
  .panel { min-width: 0; max-width: 100%; box-sizing: border-box; border-radius: 18px; padding: 18px; background: rgba(255,255,255,.06); border: 1px solid rgba(255,255,255,.1); }
  .preview-panel { overflow: hidden; }
  .candidate-panel { gap: 14px; }
  .panel-head { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; margin-bottom: 12px; }
  .panel-head .aura-kicker { margin-bottom: 4px; }
  .panel-head > span { color: var(--accent); }
  h2 { font-size: 16px; margin-bottom: 12px; }
  .panel-head h2 { margin-bottom: 0; }
  .progress { list-style: none; padding: 0; color: var(--text-secondary); line-height: 1.7; display: flex; flex-direction: column; gap: 7px; }
  .progress li {
    display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: 10px; align-items: start;
    padding: 8px 10px; border-top: 1px solid var(--border);
  }
  .progress li span { color: var(--accent); }
  .preview-card { display: grid; grid-template-columns: 96px 1fr; gap: 14px; padding: 12px 0; border-top: 1px solid var(--aura-line); background: transparent; }
  .preview-card img { width: 96px; height: 128px; border-radius: 8px; object-fit: cover; }
  .preview-title-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 10px; align-items: start; }
  strong, span { display: block; }
  span { color: var(--accent); font-size: 12px; margin: 4px 0; }
  code, em { font-family: var(--font-mono); font-style: normal; }
  article p { font-size: 13px; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  @media (max-width: 900px) { .content-grid, .toolbar { grid-template-columns: minmax(0, 1fr); } .page-head { align-items: flex-start; flex-direction: column; } }
  @media (max-width: 560px) {
    .page { padding: 18px; }
    .source-list { grid-template-columns: minmax(0, 1fr); }
    .panel-head { flex-direction: column; }
    .head-stats { margin-left: 0; justify-content: flex-start; }
    .preview-card { grid-template-columns: minmax(0, 1fr); }
    .preview-card img { width: 100%; height: auto; max-height: 220px; aspect-ratio: 3 / 4; }
  }

  .page {
    position: relative;
    isolation: isolate;
    min-width: 0;
    background: var(--bg-void);
    color: var(--text-primary);
  }
  .page-head,
  .panel {
    border: 1px solid var(--border);
    background: var(--bg-card);
    box-shadow: var(--shadow-xs);
    border-radius: 8px;
  }
  .page-head { padding: 18px 20px; }
  .panel,
  article,
  .source {
    border-radius: 8px;
  }
  .primary {
    background: var(--accent);
  }
  input,
  select {
    border-radius: 8px;
    background: var(--bg-deep);
    color: var(--text-primary);
    border-color: var(--border);
  }
</style>
