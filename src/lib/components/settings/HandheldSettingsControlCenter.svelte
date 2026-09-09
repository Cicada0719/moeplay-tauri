<script lang="ts">
  import { onMount } from "svelte";
  import { settingsStore } from "../../stores/settings.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { i18n } from "../../stores/i18n.svelte";
  import { THEME_PACKS, normalizeAppearance, type ThemePackId } from "../../theme-packs";
  import { clearAppCache, getAppCacheStats } from "../../api";
  import { setHandheldSystemBars } from "../../features/handheld/api";
  import { readHandheldImmersivePreference, readHandheldHintsPreference, readHandheldKeyboardPreference, writeHandheldHintsPreference, writeHandheldImmersivePreference, writeHandheldKeyboardPreference } from "../../platform/handheld";
  import { orientationStore, platformStore, type OrientationMode } from "../../platform";
  import { gamepadTuning, type AxisSensitivity, type RepeatSpeed } from "../../platform/gamepadTuning.svelte";
  import { motionStore } from "../../stores/motion.svelte";
  import { kineticStageStore } from "../../features/kinetic";
  import Button from "../ui/Button.svelte";
  import Card from "../ui/Card.svelte";
  import Input from "../ui/Input.svelte";
  import SegmentControl from "../ui/SegmentControl.svelte";
  import Switch from "../ui/Switch.svelte";
  import Icon from "../Icon.svelte";
  import ScrapeSection from "./ScrapeSection.svelte";
  import LibrarySection from "./LibrarySection.svelte";
  import BangumiSection from "./BangumiSection.svelte";
  import PlayerSection from "./PlayerSection.svelte";
  import RulesSection from "./RulesSection.svelte";
  import SyncSettings from "./SyncSettings.svelte";

  type Category = "appearance" | "media" | "sources" | "controls" | "system" | "maintenance";
  const CATEGORY_KEY = "moeplay-handheld-settings-category-v1";
  const CATEGORIES: { id: Category; label: string; hint: string; icon: string }[] = [
    { id: "appearance", label: "外观", hint: "主题与屏幕", icon: "eye" },
    { id: "media", label: "媒体", hint: "播放与阅读", icon: "film" },
    { id: "sources", label: "来源", hint: "规则与同步", icon: "layers" },
    { id: "controls", label: "手柄", hint: "按键与提示", icon: "gamepad" },
    { id: "system", label: "系统", hint: "资料与实验", icon: "toolbox" },
    { id: "maintenance", label: "维护", hint: "缓存与版本", icon: "refresh" },
  ];

  let activeCategory = $state<Category>(readCategory());
  let contentEl = $state<HTMLElement>();
  let cacheBytes = $state(0);
  let cacheFiles = $state(0);
  let cacheBusy = $state(false);
  let resetArmed = $state(false);
  let resetBusy = $state(false);
  let handheldImmersive = $state(readHandheldImmersivePreference());
  let handheldHints = $state(readHandheldHintsPreference());
  let handheldKeyboard = $state(readHandheldKeyboardPreference());
  let reducedMotion = $state(motionStore.reduced);
  let languageOptions = [{ value: "zh", label: "中文" }, { value: "en", label: "English" }];
  const orientationModes: { id: OrientationMode; label: string; icon: string }[] = [
    { id: "auto", label: "自动", icon: "smartphone" },
    { id: "portrait", label: "竖屏", icon: "smartphone" },
    { id: "landscape", label: "横屏", icon: "monitor" },
  ];
  const sensitivityOptions = [
    { value: "loose", label: "宽松" },
    { value: "standard", label: "标准" },
    { value: "tight", label: "紧凑" },
  ];
  const repeatOptions = [
    { value: "slow", label: "慢" },
    { value: "standard", label: "标准" },
    { value: "fast", label: "快" },
  ];

  function readCategory(): Category {
    if (typeof localStorage === "undefined") return "appearance";
    const value = localStorage.getItem(CATEGORY_KEY);
    return CATEGORIES.some((category) => category.id === value) ? value as Category : "appearance";
  }

  function selectCategory(category: Category) {
    activeCategory = category;
    localStorage.setItem(CATEGORY_KEY, category);
    requestAnimationFrame(() => contentEl?.focus({ preventScroll: true }));
  }

  async function chooseTheme(theme_pack: ThemePackId) {
    await settingsStore.setAppearance({ ...normalizeAppearance(settingsStore.settings.appearance), theme_pack, fixed_wallpaper_id: undefined });
  }

  async function setLanguage(value: string) {
    await i18n.setLanguage(value);
    settingsStore.setLanguage(value);
    await settingsStore.save(settingsStore.settings);
  }

  async function setImmersive(value: boolean) {
    handheldImmersive = value;
    writeHandheldImmersivePreference(value);
    if (platformStore.isAndroid) await setHandheldSystemBars(value).catch(() => {});
  }

  function setHints(value: boolean) {
    handheldHints = value;
    writeHandheldHintsPreference(value);
  }

  function setKeyboard(value: boolean) {
    handheldKeyboard = value;
    writeHandheldKeyboardPreference(value);
  }

  function setMotion(value: boolean) {
    reducedMotion = value;
    motionStore.setPreference(value ? "reduce" : "system");
  }

  async function refreshCache() {
    try {
      const stats = await getAppCacheStats();
      cacheBytes = stats.bytes;
      cacheFiles = stats.files;
    } catch {
      cacheBytes = 0;
      cacheFiles = 0;
    }
  }

  function formatBytes(value: number) {
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
    return `${(value / 1024 / 1024).toFixed(1)} MB`;
  }

  async function clearCache() {
    cacheBusy = true;
    try {
      const result = await clearAppCache();
      cacheBytes = 0;
      cacheFiles = 0;
      uiStore.notify(`已释放 ${formatBytes(result.bytes_freed)}，删除 ${result.files_removed} 个缓存文件`, "success");
    } catch {
      /* 状态卡保留，避免把原生错误文本塞进掌机页面。 */
    } finally {
      cacheBusy = false;
    }
  }

  async function restoreDefaults() {
    if (!resetArmed) {
      resetArmed = true;
      window.setTimeout(() => (resetArmed = false), 8000);
      return;
    }
    resetBusy = true;
    try {
      localStorage.clear();
      await settingsStore.restoreDefaults();
      window.location.reload();
    } finally {
      resetBusy = false;
    }
  }

  onMount(() => {
    void refreshCache();
  });
