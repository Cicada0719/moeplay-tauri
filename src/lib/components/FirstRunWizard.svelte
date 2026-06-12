<script lang="ts">
  import { gameStore } from "../stores/games.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { pickDirectory, scanDirectoryForGames } from "../api";
  import Icon from "./Icon.svelte";

  let step = $state(0);
  let saving = $state(false);
  let saveError = $state("");

  // Step 0: directories
  let scanDirs = $state<string[]>([]);
  let scanRunning = $state(false);
  let scanResult = $state<{ imported: number; skipped: number } | null>(null);

  // Step 1: AI
  let aiKey = $state(settingsStore.settings?.ai_api_key ?? "");
  let aiUrl = $state(settingsStore.settings?.ai_api_url ?? "https://api.openai.com/v1/chat/completions");
  let aiModel = $state(settingsStore.settings?.ai_model ?? "gpt-4o-mini");

  // Step 2: sources
  let srcVndb = $state(true);
  let srcBangumi = $state(true);
  let srcDlsite = $state(false);
  let srcTouchgal = $state(true);
  let srcErogescape = $state(false);
  let srcYmgal = $state(true);
  let srcKungal = $state(true);
  let srcSteam = $state(false);
  let srcPcgw = $state(false);

  const steps = [
    { icon: "folder", title: "欢迎使用萌游", desc: "选择包含 galgame 的文件夹，我们会自动扫描并导入游戏库。" },
    { icon: "star", title: "配置 AI 增强（可选）", desc: "AI 用于翻译和智能补全元数据，跳过也可正常使用" },
    { icon: "globe", title: "选择数据源", desc: "启用刮削源，游戏信息将自动从这些平台获取" },
  ];

  const sourceItems = [
    { key: "vndb",      label: "VNDB",            get: () => srcVndb,      set: (v: boolean) => srcVndb = v },
    { key: "bangumi",   label: "Bangumi",         get: () => srcBangumi,   set: (v: boolean) => srcBangumi = v },
    { key: "dlsite",    label: "DLsite",          get: () => srcDlsite,    set: (v: boolean) => srcDlsite = v },
    { key: "touchgal",  label: "TouchGAL",        get: () => srcTouchgal,  set: (v: boolean) => srcTouchgal = v },
    { key: "ymgal",     label: "月幕 Ymgal",       get: () => srcYmgal,     set: (v: boolean) => srcYmgal = v },
    { key: "kungal",    label: "Kungal",          get: () => srcKungal,    set: (v: boolean) => srcKungal = v },
    { key: "steam",     label: "Steam",           get: () => srcSteam,     set: (v: boolean) => srcSteam = v },
    { key: "erogamescape", label: "批评空间",       get: () => srcErogescape, set: (v: boolean) => srcErogescape = v },
    { key: "pcgw",      label: "PCGamingWiki",    get: () => srcPcgw,      set: (v: boolean) => srcPcgw = v },
  ];
  const stepProgressPct = $derived(Math.round(((step + 1) / steps.length) * 100));

  async function addFolder() {
    const dir = await pickDirectory().catch(() => "");
    if (!dir || scanDirs.includes(dir)) return;
    scanDirs = [...scanDirs, dir];
  }

  function removeFolder(dir: string) {
    scanDirs = scanDirs.filter(d => d !== dir);
  }

  async function scanFolders() {
    if (scanDirs.length === 0) return;
    scanRunning = true;
    scanResult = null;
    let totalImported = 0;
    let totalSkipped = 0;
    for (const dir of scanDirs) {
      try {
        const r = await scanDirectoryForGames(dir);
        totalImported += r.imported;
        totalSkipped += r.skipped;
      } catch (e) {
        console.error("Scan failed for", dir, e);
      }
    }
    scanResult = { imported: totalImported, skipped: totalSkipped };
    scanRunning = false;
    await gameStore.load();
  }

  async function saveAndFinish(targetView = "home") {
    saving = true;
    saveError = "";
    try {
      const s = { ...(settingsStore.settings ?? {}) };
      s.ai_api_key = aiKey;
      s.ai_api_url = aiUrl;
      s.ai_enabled = aiKey.length > 0;
      s.ai_model = aiModel;
      s.vndb_enabled = srcVndb;
      s.bangumi_enabled = srcBangumi;
      // Save all source toggles into watch settings
      await settingsStore.save(s);

      // Add scanned dirs as watch dirs
      if (scanDirs.length > 0) {
        for (const dir of scanDirs) {
          if (!s.watch_dirs?.includes(dir)) {
            s.watch_dirs = [...(s.watch_dirs ?? []), dir];
          }
        }
        await settingsStore.save(s);
      }

      // Scan if not done yet
      if (!scanResult && scanDirs.length > 0) {
        await scanFolders();
      }

      uiStore.showFirstRunWizard = false;
      uiStore.currentView = targetView;
    } catch (e) {
      saveError = "保存失败: " + (e instanceof Error ? e.message : String(e));
    } finally {
      saving = false;
    }
  }

  async function finishTo(targetView: string) {
    await saveAndFinish(targetView);
  }
