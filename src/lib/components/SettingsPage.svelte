<script lang="ts">
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { updateNsfwDisplayMode, setAutostart, type NsfwDisplayMode } from "../api";
  import Button from "./ui/Button.svelte";
  import Icon from "./Icon.svelte";

  const modes: { id: NsfwDisplayMode; label: string }[] = [
    { id: "blur", label: "模糊" },
    { id: "show", label: "显示" },
    { id: "hide", label: "隐藏" },
  ];

  const themes = [
    { id: "dark", label: "深色", icon: "home" },
    { id: "light", label: "浅色", icon: "lightbulb" },
    { id: "sakura", label: "樱夜", icon: "heart" },
  ];

  const scrapeSources = [
    { key: "vndb_enabled" as const, label: "VNDB", desc: "视觉小说数据库，获取 VN 元数据" },
    { key: "bangumi_enabled" as const, label: "Bangumi", desc: "番组计划，获取游戏/动漫元数据" },
    { key: "dlsite_enabled" as const, label: "DLsite", desc: "同人/商业游戏销售平台" },
    { key: "kungal_enabled" as const, label: "Kungal", desc: "中文 Galgame 信息聚合站" },
    { key: "steam_enabled" as const, label: "Steam", desc: "Steam 商店页面元数据" },
    { key: "pcgw_enabled" as const, label: "PCGamingWiki", desc: "PC 游戏技术资料百科" },
    { key: "erogamescape_enabled" as const, label: "批评空间", desc: "ErogameScape 日本玩家评分" },
    { key: "ymgal_enabled" as const, label: "月幕 Ymgal", desc: "Galgame 中文社区平台" },
    { key: "touchgal_enabled" as const, label: "TouchGAL", desc: "中文触控 Galgame 信息" },
  ];

  let savingAutostart = $state(false);
  let activeAnchor = $state("settings-basic");

  function isSourceEnabled(key: string): boolean {
    return !!(settingsStore.settings as any)[key];
  }

  async function save() {
    await settingsStore.save(settingsStore.settings);
  }

  async function setNsfw(mode: NsfwDisplayMode) {
    const settings = await updateNsfwDisplayMode(mode);
    await settingsStore.save(settings);
  }

  async function toggleAutostart(enabled: boolean) {
    savingAutostart = true;
    try {
      const mode = settingsStore.settings.startup_mode ?? "dashboard";
      const msg = await setAutostart(enabled, mode);
      await settingsStore.save({ ...settingsStore.settings, autostart_enabled: enabled, startup_mode: mode });
      uiStore.notify(msg, "success");
    } catch (e) {
      uiStore.notify("设置失败: " + String(e), "error");
    } finally {
      savingAutostart = false;
    }
  }

  async function setStartupMode(mode: string) {
    await settingsStore.save({ ...settingsStore.settings, startup_mode: mode });
    if (settingsStore.settings.autostart_enabled) {
      try { await setAutostart(true, mode); } catch { /* ignore */ }
    }
  }

  async function toggleScrapeSetting(key: string) {
    await settingsStore.save({ ...settingsStore.settings, [key]: !isSourceEnabled(key) });
    uiStore.notify("设置已保存", "success");
  }

  async function setAllSources(enabled: boolean) {
    const patch: Record<string, boolean> = {};
    for (const src of scrapeSources) patch[src.key] = enabled;
    await settingsStore.save({ ...settingsStore.settings, ...patch });
    uiStore.notify(enabled ? "已启用全部数据源" : "已关闭全部数据源", "success");
  }

  async function handleThemeChange(theme: string) {
    await settingsStore.save({ ...settingsStore.settings, theme });
    uiStore.notify(`主题已切换`, "success");
  }

  function onAnchorClick(id: string) {
    activeAnchor = id;
  }

  const anchorItems = [
    { id: "settings-basic", idx: "01", label: "基础" },
    { id: "settings-scrape", idx: "02", label: "刮削" },
    { id: "settings-startup", idx: "03", label: "启动" },
    { id: "settings-ai", idx: "04", label: "AI API" },
    { id: "settings-library", idx: "05", label: "库目录" },
    { id: "settings-import", idx: "06", label: "库务入口" },
    { id: "settings-about", idx: "07", label: "关于" },
  ];
