<script lang="ts">
  import { gsap } from "gsap";
  import { gameStore, type Game } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";
  import CachedImage from "./CachedImage.svelte";
  import { coverOf, gameRating, tagsOf, platformOf } from "../utils/game";

  let { open = $bindable(false) }: { open: boolean } = $props();

  const allGames = $derived(gameStore.allGames);

  let filterTag = $state("");
  let filterPlatform = $state("");
  let minRating = $state(0);
  let unplayedOnly = $state(false);
  let spinning = $state(false);
  let result = $state<Game | null>(null);
  let displayGame = $state<Game | null>(null);
  let recentResults = $state<string[]>([]);
  let loadedRecent = $state(false);

  const allTags = $derived(
    [...new Set(allGames.flatMap(g => tagsOf(g)))].sort().slice(0, 30)
  );
  const allPlatforms = $derived(
    [...new Set(allGames.map(g => platformOf(g)).filter(Boolean))].sort()
  );

  const filteredPool = $derived(
    allGames.filter(g => {
      if (filterTag && !tagsOf(g).includes(filterTag)) return false;
      if (filterPlatform && platformOf(g) !== filterPlatform) return false;
      if (minRating > 0 && gameRating(g) < minRating) return false;
      if (unplayedOnly && g.play_time_seconds > 0) return false;
      return true;
    })
  );

  function pickRandom(): Game {
    const pool = filteredPool.filter(g => !recentResults.includes(g.id));
    if (pool.length === 0) return filteredPool[Math.floor(Math.random() * filteredPool.length)];
    return pool[Math.floor(Math.random() * pool.length)];
  }

  async function spin() {
    if (spinning || filteredPool.length === 0) return;
    spinning = true;
    result = null;

    const chosen = pickRandom();

    // 双信号降级：OS 偏好或应用内 data-motion="reduce" 时跳过翻牌动画，直接给出结果。
    const reduceMotion =
      window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches ||
      document.documentElement.dataset.motion === "reduce";
    if (reduceMotion) {
      displayGame = chosen;
      result = chosen;
      spinning = false;
      recentResults = [...recentResults, chosen.id].slice(-5);
      try {
        localStorage.setItem("moeplay-wtp-recent", JSON.stringify(recentResults));
      } catch {}
      return;
    }

    const tl = gsap.timeline();

    // Rapid card flip phase
    for (let i = 0; i < 16; i++) {
      tl.call(() => {
        const g = filteredPool[Math.floor(Math.random() * filteredPool.length)];
        displayGame = g;
      }, [], "+=" + (0.04 + i * 0.008));
    }

    // Final reveal
    tl.call(() => {
      displayGame = chosen;
      result = chosen;
      spinning = false;

      // Track recent results (avoid repeats)
      recentResults = [...recentResults, chosen.id].slice(-5);

      // Save to localStorage
      try {
        localStorage.setItem("moeplay-wtp-recent", JSON.stringify(recentResults));
      } catch {}
    });

    // Scale-up animation on result
    tl.from(".wtp-result-card", {
      scale: 0.9,
      duration: 0.3,
      ease: "back.out(2)",
    });
  }

  function launchGame() {
    if (!result) return;
    gameStore.launch(result.id);
    open = false;
  }
  function viewDetail() {
    if (!result) return;
    gameStore.selectGame(result.id);
    uiStore.currentView = "game-detail";
    open = false;
  }

  // Load recent results on mount
  $effect(() => {
    if (loadedRecent) return;
    loadedRecent = true;
    try {
      const saved = localStorage.getItem("moeplay-wtp-recent");
      if (saved) recentResults = JSON.parse(saved);
    } catch {}
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="wtp-overlay" onclick={() => (open = false)} onkeydown={(e) => e.key === "Escape" && (open = false)} role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="wtp-modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="document">
      <header class="wtp-header">
        <h2><Icon name="zap" size={20} /> 今天玩什么？</h2>
        <button class="wtp-close" onclick={() => (open = false)} aria-label="关闭">
          <Icon name="x" size={18} />
        </button>
      </header>

      <div class="wtp-filters">
        <label class="wtp-filter-item">
          <span>标签</span>
          <select bind:value={filterTag}>
            <option value="">全部</option>
            {#each allTags as tag}
              <option value={tag}>{tag}</option>
            {/each}
          </select>
        </label>
        <label class="wtp-filter-item">
          <span>平台</span>
          <select bind:value={filterPlatform}>
            <option value="">全部</option>
            {#each allPlatforms as p}
              <option value={p}>{p}</option>
            {/each}
          </select>
        </label>
        <label class="wtp-filter-item">
          <span>最低评分</span>
          <input type="range" min="0" max="10" step="0.5" bind:value={minRating} />
          <span class="wtp-rating-val">{minRating > 0 ? minRating.toFixed(1) : "不限"}</span>
        </label>
        <label class="wtp-filter-check">
          <input type="checkbox" bind:checked={unplayedOnly} />
          <span>仅未玩过</span>
        </label>
      </div>

      <p class="wtp-pool-info">从 <b>{filteredPool.length}</b> 款游戏中随机选择</p>

      <div class="wtp-stage">
        {#if displayGame}
          <div class="wtp-result-card" class:spinning>
            <div class="wtp-cover">
              {#if coverOf(displayGame)}
                <CachedImage source={coverOf(displayGame)} cacheKey={`wtp-${displayGame.id}`} alt={displayGame.name} />
              {:else}
                <div class="wtp-cover-placeholder">
                  {(displayGame.name?.[0] ?? "?").toUpperCase()}
                </div>
              {/if}
            </div>
            <div class="wtp-game-info">
              <h3>{displayGame.name}</h3>
              {#if result}
                <div class="wtp-result-actions">
                  <button class="wtp-btn primary" onclick={launchGame}>
                    <Icon name="play" size={16} /> 启动游戏
                  </button>
                  <button class="wtp-btn" onclick={spin}>
                    <Icon name="refresh" size={16} /> 再来一次
                  </button>
                  <button class="wtp-btn" onclick={viewDetail}>
                    <Icon name="info" size={16} /> 查看详情
                  </button>
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <div class="wtp-empty-stage">
            <Icon name="gamepad" size={48} />
            <p>点击下方按钮开始抽取</p>
          </div>
        {/if}
      </div>

      <button class="wtp-spin-btn" onclick={spin} disabled={spinning || filteredPool.length === 0}>
        {#if spinning}
          抽选中...
        {:else}
          <Icon name="zap" size={18} /> 开始抽选
        {/if}
      </button>
    </div>
  </div>
{/if}

<style>
  .wtp-overlay {
    position: fixed; inset: 0; z-index: 1000;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    display: flex; align-items: center; justify-content: center;
    animation: fade-in 0.2s ease;
  }
  .wtp-modal {
    width: 440px; max-width: 92vw; max-height: 90vh;
    background: var(--bg-elev, #1a1d28);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 24px;
    overflow-y: auto;
    box-shadow: 0 24px 64px rgba(0,0,0,0.5);
  }

  .wtp-header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 20px;
  }
  .wtp-header h2 {
    display: flex; align-items: center; gap: 8px;
    font-size: 18px; font-weight: 700; color: var(--text-primary);
    margin: 0;
  }
  .wtp-close {
    display: flex; align-items: center; justify-content: center;
    width: 32px; height: 32px; border: none; border-radius: 8px;
    background: rgba(255,255,255,0.06); color: var(--text-muted);
    cursor: pointer; transition: all 0.15s;
  }
  .wtp-close:hover { background: rgba(255,255,255,0.12); color: var(--text-primary); }

  .wtp-filters {
    display: flex; flex-wrap: wrap; gap: 12px; margin-bottom: 16px;
  }
  .wtp-filter-item {
    display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 120px;
  }
  .wtp-filter-item span:first-child {
    font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em;
  }
  .wtp-filter-item select,
  .wtp-filter-item input[type="range"] {
    padding: 6px 8px; border: 1px solid var(--border); border-radius: 6px;
    background: var(--bg-card, #12151e); color: var(--text-primary); font-size: 13px;
  }
  .wtp-rating-val { font-size: 12px; color: var(--text-secondary); text-align: right; }
  .wtp-filter-check {
    display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-secondary);
    align-self: flex-end; cursor: pointer;
  }
  .wtp-filter-check input { accent-color: var(--accent); }

  .wtp-pool-info {
    font-size: 13px; color: var(--text-muted); margin-bottom: 20px; text-align: center;
  }
  .wtp-pool-info b { color: var(--accent); }

  .wtp-stage {
    min-height: 200px; display: flex; align-items: center; justify-content: center;
    margin-bottom: 20px;
  }
  .wtp-result-card {
    display: flex; gap: 20px; align-items: center;
    width: 100%;
  }
  .wtp-result-card.spinning { opacity: 0.7; }
  .wtp-cover {
    width: 140px; height: 187px; flex-shrink: 0;
    border-radius: 12px; overflow: hidden;
    background: var(--bg-hover);
    box-shadow: 0 8px 32px rgba(0,0,0,0.3);
  }
  .wtp-cover :global(.cached-image) { width: 100%; height: 100%; object-fit: cover; }
  .wtp-cover-placeholder {
    width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;
    font-size: 36px; font-weight: 700; color: var(--text-muted);
    background: linear-gradient(135deg, rgba(255,126,173,.2), rgba(167,139,250,.2));
  }
  .wtp-game-info { flex: 1; }
  .wtp-game-info h3 {
    font-size: 18px; font-weight: 700; color: var(--text-primary);
    margin: 0 0 16px; line-height: 1.3;
  }
  .wtp-result-actions { display: flex; flex-direction: column; gap: 8px; }

  .wtp-btn {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 8px 16px; border: 1px solid var(--border); border-radius: 8px;
    background: rgba(255,255,255,0.04); color: var(--text-secondary);
    font-size: 13px; cursor: pointer; transition: all 0.15s;
  }
  .wtp-btn:hover { border-color: var(--accent); color: var(--accent); }
  .wtp-btn.primary {
    background: var(--accent); border-color: var(--accent);
    color: #fff; font-weight: 600;
  }
  .wtp-btn.primary:hover { filter: brightness(1.1); }

  .wtp-empty-stage {
    text-align: center; color: var(--text-muted);
  }
  .wtp-empty-stage p { margin-top: 12px; font-size: 14px; }

  .wtp-spin-btn {
    width: 100%; padding: 14px; border: none; border-radius: 12px;
    background: linear-gradient(135deg, var(--accent, #e8557f), #a78bfa);
    color: #fff; font-size: 16px; font-weight: 700;
    display: flex; align-items: center; justify-content: center; gap: 8px;
    cursor: pointer; transition: all 0.2s;
  }
  .wtp-spin-btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
  .wtp-spin-btn:active { transform: translateY(0); }
  .wtp-spin-btn:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }

  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }

  @media (prefers-reduced-motion: reduce) {
    .wtp-overlay { animation: none; }
    .wtp-close, .wtp-btn, .wtp-spin-btn { transition: none; }
  }
  :global([data-motion="reduce"]) .wtp-overlay { animation: none; }
  :global([data-motion="reduce"]) .wtp-close,
  :global([data-motion="reduce"]) .wtp-btn,
  :global([data-motion="reduce"]) .wtp-spin-btn { transition: none; }
</style>