</script>

<div class="wizard-overlay aura-page" data-aura-echo="SETUP" role="dialog" tabindex="-1" aria-label="首次启动向导">
  <div class="wizard aura-panel aura-bevel">
    <header class="aura-head">
      <div class="step-icon">
        <Icon name={steps[step].icon} size={34} stroke={1.25} />
      </div>
      <div class="head-copy">
        <p class="aura-kicker">First Run</p>
        <h2 class="aura-title">{steps[step].title}</h2>
        <p class="desc">{steps[step].desc}</p>
      </div>
      <div class="step-count" aria-label="当前步骤">
        <strong class="aura-num">{String(step + 1).padStart(2, "0")}</strong>
        <span>/</span>
        <span class="aura-num">{String(steps.length).padStart(2, "0")}</span>
      </div>
    </header>

    <div class="wizard-progress" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={stepProgressPct}>
      <div class="progress-track"><span style={`--p: ${stepProgressPct / 100}`}></span></div>
      <div class="step-dots">
        {#each steps as _, i}
          <div class="dot" class:active={i === step} class:done={i < step}></div>
          {#if i < steps.length - 1}
            <div class="line" class:done={i < step}></div>
          {/if}
        {/each}
      </div>
    </div>

    <!-- Step 0: Welcome + directory picker -->
    {#if step === 0}
      <div class="dir-list">
        {#each scanDirs as dir}
          <div class="dir-chip">
            <Icon name="folder" size={14} />
            <span class="dir-path">{dir}</span>
            <button class="dir-remove" onclick={() => removeFolder(dir)} aria-label="移除"><Icon name="x" size={12} /></button>
          </div>
        {/each}
        {#if scanDirs.length === 0}
          <p class="dir-hint">尚未选择任何文件夹 — 点击下方按钮添加，或拖拽文件夹到此处</p>
        {/if}
      </div>

      <button class="btn-primary" onclick={addFolder} disabled={scanRunning}>
        <Icon name="folder" size={18} /> 选择文件夹
      </button>

      {#if scanRunning}
        <div class="scan-status">正在扫描 {scanDirs.length} 个目录...</div>
      {:else if scanResult}
        <div class="scan-status success">
          导入了 {scanResult.imported} 个游戏，跳过 {scanResult.skipped} 个
        </div>
      {/if}

      <div class="actions">
        <button class="btn-ghost" onclick={() => step = 1}>跳过</button>
        {#if scanDirs.length > 0}
          <button class="btn-ghost" onclick={scanFolders} disabled={scanRunning}>立即扫描</button>
        {/if}
        <button class="btn-primary" onclick={() => step = 1}>下一步</button>
      </div>

    <!-- Step 1: AI config -->
    {:else if step === 1}
      <div class="form">
        <label for="wiz-api-url">API 地址</label>
        <input id="wiz-api-url" type="text" bind:value={aiUrl} placeholder="https://api.openai.com/v1/chat/completions" class="input" />
        <label for="wiz-api-key">API Key</label>
        <input id="wiz-api-key" type="password" bind:value={aiKey} placeholder="sk-..." class="input" />
        <label for="wiz-model">模型</label>
        <select id="wiz-model" bind:value={aiModel} class="input">
          <option value="gpt-4o-mini">GPT-4o-mini</option>
          <option value="gpt-4o">GPT-4o</option>
          <option value="deepseek-chat">DeepSeek V3</option>
          <option value="claude-3-5-sonnet">Claude 3.5 Sonnet</option>
          <option value="qwen2.5:7b">Ollama 本地</option>
        </select>
      </div>

      <div class="actions">
        <button class="btn-ghost" onclick={() => step = 0}>上一步</button>
        <button class="btn-ghost" onclick={() => { aiKey = ""; step = 2; }}>跳过</button>
        <button class="btn-primary" onclick={() => step = 2}>下一步</button>
      </div>

    <!-- Step 2: Data sources + finish -->
    {:else if step === 2}
      <div class="source-grid">
        {#each sourceItems as item}
          <label class="source-item" class:on={item.get()}>
            <input type="checkbox" checked={item.get()} onchange={(e) => item.set((e.target as HTMLInputElement).checked)} />
            <span>{item.label}</span>
          </label>
        {/each}
      </div>

      <div class="finish-cta" aria-label="导入入口">
        <button class="entry-card" onclick={() => scanDirs.length > 0 ? saveAndFinish("home") : step = 0} disabled={saving}>
          <Icon name="folder" size={20} />
          <strong>{scanDirs.length > 0 ? "保存并扫描本地" : "添加本地目录"}</strong>
          <span>从当前电脑的游戏文件夹开始建库</span>
        </button>
        <button class="entry-card" onclick={() => finishTo("steam-import")} disabled={saving}>
          <Icon name="download" size={20} />
          <strong>Steam / Epic 导入</strong>
          <span>同步平台库、游玩时长和封面</span>
        </button>
        <button class="entry-card" onclick={() => finishTo("migration")} disabled={saving}>
          <Icon name="database" size={20} />
          <strong>旧版迁移桥</strong>
          <span>从 MoeGame / Playnite 旧库恢复数据</span>
        </button>
      </div>

      {#if saveError}
        <div class="save-error">{saveError}</div>
      {/if}

      <div class="actions">
        <button class="btn-ghost" onclick={() => step = 1}>上一步</button>
        <button class="btn-primary" onclick={() => saveAndFinish("home")} disabled={saving}>
          {saving ? "保存中..." : "开始使用"}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .wizard-overlay {
    position: fixed; inset: 0; z-index: 200;
    display: flex; align-items: center; justify-content: center;
    background: rgba(0,0,0,0.55);
  }
  .wizard {
    width: 560px; max-width: 92vw; max-height: 90vh; overflow-y: auto;
    padding: 36px 32px;
    display: flex; flex-direction: column; align-items: stretch; gap: 14px;
    animation: fadeInScale 0.35s ease-out;
    text-align: left;
    position: relative;
    clip-path: polygon(0 0, calc(100% - 28px) 0, 100% 28px, 100% 100%, 0 100%);
  }
  .wizard::before {
    content: "";
    position: absolute;
    right: 0;
    top: 0;
    width: 28px;
    height: 28px;
    border-left: 1px solid var(--aura-border-strong);
    background: var(--accent-lo);
    pointer-events: none;
  }
  .aura-head {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr) auto;
    align-items: start;
    gap: 14px;
  }
  .step-icon {
    width: 48px;
    height: 48px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--aura-inset);
    color: var(--accent);
    display: grid;
    place-items: center;
  }
  .aura-kicker {
    margin: 0 0 6px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--accent);
    text-transform: uppercase;
  }
  .aura-title { margin: 0; }
  .step-count {
    display: flex;
    align-items: baseline;
    gap: 4px;
    color: var(--text-muted);
    font-size: 0.82rem;
  }
  .step-count strong { color: var(--text-primary); font-size: 1.15rem; }
  .wizard-progress { display: flex; flex-direction: column; gap: 9px; }
  .progress-track {
    height: 6px;
    border-radius: 999px;
    background: var(--bg-hover);
    overflow: hidden;
  }
  .progress-track span {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transform: scaleX(var(--p, 0));
    transform-origin: left center;
    transition: transform 0.25s ease;
    will-change: transform;
  }
  .step-dots { display: flex; align-items: center; gap: 0; margin-bottom: 4px; justify-content: center; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--bg-hover); transition: all 0.3s; }
  .dot.active { background: var(--accent); box-shadow: 0 0 8px var(--accent-lo); }
  .dot.done { background: var(--accent); }
  .line { width: 40px; height: 2px; background: var(--bg-hover); transition: all 0.3s; }
  .line.done { background: var(--accent); }
  h2 { font-size: 1.2rem; font-weight: 650; color: var(--text-primary); }
  .desc { color: var(--text-secondary); font-size: 0.88rem; line-height: 1.55; max-width: 420px; margin: 6px 0 0; }

  .dir-list {
    width: 100%; display: flex; flex-direction: column; gap: 6px;
    min-height: 48px; border: 1px dashed var(--border); border-radius: var(--radius-md);
    padding: 8px 10px; background: var(--bg-secondary);
  }
  .dir-chip {
    display: flex; align-items: center; gap: 6px;
    padding: 5px 8px; border-radius: var(--radius-sm);
    background: var(--bg-elev); border: 1px solid var(--border);
    font-size: 0.78rem;
  }
  .dir-path {
    flex: 1; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .dir-remove {
    background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 2px; display: flex;
  }
  .dir-remove:hover { color: var(--color-error); }
  .dir-hint { font-size: 0.75rem; color: var(--text-muted); padding: 12px 4px; margin: 0; }

  .form { width: 100%; display: flex; flex-direction: column; gap: 8px; text-align: left; }
  .form label { font-size: 0.8rem; color: var(--text-muted); font-weight: 500; }
  .input {
    width: 100%; padding: 10px 12px; border-radius: var(--radius-md);
    background: var(--bg-secondary); border: 1px solid var(--border);
    color: var(--text-primary); font-family: var(--font-mono); font-size: 0.85rem;
    transition: border-color 0.18s;
  }
  .input:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent-lo); }
  select.input { font-family: var(--font-ui); }

  .actions { display: flex; gap: 10px; margin-top: 6px; width: 100%; justify-content: center; flex-wrap: wrap; }
  .btn-primary {
    padding: 10px 24px; border: none; border-radius: var(--radius-md);
    background: var(--accent); color: #fff; font-weight: 600; cursor: pointer;
    font-size: 0.9rem; transition: background 0.18s; display: inline-flex; align-items: center; gap: 6px;
  }
  .btn-primary:hover:not(:disabled) { background: var(--accent-hi); }
  .btn-primary:active:not(:disabled) { transform: translateY(1px); }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-ghost {
    padding: 10px 18px; border: 1px solid var(--border); border-radius: var(--radius-md);
    background: transparent; color: var(--text-secondary); cursor: pointer;
    font-size: 0.9rem; transition: all 0.2s;
  }
  .btn-ghost:hover { border-color: var(--accent); color: var(--text-primary); }

  .scan-status { font-size: 0.85rem; padding: 8px 14px; border-radius: var(--radius-sm); background: var(--bg-hover); color: var(--text-secondary); text-align: center; }
  .scan-status.success { background: rgba(34,197,94,0.1); color: var(--color-success); }

  .save-error { font-size: 0.8rem; padding: 8px 14px; border-radius: var(--radius-sm); background: rgba(239,68,68,0.1); color: var(--color-error); margin-top: 2px; width: 100%; }

  .source-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; width: 100%; }
  .source-item {
    display: flex; align-items: center; gap: 8px; padding: 8px 12px;
    border-radius: var(--radius-sm); background: var(--bg-secondary); border: 1px solid transparent;
    cursor: pointer; transition: all 0.2s; font-size: 0.85rem; color: var(--text-secondary);
  }
  .source-item.on { background: var(--accent-lo); border-color: var(--accent); color: var(--accent); }
  .source-item input { accent-color: var(--accent); margin: 0; }

  .finish-cta {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    margin-top: 2px;
  }
  .entry-card {
    width: 100%;
    display: grid;
    grid-template-columns: 24px 1fr;
    grid-template-rows: auto auto;
    gap: 2px 10px;
    align-items: center;
    text-align: left;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-elev);
    color: var(--text-primary);
    cursor: pointer;
    transition: border-color 0.18s ease, transform 0.18s ease, background 0.18s ease;
  }
  .entry-card:hover:not(:disabled) {
    transform: translateY(-1px);
    border-color: var(--accent-ring);
    background: var(--bg-hover);
  }
  .entry-card:disabled { opacity: 0.65; cursor: not-allowed; }
  .entry-card :global(.icon) {
    grid-row: 1 / span 2;
    color: var(--accent);
  }
  .entry-card strong {
    font-size: 0.86rem;
    font-weight: 650;
  }
  .entry-card span {
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.35;
  }
  @media (max-width: 600px) {
    .aura-head {
      grid-template-columns: 42px minmax(0, 1fr);
    }
    .step-count {
      grid-column: 2;
    }
  }

  @keyframes fadeInScale { from { opacity: 0; transform: scale(0.95); } to { opacity: 1; transform: scale(1); } }
</style>
