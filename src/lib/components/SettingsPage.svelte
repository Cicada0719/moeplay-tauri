<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { i18n } from "../stores/i18n.svelte";
  import { THEME_PACKS, normalizeAppearance, type ThemePackId } from "../theme-packs";
  import { clearAppCache, getAppCacheStats, secretDelete, secretSet, secretStatus, setAutostart } from "../api";
  import { wallpaperStore } from "../stores/wallpapers.svelte";
  import Button from "./ui/Button.svelte";
  import Card from "./ui/Card.svelte";
  import SegmentControl from "./ui/SegmentControl.svelte";
  import Switch from "./ui/Switch.svelte";
  import Input from "./ui/Input.svelte";
  import Icon from "./Icon.svelte";
  import UpdateDialog from "./UpdateDialog.svelte";
  import { PageHeader, PageShell, StateBoundary, type ViewState } from "./ui-v2";
  import ScrapeSection from "./settings/ScrapeSection.svelte";
  import LibrarySection from "./settings/LibrarySection.svelte";
  import BangumiSection from "./settings/BangumiSection.svelte";
  import PlayerSection from "./settings/PlayerSection.svelte";
  import "./settings/settings-shared.css";
  import { APP_VERSION } from "../app-version";
  import { orientationStore, platformStore, type OrientationMode } from "../platform";
  import { kineticStageStore } from "../features/kinetic";
  import { applyStartupWindowMode } from "../utils/startup-window-mode";

  let showUpdateDialog = $state(false);
  const appVersion = APP_VERSION;
  let cacheBytes = $state(0);
  let cacheFiles = $state(0);
  let cacheBusy = $state(false);
  let resetBusy = $state(false);
  let resetArmed = $state(false);

  const orientationModes: { id: OrientationMode; label: string; icon: string }[] = [
    { id: "auto", label: "自动", icon: "smartphone" },
    { id: "portrait", label: "竖屏", icon: "smartphone" },
    { id: "landscape", label: "横屏", icon: "monitor" },
  ];

  const startupModes = [
    { id: "windowed", label: "窗口模式", icon: "square" },
    { id: "fullscreen", label: "全屏模式", icon: "maximize" },
    { id: "big-picture", label: "大屏模式", icon: "tv" },
  ];

  const languageOptions = [
    { value: "zh", label: "中文" },
    { value: "en", label: "English" },
  ];

  let savingAutostart = $state(false);
  let aiKeyInput = $state("");
  let aiSecretConfigured = $state(false);
  let aiSecretBusy = $state(false);
  let aiSecretMessage = $state("");

  // 三态统一：设置尚未从后端加载完成时展示骨架/错误，而不是直接渲染默认值。
  const settingsViewState = $derived<ViewState>(
    settingsStore.loaded ? "ready" : settingsStore.loading ? "loading" : "error",
  );

  async function save() {
    await settingsStore.save(settingsStore.settings);
  }

  function scrollToSettings(id: string) {
    document.getElementById(id)?.scrollIntoView({ behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth", block: "start" });
  }

  // 界面语言：i18n store 负责 localStorage 持久化，settings 后端同步 language 字段，
  // 与外观设置一样立即生效并在重启后保持。
  async function setInterfaceLanguage(lang: string) {
    i18n.lang = lang;
    settingsStore.setLanguage(lang);
    await save();
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  async function refreshCacheStats() {
    try {
      const stats = await getAppCacheStats();
      cacheBytes = stats.bytes;
      cacheFiles = stats.files;
    } catch {
      cacheBytes = 0;
      cacheFiles = 0;
    }
  }

  async function handleClearCache() {
    cacheBusy = true;
    try {
      const result = await clearAppCache();
      cacheBytes = 0;
      cacheFiles = 0;
      await wallpaperStore.load().catch(() => {});
      uiStore.notify(`已释放 ${formatBytes(result.bytes_freed)}，删除 ${result.files_removed} 个缓存文件`, "success");
    } catch (error) {
      uiStore.notify(`清理缓存失败：${String(error)}`, "error");
    } finally {
      cacheBusy = false;
    }
  }

  async function handleRestoreDefaults() {
    if (!resetArmed) {
      resetArmed = true;
      window.setTimeout(() => { resetArmed = false; }, 8000);
      return;
    }

    resetBusy = true;
    try {
      localStorage.clear();
      await settingsStore.restoreDefaults();
      resetArmed = false;
      uiStore.notify("已恢复默认设置；游戏库、下载、存档与账号密钥均已保留", "success");
      window.setTimeout(() => window.location.reload(), 700);
    } catch (error) {
      uiStore.notify(`恢复默认设置失败：${String(error)}`, "error");
    } finally {
      resetBusy = false;
    }
  }

  onMount(() => {
    void refreshCacheStats();
    void refreshAiSecretStatus();
  });

  async function setBooleanSetting(key: "ai_enabled", value: boolean) {
    await settingsStore.save({ ...settingsStore.settings, [key]: value });
  }

  async function refreshAiSecretStatus() {
    const origin = settingsStore.settings.ai_api_url.trim();
    if (!origin) {
      aiSecretConfigured = false;
      return;
    }
    try {
      aiSecretConfigured = (await secretStatus("ai_api_key", origin)).configured;
    } catch (e) {
      aiSecretConfigured = false;
      aiSecretMessage = "无法读取密钥状态: " + String(e);
    }
  }

  async function saveAiSettings() {
    await save();
    await refreshAiSecretStatus();
  }

  async function saveAiSecret() {
    const secret = aiKeyInput.trim();
    if (!secret) {
      aiSecretMessage = "请输入 API Key";
      return;
    }
    aiSecretBusy = true;
    aiSecretMessage = "";
    try {
      await save();
      const status = await secretSet("ai_api_key", secret, settingsStore.settings.ai_api_url);
      aiSecretConfigured = status.configured;
      aiKeyInput = "";
      aiSecretMessage = "API Key 已安全保存";
    } catch (e) {
      aiSecretMessage = "保存失败: " + String(e);
    } finally {
      aiSecretBusy = false;
    }
  }

  async function deleteAiSecret() {
    aiSecretBusy = true;
    aiSecretMessage = "";
    try {
      const status = await secretDelete("ai_api_key", settingsStore.settings.ai_api_url);
      aiSecretConfigured = status.configured;
      aiKeyInput = "";
      aiSecretMessage = "API Key 已删除";
    } catch (e) {
      aiSecretMessage = "删除失败: " + String(e);
    } finally {
      aiSecretBusy = false;
    }
  }

  async function toggleAutostart(enabled: boolean) {
    savingAutostart = true;
    try {
      const mode = settingsStore.settings.startup_mode ?? "fullscreen";
      const msg = await setAutostart(enabled, mode);
      await settingsStore.save({ ...settingsStore.settings, autostart_enabled: enabled, startup_mode: mode });
      uiStore.notify(msg, "success");
    } catch (e) {
      uiStore.notify("设置失败: " + String(e), "error");
    } finally {
      savingAutostart = false;
    }
  }

  async function setCloseBehavior(value: string) {
    const minimize_to_tray = value === "tray";
    await settingsStore.save({ ...settingsStore.settings, minimize_to_tray });
    uiStore.notify(minimize_to_tray ? "关闭窗口后将驻留系统托盘" : "关闭窗口后将彻底退出进程", "success");
  }

  async function setStartupMode(mode: string) {
    await settingsStore.save({ ...settingsStore.settings, startup_mode: mode });
    if (settingsStore.settings.autostart_enabled) {
      try { await setAutostart(true, mode); } catch { /* ignore */ }
    }
    // 立即对当前窗口应用，给即时反馈（dev/web 下 getCurrentWindow 会抛错，已 catch）。
    // 与启动路径共用 applyStartupWindowMode：windowed 还原成可调整大小的居中窗口，
    // 而不是 maximize——最大化的窗口无法拖拽调整大小。
    try {
      const win = getCurrentWindow();
      await applyStartupWindowMode(mode, win);
    } catch { /* 非 Tauri 环境忽略 */ }
    const label = startupModes.find((m) => m.id === mode)?.label ?? mode;
    uiStore.notify(
      mode === "big-picture" ? `已设为${label}，重启后进入大屏中心` : `启动模式已切换：${label}`,
      "success",
    );
  }

  async function updateAppearance(patch: Record<string, unknown>) {
    await settingsStore.setAppearance({ ...normalizeAppearance(settingsStore.settings.appearance), ...patch });
  }

  async function chooseThemePack(theme_pack: ThemePackId) {
    await updateAppearance({ theme_pack, fixed_wallpaper_id: undefined });
    uiStore.notify("主题包已切换", "success");
  }
</script>

<PageShell as="div" width="full" scrollable={false} class="settings-v2-shell" labelledBy="settings-page-title" ariaLabel={i18n.t("settings.title")}>
  <div class="stg">
    <div class="v2-grain stg-grain" aria-hidden="true"></div>

    <PageHeader
      id="settings-page-title"
      class="stg-header"
      eyebrow="設置 / SETTINGS"
      title={i18n.t("settings.title")}
      description={i18n.t("settings.subtitle")}
    />

    <div class="stg-workspace">
      <aside class="stg-index" aria-label={i18n.t("settings.title")}>
        <span>SETTINGS / INDEX</span>
        <button type="button" onclick={() => scrollToSettings("settings-appearance")}><b>01</b><em>{i18n.t("settings.nav_appearance")}</em></button>
        <button type="button" onclick={() => scrollToSettings("settings-scrape")}><b>02</b><em>{i18n.t("settings.nav_scrape")}</em></button>
        <button type="button" onclick={() => scrollToSettings("settings-library")}><b>03</b><em>{i18n.t("settings.nav_library")}</em></button>
        <button type="button" onclick={() => scrollToSettings("settings-bangumi")}><b>04</b><em>{i18n.t("settings.nav_bangumi")}</em></button>
        <button type="button" onclick={() => scrollToSettings("settings-player")}><b>05</b><em>{i18n.t("settings.nav_player")}</em></button>
        <button type="button" onclick={() => scrollToSettings("settings-advanced")}><b>06</b><em>{i18n.t("settings.nav_advanced")}</em></button>
        <button type="button" onclick={() => scrollToSettings("settings-maintenance")}><b>07</b><em>{i18n.t("settings.nav_maintenance")}</em></button>
        <small>{i18n.t("settings.index_hint")}</small>
      </aside>

      <main class="stg-content">
        <StateBoundary state={settingsViewState} onRetry={() => settingsStore.load()} retryLabel={i18n.t("button.retry")}>

        <!-- 外观与窗口 -->
        <span class="section-anchor" id="settings-appearance" aria-hidden="true"></span>
        <Card class="s-section" padding="lg" ariaLabel="settings-appearance">
          <div class="s-head">
            <h2 class="s-title"><Icon name="eye" size={17} className="s-title-ic" /> {i18n.t("settings.section_appearance")}<span class="s-title-sub">外観 / APPEARANCE</span></h2>
          </div>

          <div class="theme-pack-grid" aria-label="主题包">
            {#each THEME_PACKS as pack}
              <button
                class="theme-pack-card"
                class:active={normalizeAppearance(settingsStore.settings.appearance).theme_pack === pack.id}
                type="button"
                aria-pressed={normalizeAppearance(settingsStore.settings.appearance).theme_pack === pack.id}
                onclick={() => chooseThemePack(pack.id)}
              >
                <img src={pack.preview} alt="" />
                <span class="theme-pack-card__scrim"></span>
                <span class="theme-pack-card__copy"><b>{pack.label}</b></span>
              </button>
            {/each}
          </div>

          <div class="s-row">
            <div class="s-info"><span class="s-label">{i18n.t("settings.language")}</span><span class="s-desc">{i18n.t("settings.language_desc")}</span></div>
            <SegmentControl options={languageOptions} value={i18n.lang} onChange={setInterfaceLanguage} size="sm" />
          </div>

          <div class="s-row">
            <div class="s-info"><span class="s-label">电影化主视觉</span><span class="s-desc">首页 Kinetic 电影化媒体舞台；关闭或系统减弱动态时回退为静态渐变</span></div>
            <Switch checked={kineticStageStore.enabled} onchange={(e) => kineticStageStore.setEnabled((e.target as HTMLInputElement).checked)} />
          </div>

          {#if platformStore.capabilities.orientationControl}
            <div class="s-divider"></div>
            <div class="s-info" style="padding-bottom: 12px;">
              <span class="s-label">屏幕方向</span>
              <span class="s-desc">自动跟随设备，或在此设备锁定竖屏/横屏</span>
            </div>
            <div class="mode-grid orientation-grid">
              {#each orientationModes as mode}
                <Card class="mode-card {orientationStore.mode === mode.id ? 'active' : ''}" hoverable onclick={() => orientationStore.setMode(mode.id)} padding="md">
                  <div class="mode-icon"><Icon name={mode.icon} size={22} /></div>
                  <span class="mode-label">{mode.label}</span>
                </Card>
              {/each}
            </div>
            <div class="s-row" style="margin-top: 14px;">
              <div class="s-info"><span class="s-label">视频全屏自动横屏</span><span class="s-desc">退出全屏后恢复原来的方向偏好</span></div>
              <Switch checked={orientationStore.videoAutoLandscape} onchange={() => orientationStore.setVideoAutoLandscape(!orientationStore.videoAutoLandscape)} />
            </div>
          {/if}

          {#if platformStore.capabilities.desktopWindowControl}
          <div class="s-divider"></div>
          <div class="s-info" style="padding-bottom: 12px;">
            <span class="s-label">默认启动模式</span>
            <span class="s-desc">选择打开萌游时的窗口行为</span>
          </div>
          <div class="mode-grid">
            {#each startupModes as mode}
              <Card
                class="mode-card {(settingsStore.settings.startup_mode ?? 'fullscreen') === mode.id ? 'active' : ''}"
                hoverable
                onclick={() => setStartupMode(mode.id)}
                padding="md"
              >
                <div class="mode-icon">
                  <Icon name={mode.icon} size={22} />
                </div>
                <span class="mode-label">{mode.label}</span>
              </Card>
            {/each}
          </div>

          <div class="s-divider"></div>
          <div class="s-row close-behavior-row">
            <div class="s-info">
              <span class="s-label">点击右上角关闭</span>
              <span class="s-desc">彻底退出，或隐藏到系统托盘</span>
            </div>
            <SegmentControl
              options={[{ value: "exit", label: "退出进程" }, { value: "tray", label: "驻留托盘" }]}
              value={settingsStore.settings.minimize_to_tray ? "tray" : "exit"}
              onChange={setCloseBehavior}
              size="sm"
            />
          </div>
          {/if}

        </Card>

        <!-- 数据刮削 / 库与导入 / Bangumi / 播放器：拆分为 settings/ 子组件 -->
        <ScrapeSection />
        <LibrarySection />
        <BangumiSection />
        <PlayerSection />

        <!-- 系统与实验功能 -->
        <span class="section-anchor" id="settings-advanced" aria-hidden="true"></span>
        <Card class="s-section" padding="lg" ariaLabel="settings-advanced">
          <div class="s-head">
            <h2 class="s-title"><Icon name="toolbox" size={17} className="s-title-ic" /> {i18n.t("settings.section_advanced")}<span class="s-title-sub">実験 / SYSTEM</span></h2>
          </div>

          <div class="src-item">
            <div class="src-info">
              <span class="src-name">启用 AI 增强</span>
              <span class="src-desc">使用大语言模型辅助刮削和补全</span>
            </div>
            <Switch checked={settingsStore.settings.ai_enabled} onchange={(e) => setBooleanSetting("ai_enabled", (e.target as HTMLInputElement).checked)} />
          </div>
          {#if settingsStore.settings.ai_enabled}
            <div class="ai-form">
              <label class="ai-field">
                <span class="ai-label">API 地址</span>
                <Input
                  bind:value={settingsStore.settings.ai_api_url}
                  onblur={saveAiSettings}
                  placeholder="https://api.openai.com/v1/chat/completions"
                />
              </label>
              <div class="ai-field">
                <span class="ai-label">API Key · {aiSecretConfigured ? "已配置" : "未配置"}</span>
                <Input
                  type="password"
                  bind:value={aiKeyInput}
                  autocomplete="off"
                  placeholder={aiSecretConfigured ? "输入新 Key 以替换" : "sk-..."}
                />
                <div class="ai-secret-actions">
                  <Button size="sm" variant="secondary" press={saveAiSecret} disabled={aiSecretBusy || !aiKeyInput.trim()}>
                    {aiSecretBusy ? "处理中" : "安全保存"}
                  </Button>
                  {#if aiSecretConfigured}
                    <Button size="sm" variant="ghost" press={deleteAiSecret} disabled={aiSecretBusy}>删除 Key</Button>
                  {/if}
                  {#if aiSecretMessage}<span class="ai-secret-message">{aiSecretMessage}</span>{/if}
                </div>
              </div>
              <label class="ai-field">
                <span class="ai-label">模型</span>
                <Input
                  bind:value={settingsStore.settings.ai_model}
                  onblur={save}
                  placeholder="gpt-4o-mini"
                />
              </label>
            </div>
          {/if}

          {#if platformStore.capabilities.autostart}
            <div class="s-divider"></div>
            <div class="src-item">
            <div class="src-info">
              <span class="src-name">开机自启动</span>
              <span class="s-desc">系统启动时自动打开萌游</span>
            </div>
            <Switch
              checked={settingsStore.settings.autostart_enabled ?? false}
              onchange={(e) => toggleAutostart((e.target as HTMLInputElement).checked)}
              disabled={savingAutostart}
            />
            </div>
          {/if}

        </Card>

        <!-- 维护与更新 -->
        <span class="section-anchor" id="settings-maintenance" aria-hidden="true"></span>
        <Card class="s-section" padding="lg" ariaLabel="settings-maintenance">
          <div class="s-head">
            <h2 class="s-title"><Icon name="refresh" size={17} className="s-title-ic" /> {i18n.t("settings.section_maintenance")}<span class="s-title-sub">保守 / MAINTENANCE</span></h2>
          </div>
          <p class="s-note">清理可重建文件或恢复界面偏好，不会删除游戏库、下载内容、存档和安全存储的账号密钥。</p>

          <div class="maintenance-grid">
            <article class="maintenance-card">
              <div class="maintenance-copy">
                <span class="maintenance-kicker">CACHE / 可安全重建</span>
                <strong>{formatBytes(cacheBytes)}</strong>
                <small>{cacheFiles} 个缩略图、番剧封面代理及在线壁纸缓存文件</small>
              </div>
              <Button variant="secondary" size="sm" press={handleClearCache} loading={cacheBusy}>
                <Icon name="trash" size={14} /> 清除缓存
              </Button>
            </article>

            <article class="maintenance-card maintenance-card--danger">
              <div class="maintenance-copy">
                <span class="maintenance-kicker">RESET / 仅恢复偏好</span>
                <strong>恢复初始化设置</strong>
                <small>重置主题、窗口、来源、播放器与实验设置；再次点击确认，8 秒后自动取消。</small>
              </div>
              <Button class={resetArmed ? "danger-action is-armed" : "danger-action"} variant="ghost" size="sm" press={handleRestoreDefaults} loading={resetBusy}>
                <Icon name={resetArmed ? "info" : "refresh"} size={14} /> {resetArmed ? "确认恢复默认" : "恢复默认设置"}
              </Button>
            </article>
          </div>

          <div class="s-divider"></div>
          <div class="about-block">
            <div class="about-name">
              <span class="about-brand">萌游</span>
              <span class="about-ver">v{appVersion}</span>
              {#if platformStore.capabilities.desktopUpdater}
                <span class="update-badge"><i></i> 已启用签名自动更新</span>
              {:else}
                <span class="update-badge"><i></i> Android 手机版</span>
              {/if}
            </div>
            {#if platformStore.capabilities.desktopUpdater}
              <div class="about-update-copy">官方正式版启动后自动检查更新；下载包与签名校验通过后才会安装。</div>
              <Button variant="ghost" size="sm" press={() => showUpdateDialog = true}>
                <Icon name="download" size={14} /> 立即检查更新
              </Button>
            {:else}
              <div class="about-update-copy">手机版更新通过新的 Android 安装包完成，不调用 Windows 桌面更新器。</div>
            {/if}
          </div>
        </Card>

        </StateBoundary>
      </main>
    </div>
  </div>
</PageShell>

{#if platformStore.capabilities.desktopUpdater}
  <UpdateDialog bind:open={showUpdateDialog} />
{/if}

<style>
  :global(.settings-v2-shell) { height: 100%; }
  :global(.settings-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .stg {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .stg-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.stg-header) {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    flex-shrink: 0;
  }

  /* ── Wide settings workspace ── */
  .stg-workspace { position: relative; z-index: 1; flex: 1; min-height: 0; width: 100%; max-width: 1280px; margin: 0 auto; padding: 0 28px 36px; display: grid; grid-template-columns: 220px minmax(0, 1fr); gap: 28px; overflow: hidden; }
  .stg-index { align-self: start; position: sticky; top: 0; display: grid; padding-top: 10px; border-top: 1px solid var(--border-hover); }
  .stg-index > span { padding: 0 0 14px; color: var(--accent); font: 700 8px/1 var(--font-mono); letter-spacing: .16em; }
  .stg-index button { width: 100%; display: grid; padding: 0; border-right: 0; border-bottom: 0; border-left: 0; border-radius: 0; background: transparent; text-align: left; cursor: pointer; grid-template-columns: 32px 1fr; align-items: center; min-height: 42px; border-top: 1px solid var(--border); color: var(--text-secondary); text-decoration: none; transition: padding-left .28s var(--ui-ease-out), color .2s ease, background .2s ease; }
  .stg-index button:last-of-type { border-bottom: 1px solid var(--border); }
  .stg-index button:hover, .stg-index button:focus-visible { padding-left: 7px; color: var(--text-primary); background: color-mix(in srgb, var(--accent) 6%, transparent); outline: 0; }
  .stg-index b { color: var(--text-dim); font: 700 8px/1 var(--font-mono); }
  .stg-index em { font: 650 12px/1 var(--font-ui); font-style: normal; }
  .stg-index small { margin-top: 16px; color: var(--text-dim); font-size: 10px; line-height: 1.6; }
  .stg-content { min-height: 0; overflow-y: auto; width: 100%; padding: 8px 2px 40px; display: flex; flex-direction: column; gap: 14px; scroll-behavior: smooth; }

  /* ── Startup / orientation mode cards ── */
  .mode-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  :global(.mode-card) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }
  :global(.mode-card):active { transform: translateY(0) scale(0.98); }
  :global(.mode-card.active) {
    border-color: var(--accent-ring);
    background: var(--accent-lo);
    color: var(--text-primary);
    box-shadow: 0 0 0 1px var(--accent-ring), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }
  .mode-icon {
    width: 40px; height: 40px;
    display: grid; place-items: center;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-muted);
    transition: color 0.2s, background 0.2s;
  }
  :global(.mode-card.active) .mode-icon {
    background: var(--accent);
    color: #fff;
  }
  .mode-label { font-size: 13px; font-weight: 700; }

  /* ── AI form ── */
  .ai-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 14px;
  }
  .ai-field {
    display: grid;
    grid-template-columns: 80px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
  }
  .ai-label {
    font-size: 12.5px;
    color: var(--text-muted);
    font-weight: 600;
  }
  .ai-secret-actions {
    grid-column: 2;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .ai-secret-message {
    color: var(--text-muted);
    font-size: 12px;
  }

  /* ── Maintenance ── */
  .maintenance-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; margin-top: 12px; }
  .maintenance-card { min-height: 150px; padding: 16px; border: 1px solid var(--border); border-radius: var(--radius-lg); background: linear-gradient(145deg, var(--bg-elev), color-mix(in srgb, var(--bg-elev) 72%, transparent)); display: flex; flex-direction: column; align-items: flex-start; justify-content: space-between; gap: 18px; }
  .maintenance-card--danger { border-color: color-mix(in srgb, var(--color-error) 28%, var(--border)); }
  .maintenance-copy { display: flex; flex-direction: column; align-items: flex-start; gap: 5px; }
  .maintenance-copy strong { font-size: 18px; color: var(--text-primary); letter-spacing: -.02em; }
  .maintenance-copy small { color: var(--text-muted); font-size: 11px; line-height: 1.55; max-width: 38ch; }
  .maintenance-kicker { color: var(--accent); font: 600 9px/1.2 var(--font-mono); letter-spacing: .13em; }
  :global(.danger-action.is-armed) { border-color: var(--color-error) !important; color: var(--color-error) !important; background: color-mix(in srgb, var(--color-error) 9%, transparent) !important; }
  .update-badge { display: inline-flex; align-items: center; gap: 6px; padding: 4px 8px; border: 1px solid color-mix(in srgb, var(--color-success) 34%, var(--border)); border-radius: 999px; color: var(--color-success); font-size: 10px; font-weight: 650; }
  .update-badge i { width: 6px; height: 6px; border-radius: 50%; background: var(--color-success); box-shadow: 0 0 12px color-mix(in srgb, var(--color-success) 75%, transparent); }
  .about-update-copy { color: var(--text-muted); font-size: 11px; line-height: 1.55; max-width: 58ch; }

  /* ── About ── */
  .about-block { display: flex; flex-direction: column; gap: 8px; padding: 4px 0; }
  .about-name { display: flex; align-items: baseline; gap: 10px; }
  .about-brand {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 750;
    color: var(--text-primary);
  }
  :global(.about-ver) {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
  }

  /* ── Theme pack cards ── */
  .theme-pack-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; margin: 4px 0 14px; }
  .theme-pack-card { position: relative; min-height: 148px; overflow: hidden; padding: 0; border: 1px solid var(--border); border-radius: 16px; background: var(--bg-card); color: white; cursor: pointer; text-align: left; transition: transform .18s var(--ease-enter), border-color .18s, box-shadow .18s; }
  .theme-pack-card:hover { transform: translateY(-3px); border-color: var(--border-hover); }
  .theme-pack-card.active { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent-lo), 0 16px 34px rgba(0, 0, 0, .28); }
  .theme-pack-card:focus-visible { outline: none; box-shadow: var(--focus-ring); }
  .theme-pack-card img, .theme-pack-card__scrim { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .theme-pack-card__scrim { background: linear-gradient(0deg, rgba(3, 5, 12, .9), rgba(3, 5, 12, .05) 75%); }
  .theme-pack-card__copy { position: absolute; left: 14px; right: 14px; bottom: 12px; display: flex; flex-direction: column; gap: 3px; }
  .theme-pack-card__copy b { font-size: 14px; letter-spacing: .02em; }

  /* ── Responsive ── */
  @media (max-width: 720px) {
    .maintenance-grid { grid-template-columns: 1fr; }
    .mode-grid { grid-template-columns: 1fr; }
    .ai-field { grid-template-columns: 1fr; }
    .ai-secret-actions { grid-column: 1; }
    .stg-content { padding: 8px 16px 36px; }
    :global(.stg-header) { padding: 20px 16px 12px; }
  }
  @media (max-width: 760px) { .theme-pack-grid { grid-template-columns: 1fr; } .theme-pack-card { min-height: 120px; } }
  @media (max-width: 980px) { .stg-workspace { grid-template-columns: 1fr; padding-inline: 18px; } .stg-index { position: static; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0; padding: 0; } .stg-index > span, .stg-index small { grid-column: 1 / -1; } .stg-index button { padding-inline: 8px; } :global(.stg-header) { padding-inline: 18px; } }
  @media (max-width: 620px) { .stg-index { grid-template-columns: repeat(2, minmax(0, 1fr)); } .stg-workspace { padding-inline: 12px; } :global(.stg-header) { padding-inline: 12px; } }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .stg-index button, .theme-pack-card, .mode-icon { transition: none; }
    :global(.mode-card) { transition: none; }
    .stg-content { scroll-behavior: auto; }
    .theme-pack-card:hover { transform: none; }
  }
  :global([data-motion="reduce"]) .stg-index button,
  :global([data-motion="reduce"]) .theme-pack-card,
  :global([data-motion="reduce"]) .mode-icon { transition: none; }
  :global([data-motion="reduce"] .mode-card) { transition: none; }
  :global([data-motion="reduce"]) .stg-content { scroll-behavior: auto; }
  :global([data-motion="reduce"]) .theme-pack-card:hover { transform: none; }
</style>