</script>

<div class="hh-settings" data-testid="android-settings-center" data-active-category={activeCategory}>
  <aside class="hh-settings__nav" aria-label="掌机设置分类">
    <div class="hh-settings__nav-head"><span>CONTROL CENTER</span><strong>设备偏好</strong></div>
    <div class="hh-settings__categories">
      {#each CATEGORIES as category, index (category.id)}
        <button type="button" class:active={activeCategory === category.id} aria-current={activeCategory === category.id ? "page" : undefined} onclick={() => selectCategory(category.id)}>
          <b>{String(index + 1).padStart(2, "0")}</b><span class="hh-settings__cat-icon"><Icon name={category.icon} size={16} /></span><span><strong>{category.label}</strong><small>{category.hint}</small></span>
        </button>
      {/each}
    </div>
    <small class="hh-settings__nav-foot">A 选择 · B 返回<br />只挂载当前分类</small>
  </aside>

  <main class="hh-settings__content" bind:this={contentEl} tabindex="-1">
    <header class="hh-settings__content-head">
      <div><span>MOEPLAY / SETTINGS</span><h1>{CATEGORIES.find((category) => category.id === activeCategory)?.label}</h1></div>
      <small>横屏掌机配置 · 即时生效</small>
    </header>

    {#if activeCategory === "appearance"}
      <Card class="hh-settings-card" padding="lg" ariaLabel="外观与屏幕">
        <div class="hh-settings-card__head"><div><span>01 / MATERIAL</span><h2>外观与屏幕</h2></div><Icon name="eye" size={22} /></div>
        <div class="hh-theme-grid" aria-label="主题包">
          {#each THEME_PACKS as pack (pack.id)}
            <button type="button" class:active={normalizeAppearance(settingsStore.settings.appearance).theme_pack === pack.id} aria-pressed={normalizeAppearance(settingsStore.settings.appearance).theme_pack === pack.id} onclick={() => void chooseTheme(pack.id)}>
              <img src={pack.preview} alt="" /><span>{pack.label}</span>
            </button>
          {/each}
        </div>
        <div class="hh-setting-row"><div><strong>界面语言</strong><small>标题、提示和状态卡的显示语言</small></div><SegmentControl options={languageOptions} value={i18n.lang} onChange={setLanguage} size="sm" /></div>
        <div class="hh-setting-row"><div><strong>沉浸式系统栏</strong><small>默认隐藏 Android 状态栏与底部导航栏；边缘滑动仍可临时唤出</small></div><Switch checked={handheldImmersive} onchange={(event) => void setImmersive((event.target as HTMLInputElement).checked)} /></div>
        <div class="hh-setting-row"><div><strong>减少动态效果</strong><small>停用持续动效，保留焦点和状态变化</small></div><Switch checked={reducedMotion} onchange={(event) => setMotion((event.target as HTMLInputElement).checked)} /></div>
        <div class="hh-setting-row"><div><strong>电影化主视觉</strong><small>关闭后回退为更轻量的静态封面舞台</small></div><Switch checked={kineticStageStore.enabled} onchange={(event) => kineticStageStore.setEnabled((event.target as HTMLInputElement).checked)} /></div>
        <div class="hh-settings-subhead">方向策略</div>
        <div class="hh-choice-grid">
          {#each orientationModes as mode (mode.id)}
            <button type="button" class:active={orientationStore.mode === mode.id} onclick={() => orientationStore.setMode(mode.id)}><Icon name={mode.icon} size={17} /><strong>{mode.label}</strong></button>
          {/each}
        </div>
      </Card>
    {:else if activeCategory === "media"}
      <PlayerSection />
      <Card class="hh-settings-card hh-settings-note" padding="lg" ariaLabel="媒体说明"><div class="hh-settings-card__head"><div><span>MEDIA / READ</span><h2>掌机观看方式</h2></div><Icon name="book" size={22} /></div><p>番剧播放器、漫画阅读器和小说分页阅读器会在进入媒体页面时自动横屏；离开最后一个媒体页面后恢复你的方向偏好。</p></Card>
    {:else if activeCategory === "sources"}
      <ScrapeSection />
      <BangumiSection />
      <RulesSection />
      <SyncSettings />
    {:else if activeCategory === "controls"}
      <Card class="hh-settings-card" padding="lg" ariaLabel="手柄控制">
        <div class="hh-settings-card__head"><div><span>04 / INPUT</span><h2>手柄控制</h2></div><Icon name="gamepad" size={22} /></div>
        <div class="hh-setting-row"><div><strong>提示条常显</strong><small>连接手柄时显示当前页面可用操作</small></div><Switch checked={handheldHints} onchange={(event) => setHints((event.target as HTMLInputElement).checked)} /></div>
        {#if platformStore.isAndroid}<div class="hh-setting-row"><div><strong>自动屏幕键盘</strong><small>输入框聚焦时唤起掌机键盘</small></div><Switch checked={handheldKeyboard} onchange={(event) => setKeyboard((event.target as HTMLInputElement).checked)} /></div>{/if}
        <div class="hh-settings-subhead">方向灵敏度</div>
        <SegmentControl options={sensitivityOptions} value={gamepadTuning.sensitivity} onChange={(value) => { if (value === "loose" || value === "standard" || value === "tight") gamepadTuning.sensitivity = value as AxisSensitivity; }} size="sm" />
        <div class="hh-settings-subhead">连发速度</div>
        <SegmentControl options={repeatOptions} value={gamepadTuning.repeatSpeed} onChange={(value) => { if (value === "slow" || value === "standard" || value === "fast") gamepadTuning.repeatSpeed = value as RepeatSpeed; }} size="sm" />
        <div class="hh-mapping"><strong>操作语义</strong><span>A 确认 / 播放　 B 返回　 LB·RB 翻页或切换　 X 选集 / 章节　 Y 显示设置　 START 更多</span></div>
      </Card>
    {:else if activeCategory === "system"}
      <LibrarySection />
      <Card class="hh-settings-card" padding="lg" ariaLabel="系统偏好"><div class="hh-settings-card__head"><div><span>05 / SYSTEM</span><h2>系统偏好</h2></div><Icon name="toolbox" size={22} /></div><p class="hh-settings-note-copy">游戏资料、下载、导入和诊断从“更多”进入。当前分类只挂载与系统偏好直接相关的库设置，避免一次出现过多焦点目标。</p><Input bind:value={settingsStore.settings.ai_model} onblur={() => settingsStore.save(settingsStore.settings)} placeholder="AI 模型（可选）" ariaLabel="AI 模型" /></Card>
    {:else}
      <Card class="hh-settings-card" padding="lg" ariaLabel="缓存与维护">
        <div class="hh-settings-card__head"><div><span>06 / CARE</span><h2>缓存与维护</h2></div><Icon name="refresh" size={22} /></div>
        <div class="hh-maintenance-stat"><strong>{formatBytes(cacheBytes)}</strong><span>{cacheFiles} 个可重建缓存文件</span></div>
        <div class="hh-setting-actions"><Button variant="secondary" size="sm" press={clearCache} loading={cacheBusy}><Icon name="trash" size={14} /> 清理缓存</Button><Button variant="ghost" size="sm" press={refreshCache}><Icon name="refresh" size={14} /> 刷新统计</Button></div>
        <div class="hh-settings-subhead">恢复偏好</div><p>只恢复主题、显示、来源和实验偏好，不删除游戏库、下载、存档或安全存储凭据。</p>
        <Button variant="ghost" size="sm" press={restoreDefaults} loading={resetBusy}><Icon name={resetArmed ? "info" : "refresh"} size={14} /> {resetArmed ? "再次确认恢复默认" : "恢复默认设置"}</Button>
      </Card>
    {/if}
  </main>
</div>

<style>
  .hh-settings { position: relative; z-index: 1; display: grid; grid-template-columns: minmax(172px, 22%) minmax(0, 1fr); flex: 1; min-height: 0; width: 100%; max-width: 1120px; margin: 0 auto; padding: 0 22px 24px; gap: 18px; color: var(--text-primary); }
  .hh-settings__nav { display: flex; min-height: 0; flex-direction: column; padding: 10px 0; border-top: 1px solid var(--border-hover); }
  .hh-settings__nav-head { display: grid; gap: 7px; padding: 0 4px 14px; }
  .hh-settings__nav-head span, .hh-settings__content-head span, .hh-settings-card__head span { color: var(--accent); font: 700 9px/1 var(--font-mono); letter-spacing: .14em; }
  .hh-settings__nav-head strong { font: 800 1.18rem/1 var(--font-display); }
  .hh-settings__categories { display: grid; min-height: 0; overflow-y: auto; border-top: 1px solid var(--border); }
  .hh-settings__categories button { display: grid; grid-template-columns: 22px 30px minmax(0, 1fr); align-items: center; gap: 7px; min-height: 48px; padding: 6px 5px; border: 0; border-bottom: 1px solid var(--border); background: transparent; color: var(--text-muted); text-align: left; cursor: pointer; }
  .hh-settings__categories button:hover, .hh-settings__categories button.active { background: color-mix(in srgb, var(--accent) 10%, transparent); color: var(--text-primary); }
  .hh-settings__categories button.active { box-shadow: inset 2px 0 var(--accent); }
  .hh-settings__categories b { color: var(--text-dim); font: 700 9px/1 var(--font-mono); }
  .hh-settings__cat-icon { display: grid; width: 28px; height: 28px; place-items: center; border: 1px solid var(--border); color: var(--accent-hi); }
  .hh-settings__categories button > span:last-child { display: grid; gap: 3px; min-width: 0; }
  .hh-settings__categories strong { font-size: .75rem; }
  .hh-settings__categories small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-dim); font-size: .6rem; }
  .hh-settings__nav-foot { margin-top: auto; padding: 14px 4px 0; color: var(--text-dim); font: 600 9px/1.6 var(--font-mono); }
  .hh-settings__content { min-width: 0; min-height: 0; overflow-y: auto; padding: 8px 2px 30px; scroll-behavior: smooth; outline: none; }
  .hh-settings__content-head { display: flex; align-items: end; justify-content: space-between; gap: 12px; padding: 0 2px 12px; }
  .hh-settings__content-head div { display: grid; gap: 6px; }
  .hh-settings__content-head h1 { margin: 0; font: 850 clamp(1.35rem, 3vw, 2rem)/1 var(--font-display); letter-spacing: -.04em; }
  .hh-settings__content-head small { color: var(--text-muted); font: 600 9px/1.5 var(--font-mono); }
  :global(.hh-settings-card) { display: grid; gap: 14px; border-color: color-mix(in srgb, var(--accent) 18%, var(--border)); background: linear-gradient(135deg, color-mix(in srgb, var(--bg-card) 96%, transparent), color-mix(in srgb, var(--accent) 5%, transparent)); }
  .hh-settings-card__head { display: flex; align-items: start; justify-content: space-between; gap: 12px; }
  .hh-settings-card__head div { display: grid; gap: 7px; }
  .hh-settings-card__head h2 { margin: 0; font: 800 1.2rem/1 var(--font-display); }
  :global(.hh-settings-card__head > svg) { color: var(--accent); opacity: .78; }
  .hh-theme-grid { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 7px; }
  .hh-theme-grid button { position: relative; min-width: 0; min-height: 58px; overflow: hidden; padding: 0; border: 1px solid var(--border); background: var(--bg-elev); color: #fff; text-align: left; cursor: pointer; }
  .hh-theme-grid button.active { border: 2px solid var(--accent); }
  .hh-theme-grid img { width: 100%; height: 100%; min-height: 58px; object-fit: cover; opacity: .66; }
  .hh-theme-grid span { position: absolute; right: 5px; bottom: 5px; left: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font: 700 9px/1 var(--font-ui); text-shadow: 0 1px 5px #000; }
  .hh-setting-row { display: flex; align-items: center; justify-content: space-between; gap: 14px; min-height: 48px; padding: 9px 0; border-top: 1px solid var(--border); }
  .hh-setting-row > div { display: grid; gap: 4px; min-width: 0; }
  .hh-setting-row strong { font-size: .78rem; }
  .hh-setting-row small, :global(.hh-settings-card p), .hh-settings-note-copy { color: var(--text-muted); font-size: .7rem; line-height: 1.55; }
  .hh-settings-subhead { padding-top: 4px; color: var(--text-muted); font: 700 9px/1 var(--font-mono); letter-spacing: .14em; }
  .hh-choice-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
  .hh-choice-grid button { min-height: 44px; display: flex; align-items: center; justify-content: center; gap: 7px; border: 1px solid var(--border); background: transparent; color: var(--text-muted); cursor: pointer; }
  .hh-choice-grid button.active, .hh-choice-grid button:hover { border-color: var(--accent-ring); background: var(--accent-lo); color: var(--text-primary); }
  .hh-choice-grid strong { font-size: .72rem; }
  .hh-mapping { display: grid; gap: 7px; padding: 12px; border: 1px solid var(--border); background: color-mix(in srgb, var(--bg-elev) 74%, transparent); }
  .hh-mapping strong { font-size: .75rem; }
  .hh-mapping span { color: var(--text-muted); font: 600 .65rem/1.6 var(--font-mono); }
  .hh-maintenance-stat { display: flex; align-items: baseline; gap: 12px; padding: 16px; border-left: 3px solid var(--accent); background: color-mix(in srgb, var(--accent) 7%, transparent); }
  .hh-maintenance-stat strong { font: 800 1.45rem/1 var(--font-display); }
  .hh-maintenance-stat span { color: var(--text-muted); font-size: .7rem; }
  .hh-setting-actions { display: flex; gap: 8px; }
  :global(.hh-settings .s-section) { margin: 0; }
  @media (orientation: landscape) and (max-height: 620px) { .hh-settings { padding-inline: 14px; gap: 12px; } .hh-settings__nav { padding-block: 4px; } .hh-settings__nav-head { padding-bottom: 8px; } .hh-settings__nav-foot { padding-top: 6px; } .hh-settings__categories button { min-height: 42px; } .hh-settings__cat-icon { width: 25px; height: 25px; } .hh-settings__content { padding-top: 2px; } .hh-settings__content-head { padding-bottom: 8px; } .hh-theme-grid button, .hh-theme-grid img { min-height: 44px; } :global(.hh-settings-card) { gap: 9px; } .hh-setting-row { min-height: 40px; padding-block: 5px; } }
  @media (max-width: 620px) { .hh-settings { grid-template-columns: 132px minmax(0, 1fr); padding-inline: 10px; gap: 10px; } .hh-settings__nav-head strong { font-size: .95rem; } .hh-settings__categories button { grid-template-columns: 19px 26px minmax(0, 1fr); gap: 4px; } .hh-settings__cat-icon { width: 24px; height: 24px; } .hh-settings__categories strong { font-size: .68rem; } .hh-settings__categories small { font-size: .54rem; } .hh-settings__content-head small { display: none; } .hh-theme-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
  @media (prefers-reduced-motion: reduce) { .hh-settings__content { scroll-behavior: auto; } }
</style>
