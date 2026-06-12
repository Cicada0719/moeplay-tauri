<script lang="ts">
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";

  const themes = [
    { id: "dark", label: "深色", icon: "home" },
    { id: "light", label: "浅色", icon: "lightbulb" },
    { id: "sakura", label: "樱夜", icon: "heart" },
  ];

  async function handleThemeChange(theme: string) {
    await settingsStore.save({ ...settingsStore.settings, theme });
    uiStore.notify(`主题已切换为${themes.find((t) => t.id === theme)?.label}`, "success");
  }

  async function handleToggleVndb() {
    await settingsStore.save({ ...settingsStore.settings, vndb_enabled: !settingsStore.settings.vndb_enabled });
  }
  async function handleToggleBangumi() {
    await settingsStore.save({ ...settingsStore.settings, bangumi_enabled: !settingsStore.settings.bangumi_enabled });
  }
  async function handleToggleAutoScrape() {
    await settingsStore.save({ ...settingsStore.settings, auto_scrape: !settingsStore.settings.auto_scrape });
  }
  async function handleToggleAi() {
    await settingsStore.save({ ...settingsStore.settings, ai_enabled: !settingsStore.settings.ai_enabled });
  }
  async function handleSaveAiSettings() {
    await settingsStore.save(settingsStore.settings);
    uiStore.notify("AI 设置已保存", "success");
  }
  async function handleAddWatchDir() {
    await settingsStore.addWatchDir();
    uiStore.notify("监控目录已添加", "success");
  }
  async function handleRemoveWatchDir(dir: string) {
    await settingsStore.removeWatchDir(dir);
    uiStore.notify("监控目录已移除", "info");
  }
</script>

