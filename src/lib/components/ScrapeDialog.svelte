<script lang="ts">
  import { uiStore } from "../stores/ui.svelte";
  import { gameStore } from "../stores/games.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { scrapeGames, scrapeGame, fetchFullDetail } from "../api";
  import type { ScrapeResult, ScrapeSourceStatus } from "../api";
  import Icon from "./Icon.svelte";

  let query = $state("");
  let results = $state<ScrapeResult[]>([]);
  let sourceStatus = $state<ScrapeSourceStatus[]>([]);
  let isSearching = $state(false);
  let selectedResult = $state<ScrapeResult | null>(null);
  let applying = $state(false);
  let error = $state("");

  // 自动填充游戏名
  $effect(() => {
    if (uiStore.showScrapeDialog && uiStore.scrapeTargetGameId) {
      const game = gameStore.games.find(
        (g) => g.id === uiStore.scrapeTargetGameId
      );
      if (game) {
        query = game.name;
      }
    }
  });

  async function handleSearch() {
    if (!query.trim()) return;

    isSearching = true;
    error = "";
    results = [];
    sourceStatus = [];

    try {
      const useAi = settingsStore.settings.ai_enabled && settingsStore.settings.ai_api_key;
      const searchFn = useAi ? scrapeGame : scrapeGames;
      const s = settingsStore.settings;
      const resp = await searchFn(
        query,
        s.vndb_enabled,
        s.bangumi_enabled,
        {
          dlsite: s.dlsite_enabled ?? true,
          touchgal: s.touchgal_enabled ?? true,
          erogamescape: s.erogamescape_enabled ?? true,
          ymgal: s.ymgal_enabled ?? true,
          kungal: s.kungal_enabled ?? true,
          steam: s.steam_enabled ?? true,
          pcgw: s.pcgw_enabled ?? true,
        }
      );
      results = resp.results;
      sourceStatus = resp.source_status;
      if (results.length === 0) {
        const failedSources = sourceStatus.filter(s => !s.ok);
        if (failedSources.length > 0) {
          error = `未找到匹配结果（${failedSources.length} 个数据源连接失败，可在设置中配置代理）`;
        } else {
          error = "未找到匹配结果";
        }
      }
    } catch (e) {
      error = `搜索失败: ${e}`;
    } finally {
      isSearching = false;
    }
  }

  async function handleApply() {
    if (!selectedResult || !uiStore.scrapeTargetGameId) return;

    applying = true;
    error = "";
    try {
      // 搜索结果是浅层的；落库前先取「全量详情」（截图/开发商/发行商/流派/别名…）
      let full: ScrapeResult = selectedResult;
      const baseSource = (selectedResult.source || "").replace("+ai", "").trim();
      if (baseSource && selectedResult.source_id) {
        try {
          const detailed = await fetchFullDetail(baseSource, selectedResult.source_id);
          // 全量详情为主，缺的用搜索结果兜底（封面/简介/评分常在搜索结果里更全）
          full = {
            ...detailed,
            cover: detailed.cover ?? selectedResult.cover,
            background: detailed.background ?? selectedResult.background,
            description: detailed.description ?? selectedResult.description,
            rating: detailed.rating ?? selectedResult.rating,
            release_year: detailed.release_year ?? selectedResult.release_year,
            tags: detailed.tags?.length ? detailed.tags : selectedResult.tags,
          };
        } catch (e) {
          // 详情接口失败 → 用搜索结果兜底，不阻断
          console.warn("fetch_full_detail failed, applying shallow result:", e);
        }
      }
      await gameStore.scrape(uiStore.scrapeTargetGameId, full);
      const n = full.detail?.screenshots?.length ?? 0;
      uiStore.notify(n > 0 ? `详情已补全并应用（含 ${n} 张截图）` : "刮削信息已应用！", "success");
      handleClose();
    } catch (e) {
      error = `应用失败: ${e}`;
    } finally {
      applying = false;
    }
  }

  function handleClose() {
    uiStore.closeScrapeDialog();
    query = "";
    results = [];
    sourceStatus = [];
    selectedResult = null;
    error = "";
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") handleClose();
    if (e.key === "Enter") handleSearch();
  }
</script>