</script>

<section class="page aura-page settings-page" data-aura-echo="SETTINGS">
  <header class="aura-head aura-bevel">
    <div class="head-copy">
      <span class="aura-kicker">System Settings</span>
      <h1 class="aura-title">设置</h1>
      <p>主题、AI、扫描目录和导入工具配置。</p>
    </div>
  </header>

  <div class="settings-layout">
    <aside class="settings-anchors aura-panel" aria-label="设置分组目录">
      {#each anchorItems as anchor}
        <a
          href="#{anchor.id}"
          class:active={activeAnchor === anchor.id}
          onclick={() => onAnchorClick(anchor.id)}
        >
          <span class="anchor-index">{anchor.idx}</span><span>{anchor.label}</span>
        </a>
      {/each}
    </aside>

    <main class="settings-groups">
      <!-- 01 基础 -->
      <section id="settings-basic" class="panel aura-panel setting-group aura-enter">
        <div class="panel-head">
          <div>
            <span class="group-kicker">Display</span>
            <h2>基础显示</h2>
          </div>
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">主题</span>
            <span class="setting-desc">选择界面色彩方案</span>
          </div>
          <div class="segmented theme-seg">
            {#each themes as theme}
              <button
                class:active={settingsStore.settings.theme === theme.id}
                onclick={() => handleThemeChange(theme.id)}
              >
                <Icon name={theme.icon} size={14} />{theme.label}
              </button>
            {/each}
          </div>
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">NSFW</span>
            <span class="setting-desc">控制敏感封面的显示方式</span>
          </div>
          <div class="segmented">
            {#each modes as mode}
              <button class:active={settingsStore.settings.nsfw_display_mode === mode.id} onclick={() => setNsfw(mode.id)}>{mode.label}</button>
            {/each}
          </div>
        </div>
      </section>

      <!-- 02 刮削 -->
      <section id="settings-scrape" class="panel aura-panel setting-group aura-enter">
        <div class="panel-head">
          <div>
            <span class="group-kicker">Metadata</span>
            <h2>数据刮削</h2>
          </div>
          <p class="desc">选择从哪些数据库抓取游戏元数据（封面、简介、标签等）。</p>
        </div>
        <div class="source-actions">
          <Button variant="secondary" size="sm" onclick={() => setAllSources(true)}>全开</Button>
          <Button variant="secondary" size="sm" onclick={() => setAllSources(false)}>全关</Button>
        </div>
        <div class="toggle-grid">
          {#each scrapeSources as src}
            <div class="toggle-row">
              <div class="toggle-info">
                <span class="toggle-label">{src.label}</span>
                <span class="toggle-sub">{src.desc}</span>
              </div>
              <label class="sw">
                <input type="checkbox" checked={isSourceEnabled(src.key)} onchange={() => toggleScrapeSetting(src.key)} />
                <span class="sw-track"><span class="sw-thumb"></span></span>
              </label>
            </div>
          {/each}
        </div>
        <div class="toggle-row auto-row">
          <div class="toggle-info">
            <span class="toggle-label">自动刮削</span>
            <span class="toggle-sub">添加游戏时自动搜索并填充元数据</span>
          </div>
          <label class="sw">
            <input type="checkbox" checked={settingsStore.settings.auto_scrape} onchange={() => toggleScrapeSetting("auto_scrape")} />
            <span class="sw-track"><span class="sw-thumb"></span></span>
          </label>
        </div>
      </section>

      <!-- 03 启动 -->
      <section id="settings-startup" class="panel aura-panel setting-group aura-enter">
        <div class="panel-head">
          <div>
            <span class="group-kicker">Startup</span>
            <h2>开机与启动</h2>
          </div>
          <p class="desc">开机自动启动萌游，并选择启动时的显示模式。</p>
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">开机自动启动</span>
            <span class="setting-desc">系统启动时自动打开萌游</span>
          </div>
          <label class="toggle">
            <input
              type="checkbox"
              checked={settingsStore.settings.autostart_enabled ?? false}
              onchange={(e) => toggleAutostart((e.target as HTMLInputElement).checked)}
              disabled={savingAutostart}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <div class="setting-row">
          <div class="setting-info">
            <span class="setting-label">默认启动模式</span>
            <span class="setting-desc">选择打开萌游时进入的界面模式</span>
          </div>
          <div class="segmented">
            <button
              class:active={(settingsStore.settings.startup_mode ?? "dashboard") === "dashboard"}
              onclick={() => setStartupMode("dashboard")}
            >普通模式</button>
            <button
              class:active={settingsStore.settings.startup_mode === "big-picture"}
              onclick={() => setStartupMode("big-picture")}
            >大屏模式</button>
          </div>
        </div>
      </section>

      <!-- 04 AI API -->
      <section id="settings-ai" class="panel aura-panel setting-group aura-enter">
        <div class="panel-head">
          <div>
            <span class="group-kicker">Assistant</span>
            <h2>AI API</h2>
          </div>
        </div>
        <div class="form">
          <div class="form-row">
            <label class="checkbox-row">启用 <input type="checkbox" bind:checked={settingsStore.settings.ai_enabled} onchange={save} /></label>
          </div>
          <label class="form-row">
            <span>API 地址</span>
            <input bind:value={settingsStore.settings.ai_api_url} onblur={save} placeholder="API 地址" />
          </label>
          <label class="form-row">
            <span>API Key</span>
            <input bind:value={settingsStore.settings.ai_api_key} onblur={save} placeholder="API Key" type="password" />
          </label>
          <label class="form-row">
            <span>模型</span>
            <input bind:value={settingsStore.settings.ai_model} onblur={save} placeholder="模型" />
          </label>
        </div>
      </section>

      <!-- 05 库目录 -->
      <section id="settings-library" class="panel aura-panel setting-group aura-enter">
        <div class="panel-head">
          <div>
            <span class="group-kicker">Library</span>
            <h2>扫描目录</h2>
          </div>
        </div>
        <div class="dirs">
          {#each settingsStore.settings.watch_dirs as dir}
            <article>
              <span>{dir}</span>
              <button onclick={() => settingsStore.removeWatchDir(dir)}>移除</button>
            </article>
          {/each}
        </div>
        <Button variant="primary" onclick={() => settingsStore.addWatchDir()}>添加目录</Button>
      </section>

      <!-- 06 库务入口 -->
      <section id="settings-import" class="panel aura-panel setting-group aura-enter">
        <div class="panel-head">
          <div>
            <span class="group-kicker">Operations</span>
            <h2>库务入口</h2>
          </div>
        </div>
        <div class="action-list">
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-label">Steam / Epic 导入</span>
              <span class="setting-desc">扫描本机 Steam / Epic 已安装游戏；需要完整 Steam 账户库时可继续使用登录和 Web API 同步。</span>
            </div>
            <Button variant="primary" onclick={() => uiStore.currentView = "steam-import"}>打开导入</Button>
          </div>
          <div class="setting-row">
            <div class="setting-info">
              <span class="setting-label">从旧版萌游迁移</span>
              <span class="setting-desc">如果你使用过 C# 版萌游，可以一键迁移游戏库和元数据。</span>
            </div>
            <Button variant="primary" onclick={() => uiStore.currentView = "migration"}>开始迁移</Button>
          </div>
        </div>
      </section>

      <!-- 07 关于 -->
      <section id="settings-about" class="panel aura-panel setting-group aura-enter">
        <div class="panel-head">
          <div>
            <span class="group-kicker">About</span>
            <h2>关于</h2>
          </div>
        </div>
        <div class="about-info">
          <p><strong>萌游 MoeGame</strong> v0.1.1</p>
          <p>可爱的游戏管理器</p>
          <p class="tech-stack">Tauri v2 + Svelte 5 + Rust</p>
        </div>
      </section>
    </main>
  </div>
</section>

<style>
  .page { height: 100%; padding: 24px; overflow: auto; display: flex; flex-direction: column; gap: 18px; }
  h1, h2, p { margin: 0; }
  h2 { font-size: 16px; color: var(--text-primary); }
  .desc { min-width: 0; color: var(--text-muted); font-size: 0.85rem; line-height: 1.55; }
  .aura-head { position: relative; padding: 20px 22px; overflow: hidden; }
  .aura-bevel::before {
    content: "";
    position: absolute;
    inset: 10px auto 10px 0;
    width: 3px;
    background: linear-gradient(180deg, var(--aura-data-a), var(--aura-data-b));
  }
  .head-copy { min-width: 0; display: grid; gap: 6px; }
  .aura-kicker,
  .group-kicker,
  .anchor-index {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 750;
    letter-spacing: 0;
    text-transform: uppercase;
  }
  .aura-title { color: var(--text-primary); font-size: 28px; font-weight: 700; line-height: 1.1; }
  .aura-head p { color: var(--text-secondary); line-height: 1.55; }
  .settings-layout {
    min-width: 0;
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    gap: 18px;
    align-items: start;
  }
  .settings-anchors {
    position: sticky;
    top: 0;
    z-index: 1;
    min-width: 0;
    padding: 10px;
    display: grid;
    gap: 4px;
  }
  .settings-anchors a {
    min-width: 0;
    min-height: 38px;
    border-radius: var(--radius-full);
    padding: 0 10px;
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    color: var(--text-secondary);
    text-decoration: none;
    font-size: 13px;
    position: relative;
  }
  .settings-anchors a::before {
    content: "";
    position: absolute;
    inset: 6px auto 6px 0;
    width: 3px;
    border-radius: 3px;
    background: transparent;
    transition: background 0.2s ease;
  }
  .settings-anchors a:hover {
    color: var(--text-primary);
    background: var(--bg-elev);
  }
  .settings-anchors a.active {
    color: var(--text-primary);
    background: var(--bg-elev);
  }
  .settings-anchors a.active::before {
    background: var(--accent);
  }
  .settings-groups {
    min-width: 0;
    display: grid;
    gap: 14px;
  }
  .panel { min-width: 0; padding: 18px; }
  .panel-head {
    min-width: 0;
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 14px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--aura-line);
  }
  .panel-head > div { min-width: 0; display: grid; gap: 4px; }
  .setting-row,
  .toggle-row {
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    align-items: center;
    padding: 13px 0;
    border-bottom: 1px solid var(--aura-line);
  }
  .setting-row:last-child,
  .toggle-row:last-child { border-bottom: 0; padding-bottom: 0; }
  .auto-row { border-top: 1px solid var(--aura-line); margin-top: 4px; padding-top: 16px; }
  .panel-head + .source-actions { margin-top: 2px; }
  .setting-info,
  .toggle-info { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .setting-label,
  .toggle-label { font-size: 14px; color: var(--text-primary); font-weight: 650; }
  .setting-desc,
  .toggle-sub { font-size: 12px; color: var(--text-muted); line-height: 1.45; overflow-wrap: anywhere; }
  .segmented { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .theme-seg button { display: inline-flex; align-items: center; gap: 5px; }
  button {
    min-width: 0;
    border: 1px solid var(--aura-border);
    border-radius: var(--radius-full);
    padding: 9px 13px;
    color: var(--text-secondary);
    background: var(--bg-elev);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    transition: background 0.18s ease, border-color 0.18s ease, color 0.18s ease, transform 0.18s ease;
  }
  button:hover { border-color: var(--border-hover); color: var(--text-primary); background: var(--bg-hover); }
  button:active { transform: translateY(1px); }
  button.active {
    border-color: var(--accent-ring);
    color: #fff;
    background: var(--accent);
  }
  button.active:hover { background: var(--accent-hi); border-color: var(--accent-hi); }

  .source-actions { display: flex; gap: 8px; margin-bottom: 2px; }
  .toggle-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 4px 16px; }
  .toggle-grid .toggle-row { grid-template-columns: minmax(0, 1fr) auto; }

  .form { display: grid; padding-top: 2px; }
  .form-row {
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 140px) minmax(0, 1fr);
    gap: 14px;
    align-items: center;
    padding: 13px 0;
    border-bottom: 1px solid var(--aura-line);
    color: var(--text-secondary);
    font-size: 0.82rem;
  }
  .form-row:last-child { border-bottom: 0; padding-bottom: 0; }
  .form-row > span { color: var(--text-muted); }
  .checkbox-row { display: flex; align-items: center; gap: 8px; font-size: 0.9rem; color: var(--text-secondary); }
  .checkbox-row input { width: auto; accent-color: var(--accent); }
  input {
    min-width: 0;
    width: 100%;
    background: var(--aura-inset);
    color: var(--text-primary);
    border: 1px solid var(--aura-border);
    border-radius: var(--radius-sm);
    padding: 12px 14px;
    font-family: var(--font-mono);
    font-size: 0.85rem;
  }
  input:focus { outline: none; border-color: var(--accent); }
  .dirs { display: grid; padding: 2px 0 12px; }
  .dirs article {
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 13px 0;
    border-bottom: 1px solid var(--aura-line);
    background: transparent;
  }
  .dirs article:last-child { border-bottom: 0; }
  .dirs article span {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .action-list { display: grid; }

  /* Toggle switch */
  .toggle { position: relative; display: inline-block; width: 48px; height: 26px; flex-shrink: 0; }
  .toggle input { opacity: 0; width: 0; height: 0; }
  .toggle-slider {
    position: absolute; cursor: pointer; inset: 0;
    background: var(--bg-hover); border-radius: 26px; transition: 0.3s;
  }
  .toggle-slider::before {
    content: ""; position: absolute; height: 20px; width: 20px; left: 3px; bottom: 3px;
    background: #fff; border-radius: 50%; transition: 0.3s;
  }
  .toggle input:checked + .toggle-slider { background: var(--accent); }
  .toggle input:checked + .toggle-slider::before { transform: translateX(22px); }
  .toggle input:disabled + .toggle-slider { opacity: 0.5; cursor: not-allowed; }

  .sw { position: relative; display: inline-block; width: 44px; height: 24px; flex-shrink: 0; cursor: pointer; }
  .sw input { opacity: 0; width: 0; height: 0; position: absolute; }
  .sw-track { display: block; width: 44px; height: 24px; background: var(--bg-hover); border: 1px solid var(--border); border-radius: 24px; transition: background 0.2s, border-color 0.2s; position: relative; }
  .sw-thumb { position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: var(--text-muted); transition: transform 0.2s, background 0.2s; }
  .sw input:checked ~ .sw-track { background: var(--accent-lo); border-color: var(--accent-ring); }
  .sw input:checked ~ .sw-track .sw-thumb { transform: translateX(20px); background: var(--accent); }

  .about-info { padding: 12px 0; }
  .about-info p { margin-bottom: 4px; font-size: 14px; color: var(--text-secondary); }
  .about-info strong { color: var(--text-primary); }
  .tech-stack { color: var(--text-muted) !important; font-size: 12px !important; margin-top: 8px !important; }

  /* Entry animations */
  @keyframes fadeInUp {
    from { opacity: 0; transform: translateY(12px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .aura-enter {
    opacity: 0;
    animation: fadeInUp 0.4s var(--aura-ease, ease) forwards;
  }
  .setting-group:nth-child(1) { animation-delay: 0.05s; }
  .setting-group:nth-child(2) { animation-delay: 0.10s; }
  .setting-group:nth-child(3) { animation-delay: 0.15s; }
  .setting-group:nth-child(4) { animation-delay: 0.20s; }
  .setting-group:nth-child(5) { animation-delay: 0.25s; }
  .setting-group:nth-child(6) { animation-delay: 0.30s; }
  .setting-group:nth-child(7) { animation-delay: 0.35s; }

  @media (max-width: 900px) {
    .settings-layout { grid-template-columns: 1fr; }
    .settings-anchors { position: static; grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .toggle-grid { grid-template-columns: minmax(0, 1fr); }
    .panel-head,
    .form-row,
    .setting-row,
    .toggle-row { grid-template-columns: 1fr; }
    .segmented { justify-content: flex-start; }
  }

  @media (max-width: 560px) {
    .page { padding: 18px; }
    .settings-anchors { grid-template-columns: 1fr; }
    .dirs article { grid-template-columns: 1fr; }
  }
</style>