<div class="settings-view">
  <h2>设置</h2>

  <!-- 主题 -->
  <section class="settings-section">
    <h3><Icon name="home" size={16} /> 主题</h3>
    <div class="theme-grid">
      {#each themes as theme}
        <button
          class="theme-btn"
          class:active={settingsStore.settings.theme === theme.id}
          onclick={() => handleThemeChange(theme.id)}
        >
          <Icon name={theme.icon} size={24} />
          <span>{theme.label}</span>
        </button>
      {/each}
    </div>
  </section>

  <!-- 刮削设置 -->
  <section class="settings-section">
    <h3><Icon name="search" size={16} /> 刮削数据源</h3>
    <div class="setting-item">
      <div class="setting-info">
        <span class="setting-label">VNDB (视觉小说数据库)</span>
        <span class="setting-desc">获取视觉小说的元数据信息</span>
      </div>
      <label class="toggle">
        <input type="checkbox" checked={settingsStore.settings.vndb_enabled} onchange={handleToggleVndb} />
        <span class="toggle-slider"></span>
      </label>
    </div>
    <div class="setting-item">
      <div class="setting-info">
        <span class="setting-label">Bangumi (番组计划)</span>
        <span class="setting-desc">获取游戏/动漫的元数据信息</span>
      </div>
      <label class="toggle">
        <input type="checkbox" checked={settingsStore.settings.bangumi_enabled} onchange={handleToggleBangumi} />
        <span class="toggle-slider"></span>
      </label>
    </div>
    <div class="setting-item">
      <div class="setting-info">
        <span class="setting-label">自动刮削</span>
        <span class="setting-desc">添加游戏时自动搜索元数据</span>
      </div>
      <label class="toggle">
        <input type="checkbox" checked={settingsStore.settings.auto_scrape} onchange={handleToggleAutoScrape} />
        <span class="toggle-slider"></span>
      </label>
    </div>
  </section>

  <!-- AI -->
  <section class="settings-section">
    <h3><Icon name="star" size={16} /> AI 增强刮削</h3>
    <div class="setting-item">
      <div class="setting-info">
        <span class="setting-label">启用 AI 增强</span>
        <span class="setting-desc">使用 LLM 智能生成游戏描述和标签</span>
      </div>
      <label class="toggle">
        <input type="checkbox" checked={settingsStore.settings.ai_enabled} onchange={handleToggleAi} />
        <span class="toggle-slider"></span>
      </label>
    </div>
    {#if settingsStore.settings.ai_enabled}
      <div class="ai-config">
        <div class="config-field">
          <label class="config-label" for="ai-api-url">API 地址</label>
          <input id="ai-api-url" type="text" class="config-input" placeholder="https://api.openai.com/v1/chat/completions" bind:value={settingsStore.settings.ai_api_url} onblur={handleSaveAiSettings} />
        </div>
        <div class="config-field">
          <label class="config-label" for="ai-api-key">API Key</label>
          <input id="ai-api-key" type="password" class="config-input" placeholder="sk-..." bind:value={settingsStore.settings.ai_api_key} onblur={handleSaveAiSettings} />
        </div>
        <div class="config-field">
          <label class="config-label" for="ai-model">模型</label>
          <input id="ai-model" type="text" class="config-input" placeholder="gpt-4o-mini" bind:value={settingsStore.settings.ai_model} onblur={handleSaveAiSettings} />
        </div>
        <p class="config-hint">支持 OpenAI 兼容 API（OpenAI / DeepSeek / 本地模型等）</p>
      </div>
    {/if}
  </section>

  <!-- 监控目录 -->
  <section class="settings-section">
    <h3><Icon name="folder" size={16} /> 自动导入目录</h3>
    <p class="section-desc">添加监控目录后，新放入的游戏将自动导入到游戏库</p>
    <div class="watch-dirs">
      {#each settingsStore.settings.watch_dirs as dir}
        <div class="watch-dir-item">
          <span class="dir-path">{dir}</span>
          <button class="remove-btn" onclick={() => handleRemoveWatchDir(dir)}><Icon name="x" size={12} /></button>
        </div>
      {:else}
        <p class="no-dirs">暂无监控目录</p>
      {/each}
    </div>
    <button class="add-dir-btn" onclick={handleAddWatchDir}>
      <Icon name="folder" size={14} /> 添加监控目录
    </button>
  </section>

  <!-- 关于 -->
  <section class="settings-section">
    <h3><Icon name="toolbox" size={16} /> 关于</h3>
    <div class="about-info">
      <p><strong>萌游 MoeGame</strong> v0.1.0</p>
      <p>可爱的游戏管理器</p>
      <p class="tech-stack">Tauri v2 + Svelte 5 + Rust</p>
    </div>
  </section>
</div>

<style>
  .settings-view { padding: 24px; overflow-y: auto; max-width: 700px; }
  .settings-view h2 { font-size: 24px; font-weight: 700; margin-bottom: 24px; color: var(--text-primary); }
  .settings-section { margin-bottom: 32px; }
  .settings-section h3 { font-size: 16px; margin-bottom: 16px; color: var(--text-primary); display: flex; align-items: center; gap: 8px; }
  .section-desc { font-size: 13px; color: var(--text-muted); margin-bottom: 16px; }

  .theme-grid { display: flex; gap: 12px; }
  .theme-btn {
    flex: 1; padding: 20px; border: 1px solid var(--border); background: var(--bg-secondary);
    color: var(--text-secondary); border-radius: var(--radius-md); cursor: pointer;
    display: flex; flex-direction: column; align-items: center; gap: 8px;
    transition: all 0.2s; font-size: 14px;
  }
  .theme-btn:hover { border-color: var(--border-hover); background: var(--bg-hover); }
  .theme-btn.active { border-color: var(--accent); background: var(--accent-lo); color: var(--accent); }

  .setting-item { display: flex; justify-content: space-between; align-items: center; padding: 16px; background: var(--bg-secondary); border-radius: var(--radius-md); margin-bottom: 8px; }
  .setting-info { display: flex; flex-direction: column; gap: 4px; }
  .setting-label { font-size: 14px; color: var(--text-primary); }
  .setting-desc { font-size: 12px; color: var(--text-muted); }

  .toggle { position: relative; display: inline-block; width: 48px; height: 26px; flex-shrink: 0; }
  .toggle input { opacity: 0; width: 0; height: 0; }
  .toggle-slider {
    position: absolute; cursor: pointer; inset: 0;
    background: var(--bg-hover); border-radius: 26px; transition: 0.3s;
  }
  .toggle-slider::before {
    content: ""; position: absolute; height: 20px; width: 20px; left: 3px; bottom: 3px;
    background: white; border-radius: 50%; transition: 0.3s;
  }
  .toggle input:checked + .toggle-slider { background: var(--accent); }
  .toggle input:checked + .toggle-slider::before { transform: translateX(22px); }

  .watch-dirs { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
  .watch-dir-item { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; background: var(--bg-secondary); border-radius: var(--radius-sm); }
  .dir-path { font-size: 13px; color: var(--text-muted); word-break: break-all; }
  .remove-btn { background: transparent; border: 1px solid var(--border); color: var(--text-muted); width: 28px; height: 28px; border-radius: 50%; cursor: pointer; display: grid; place-items: center; flex-shrink: 0; margin-left: 12px; }
  .remove-btn:hover { border-color: var(--color-error); color: var(--color-error); }
  .no-dirs { font-size: 13px; color: var(--text-muted); text-align: center; padding: 16px; }
  .add-dir-btn {
    padding: 12px 20px; border: 1px dashed var(--border); background: transparent;
    color: var(--text-secondary); border-radius: var(--radius-md); cursor: pointer;
    font-size: 14px; width: 100%; display: flex; align-items: center; justify-content: center; gap: 8px;
    transition: all 0.2s;
  }
  .add-dir-btn:hover { border-color: var(--accent); color: var(--accent); background: var(--accent-lo); }

  .ai-config { margin-top: 12px; padding: 16px; background: var(--bg-secondary); border-radius: var(--radius-md); display: flex; flex-direction: column; gap: 12px; }
  .config-field { display: flex; flex-direction: column; gap: 6px; }
  .config-label { font-size: 12px; color: var(--text-muted); font-weight: 500; }
  .config-input {
    background: rgba(255,255,255,0.06); border: 1px solid var(--border); color: var(--text-primary);
    padding: 10px 14px; border-radius: var(--radius-sm); font-size: 13px; outline: none;
    font-family: var(--font-mono); transition: border-color 0.2s;
  }
  .config-input:focus { border-color: var(--accent); }
  .config-input::placeholder { color: var(--text-dim); }
  .config-hint { font-size: 11px; color: var(--text-muted); margin: 0; opacity: 0.7; }

  .about-info { padding: 20px; background: var(--bg-secondary); border-radius: var(--radius-md); }
  .about-info p { margin-bottom: 4px; font-size: 14px; color: var(--text-secondary); }
  .about-info strong { color: var(--text-primary); }
  .tech-stack { color: var(--text-muted) !important; font-size: 12px !important; margin-top: 8px !important; }
</style>