{#if uiStore.showScrapeDialog}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onkeydown={handleKeydown}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dialog" onkeydown={handleKeydown}>
      <div class="dialog-header">
        <h2><Icon name="search" size={22} /> AI 刮削游戏信息</h2>
        <button class="close-btn" onclick={handleClose}>✕</button>
      </div>

      <div class="search-section">
        <div class="search-input">
          <input
            type="text"
            placeholder="输入游戏名称搜索..."
            bind:value={query}
            onkeydown={(e) => e.key === "Enter" && handleSearch()}
          />
          <button class="search-btn" onclick={handleSearch} disabled={isSearching}>
            {isSearching ? "搜索中..." : "搜索"}
          </button>
        </div>
        <div class="source-info">
          数据源:
          {#if settingsStore.settings.vndb_enabled}<span class="source-tag vndb">VNDB</span>{/if}
          {#if settingsStore.settings.bangumi_enabled}<span class="source-tag bangumi">Bangumi</span>{/if}
          {#if settingsStore.settings.dlsite_enabled ?? true}<span class="source-tag">DLsite</span>{/if}
          {#if settingsStore.settings.touchgal_enabled ?? true}<span class="source-tag">TouchGAL</span>{/if}
          {#if settingsStore.settings.erogamescape_enabled ?? true}<span class="source-tag">EGS</span>{/if}
          {#if settingsStore.settings.ymgal_enabled ?? true}<span class="source-tag">Ymgal</span>{/if}
          {#if settingsStore.settings.kungal_enabled ?? true}<span class="source-tag">Kungal</span>{/if}
          {#if settingsStore.settings.steam_enabled ?? true}<span class="source-tag">Steam</span>{/if}
          {#if settingsStore.settings.pcgw_enabled ?? true}<span class="source-tag">PCGW</span>{/if}
          {#if settingsStore.settings.ai_enabled && settingsStore.settings.ai_api_key}
            <span class="source-tag ai"><Icon name="lightbulb" size={14} /> AI</span>
          {/if}
        </div>
      </div>

      {#if sourceStatus.length > 0}
        <div class="source-status-bar">
          {#each sourceStatus as st}
            <span class="src-st" class:ok={st.ok} class:fail={!st.ok} title={st.error ?? `${st.count} 条结果`}>
              {st.source.toUpperCase()}
              {#if st.ok}
                <span class="src-count">{st.count}</span>
              {:else}
                <span class="src-fail-mark">!</span>
              {/if}
            </span>
          {/each}
        </div>
      {/if}

      {#if error}
        <div class="error-msg">{error}</div>
      {/if}

      {#if results.length > 0}
        <div class="results-section">
          <h3>搜索结果 ({results.length})</h3>
          <div class="results-list">
            {#each results as result}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div
                class="result-item"
                class:selected={selectedResult === result}
                onclick={() => (selectedResult = result)}
                ondblclick={() => {
                  selectedResult = result;
                  handleApply();
                }}
                onkeydown={(e) => {
                  if (e.key === "Enter") { selectedResult = result; handleApply(); }
                }}
                role="button"
                tabindex="0"
              >
                <div class="result-cover">
                  {#if result.cover}
                    <img src={result.cover} alt={result.title} />
                  {:else}
                    <div class="no-cover">{result.title?.trim()?.[0]?.toUpperCase() ?? "?"}</div>
                  {/if}
                </div>
                <div class="result-info">
                  <h4>{result.title}</h4>
                  {#if result.description}
                    <p class="result-desc">{result.description.slice(0, 100)}...</p>
                  {/if}
                  <div class="result-meta">
                    <span class="source-badge" class:ai-boosted={result.source.includes('+ai')}>
                      {result.source.replace('+ai', '').toUpperCase()}{#if result.source.includes('+ai')}<span class="ai-suffix">AI</span>{/if}
                    </span>
                    {#if result.rating}
                      <span class="rating"><Icon name="star" size={11} /> {result.rating.toFixed(1)}</span>
                    {/if}
                    {#if result.release_year}
                      <span>{result.release_year}</span>
                    {/if}
                  </div>
                  {#if result.tags.length > 0}
                    <div class="result-tags">
                      {#each result.tags.slice(0, 5) as tag}
                        <span class="tag">{tag}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <div class="dialog-footer">
        <button class="cancel-btn" onclick={handleClose}>取消</button>
        <button
          class="apply-btn"
          disabled={!selectedResult || applying}
          onclick={handleApply}
        >
          {applying ? "补全详情中…" : "应用选中结果"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    width: 700px;
    max-height: 80vh;
    background: linear-gradient(135deg, #1a0a2e, #2d1b4e);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .dialog-header h2 {
    font-size: 18px;
    margin: 0;
  }

  .close-btn {
    background: rgba(255, 255, 255, 0.1);
    border: none;
    color: var(--color-text);
    width: 32px;
    height: 32px;
    border-radius: 50%;
    cursor: pointer;
    font-size: 16px;
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  .search-section {
    padding: 20px 24px;
  }

  .search-input {
    display: flex;
    gap: 12px;
  }

  .search-input input {
    flex: 1;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: var(--color-text);
    padding: 12px 16px;
    border-radius: var(--border-radius);
    font-size: 14px;
    outline: none;
  }

  .search-input input:focus {
    border-color: var(--accent-ring);
  }

  .search-btn {
    padding: 12px 24px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--border-radius);
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
    transition: all 0.2s ease;
  }

  .search-btn:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .search-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .source-info {
    margin-top: 10px;
    font-size: 12px;
    color: var(--color-text-secondary);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .source-tag {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 6px;
  }

  .source-tag.vndb {
    background: rgba(76, 175, 80, 0.2);
    color: #81c784;
  }

  .source-tag.bangumi {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }

  .source-tag.ai {
    background: rgba(255, 193, 7, 0.2);
    color: #ffd54f;
  }

  .source-status-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 24px 12px;
  }

  .src-st {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 6px;
    font-family: var(--font-mono, monospace);
  }

  .src-st.ok {
    background: rgba(76, 175, 80, 0.15);
    color: #81c784;
  }

  .src-st.fail {
    background: rgba(255, 80, 80, 0.15);
    color: #ff6b6b;
    cursor: help;
  }

  .src-count {
    font-size: 9px;
    opacity: 0.8;
  }

  .src-fail-mark {
    font-size: 10px;
  }

  .error-msg {
    margin: 0 24px 16px;
    padding: 12px 16px;
    background: rgba(255, 80, 80, 0.15);
    color: #ff6b6b;
    border-radius: 8px;
    font-size: 13px;
  }

  .results-section {
    flex: 1;
    overflow-y: auto;
    padding: 0 24px;
  }

  .results-section h3 {
    font-size: 14px;
    color: var(--color-text-secondary);
    margin-bottom: 12px;
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 350px;
    overflow-y: auto;
  }

  .result-item {
    display: flex;
    gap: 16px;
    padding: 16px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    cursor: pointer;
    border: 2px solid transparent;
    transition: all 0.2s ease;
  }

  .result-item:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .result-item.selected {
    border-color: var(--accent-ring);
    background: var(--accent-lo);
  }

  .result-cover {
    width: 80px;
    height: 110px;
    border-radius: 8px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .result-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .no-cover {
    width: 100%;
    height: 100%;
    background: var(--bg-elev);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 32px;
  }

  .result-info {
    flex: 1;
    min-width: 0;
  }

  .result-info h4 {
    font-size: 15px;
    margin-bottom: 6px;
  }

  .result-desc {
    font-size: 12px;
    color: var(--color-text-secondary);
    line-height: 1.4;
    margin-bottom: 8px;
  }

  .result-meta {
    display: flex;
    gap: 12px;
    align-items: center;
    font-size: 12px;
    color: var(--color-text-secondary);
    margin-bottom: 8px;
  }

  .source-badge {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    font-weight: 600;
  }

  .source-badge.ai-boosted {
    background: rgba(255, 193, 7, 0.15);
    color: #ffd54f;
  }

  .ai-suffix {
    font-size: 9px;
    margin-left: 2px;
  }

  .result-meta .rating {
    color: #ffd700;
  }

  .result-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .result-tags .tag {
    font-size: 10px;
    background: var(--accent-lo);
    color: var(--accent-hi);
    padding: 2px 8px;
    border-radius: 8px;
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 24px;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
  }

  .cancel-btn {
    padding: 10px 20px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: transparent;
    color: var(--color-text);
    border-radius: var(--border-radius);
    cursor: pointer;
    font-size: 14px;
  }

  .cancel-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .apply-btn {
    padding: 10px 24px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--border-radius);
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
  }

  .apply-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .apply-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: var(--shadow-sakura);
  }
</style>
