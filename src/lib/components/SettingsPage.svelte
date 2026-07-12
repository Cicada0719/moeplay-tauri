<script lang="ts">
  import { onMount, tick } from "svelte";
  import gsap from "gsap";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { THEME_PACKS, normalizeAppearance, type ColorMode, type ThemePackId } from "../theme-packs";
  import { secretDelete, secretSet, secretStatus, updateNsfwDisplayMode, setAutostart, syncSteamAchievements, pickImageFile, importWallpaper, getWallpaperAttribution, type NsfwDisplayMode, type WallpaperAttribution } from "../api";
  import { gameStore } from "../stores/games.svelte";
  import { animeStore } from "../stores/anime.svelte";
  import { wallpaperStore } from "../stores/wallpapers.svelte";
  import Button from "./ui/Button.svelte";
  import Card from "./ui/Card.svelte";
  import SegmentControl from "./ui/SegmentControl.svelte";
  import Switch from "./ui/Switch.svelte";
  import Input from "./ui/Input.svelte";
  import Tag from "./ui/Tag.svelte";
  import Icon from "./Icon.svelte";
  import UpdateDialog from "./UpdateDialog.svelte";

  let showUpdateDialog = $state(false);
  let wallpaperAttribution = $state<WallpaperAttribution | null>(null);
  let showWallpaperAttribution = $state(false);

  const nsfwModes: { id: NsfwDisplayMode; label: string }[] = [
    { id: "blur", label: "模糊" },
    { id: "show", label: "显示" },
    { id: "hide", label: "隐藏" },
  ];

  const colorModes: { id: ColorMode; label: string }[] = [
    { id: "pack-default", label: "主题默认" }, { id: "system", label: "跟随系统" }, { id: "light", label: "浅色" },
    { id: "dark", label: "深色" }, { id: "black", label: "纯黑" }, { id: "contrast", label: "高对比" },
  ];

  const startupModes = [
    { id: "dashboard", label: "普通模式", desc: "最大化窗口，保留任务栏", icon: "monitor" },
    { id: "windowed", label: "窗口模式", desc: "可自由调整大小的窗口", icon: "square" },
    { id: "fullscreen", label: "全屏模式", desc: "全屏显示，隐藏任务栏", icon: "maximize" },
    { id: "big-picture", label: "大屏模式", desc: "全屏 + 大屏游戏中心", icon: "tv" },
  ];

  const scrapeSources = [
    { key: "vndb_enabled" as const, label: "VNDB", desc: "视觉小说数据库" },
    { key: "bangumi_enabled" as const, label: "Bangumi", desc: "番组计划" },
    { key: "dlsite_enabled" as const, label: "DLsite", desc: "同人 / 商业游戏" },
    { key: "kungal_enabled" as const, label: "Kungal", desc: "中文 Galgame 聚合" },
    { key: "steam_enabled" as const, label: "Steam", desc: "Steam 商店元数据" },
    { key: "pcgw_enabled" as const, label: "PCGW", desc: "PC 游戏技术资料" },
    { key: "erogamescape_enabled" as const, label: "批评空间", desc: "EGS 日本评分" },
    { key: "ymgal_enabled" as const, label: "月幕 Ymgal", desc: "Galgame 中文社区" },
    { key: "touchgal_enabled" as const, label: "TouchGAL", desc: "中文 Galgame 资讯" },
  ];

  let savingAutostart = $state(false);
  let syncingAchievements = $state(false);
  let achievementMsg = $state("");
  let contentEl: HTMLElement | undefined = $state();
  let aiKeyInput = $state("");
  let aiSecretConfigured = $state(false);
  let aiSecretBusy = $state(false);
  let aiSecretMessage = $state("");

  // Bangumi settings state
  let bangumiTokenInput = $state("");
  let bangumiConnecting = $state(false);
  let bangumiConnectMsg = $state("");
  let bangumiSyncing = $state(false);

  onMount(async () => {
    await refreshAiSecretStatus();
    await tick();
    if (!contentEl) return;
    const panels = contentEl.querySelectorAll(".s-section");
    if (panels.length) {
      gsap.from(panels, {
        y: 24,
        opacity: 0,
        duration: 0.5,
        stagger: 0.06,
        ease: "power3.out",
        clearProps: "all",
      });
    }
  });

  function isSourceEnabled(key: string): boolean {
    return !!(settingsStore.settings as any)[key];
  }

  async function save() {
    await settingsStore.save(settingsStore.settings);
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

  async function setNsfw(mode: NsfwDisplayMode) {
    const settings = await updateNsfwDisplayMode(mode);
    await settingsStore.save(settings);
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

  async function setStartupMode(mode: string) {
    await settingsStore.save({ ...settingsStore.settings, startup_mode: mode });
    if (settingsStore.settings.autostart_enabled) {
      try { await setAutostart(true, mode); } catch { /* ignore */ }
    }
    // 立即对当前窗口应用，给即时反馈（dev/web 下 getCurrentWindow 会抛错，已 catch）
    try {
      const win = getCurrentWindow();
      if (mode === "fullscreen" || mode === "big-picture") {
        await win.setFullscreen(true);
      } else {
        await win.setFullscreen(false);
        await win.maximize();
      }
    } catch { /* 非 Tauri 环境忽略 */ }
    const label = startupModes.find((m) => m.id === mode)?.label ?? mode;
    uiStore.notify(
      mode === "big-picture" ? `已设为${label}，重启后进入大屏中心` : `启动模式已切换：${label}`,
      "success",
    );
  }

  async function handleSyncAchievements() {
    syncingAchievements = true;
    achievementMsg = "";
    try {
      const r = await syncSteamAchievements();
      await gameStore.load();
      achievementMsg = `已同步 ${r.synced}，跳过 ${r.skipped}，失败 ${r.failed}`;
      uiStore.notify(achievementMsg, r.failed > 0 ? "error" : "success");
    } catch (e: any) {
      achievementMsg = e?.message ?? String(e);
      uiStore.notify("成就同步失败: " + achievementMsg, "error");
    } finally {
      syncingAchievements = false;
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

  async function updateAppearance(patch: Record<string, unknown>) {
    await settingsStore.setAppearance({ ...normalizeAppearance(settingsStore.settings.appearance), ...patch });
  }

  async function chooseThemePack(theme_pack: ThemePackId) {
    await updateAppearance({ theme_pack, fixed_wallpaper_id: undefined });
    uiStore.notify("主题包已切换", "success");
  }

  async function pickCustomImage(kind: "wallpaper" | "mascot") {
    try {
      if (kind === "wallpaper") {
        const wallpaper = await importWallpaper();
        if (!wallpaper?.local_path) return;
        await updateAppearance({ custom_wallpaper_path: wallpaper.local_path, wallpaper_rotation: "fixed", fixed_wallpaper_id: wallpaper.asset.id });
        await wallpaperStore.load();
      } else {
        const path = await pickImageFile();
        await updateAppearance({ custom_mascot_path: path });
      }
    } catch (error) { uiStore.notify(`导入图片失败：${String(error)}`, "error"); }
  }

  async function openWallpaperAttribution() {
    const appearance = normalizeAppearance(settingsStore.settings.appearance);
    if (!appearance.fixed_wallpaper_id || appearance.fixed_wallpaper_id.startsWith("builtin:")) {
      wallpaperAttribution = { id: appearance.fixed_wallpaper_id ?? "builtin", title: "内置主题壁纸", author: "MoePlay / 自用主题素材", source_url: "", license_id: "Bundled", license_url: "", attribution_required: false };
    } else {
      try { wallpaperAttribution = await getWallpaperAttribution(appearance.fixed_wallpaper_id); }
      catch { wallpaperAttribution = null; }
    }
    showWallpaperAttribution = true;
  }

  async function rotateWallpaper() {
    const appearance = normalizeAppearance(settingsStore.settings.appearance);
    const pack = THEME_PACKS.find((item) => item.id === appearance.theme_pack) ?? THEME_PACKS[0];
    const current = pack.wallpapers.findIndex((item) => item.id === appearance.fixed_wallpaper_id);
    const next = pack.wallpapers[(current + 1 + pack.wallpapers.length) % pack.wallpapers.length];
    await updateAppearance({ wallpaper_rotation: "fixed", fixed_wallpaper_id: next.id, custom_wallpaper_path: undefined });
  }

  async function handleBangumiConnect() {
    if (!bangumiTokenInput.trim()) {
      bangumiConnectMsg = "请输入 Access Token";
      return;
    }
    bangumiConnecting = true;
    bangumiConnectMsg = "";
    try {
      const username = await animeStore.setBangumiToken(bangumiTokenInput.trim());
      bangumiConnectMsg = `已连接: ${username}`;
      uiStore.notify(`Bangumi 已连接: ${username}`, "success");
    } catch (e) {
      bangumiConnectMsg = `连接失败: ${e}`;
    } finally {
      bangumiConnecting = false;
    }
  }

  async function handleBangumiDisconnect() {
    animeStore.disconnectBangumi();
    bangumiTokenInput = "";
    bangumiConnectMsg = "";
    uiStore.notify("已断开 Bangumi 连接", "success");
  }

  async function handleBangumiSync() {
    bangumiSyncing = true;
    try {
      await animeStore.loadBangumiCollection();
      await animeStore.syncBangumiToLocal();
      uiStore.notify(animeStore.bangumiSyncProgress || "同步完成", "success");
    } catch (e) {
      uiStore.notify(`同步失败: ${e}`, "error");
    } finally {
      bangumiSyncing = false;
    }
  }
</script>

<section class="stg">
  <header class="stg-head">
    <div class="stg-head-accent"></div>
    <h1>设置</h1>
    <p class="stg-subtitle">外观、数据源、库管理与高级选项</p>
  </header>

  <div class="stg-workspace">
    <aside class="stg-index" aria-label="设置分类">
      <span>SETTINGS / INDEX</span>
      <a href="#settings-appearance"><b>01</b><em>常用与外观</em></a>
      <a href="#settings-scrape"><b>02</b><em>数据刮削</em></a>
      <a href="#settings-library"><b>03</b><em>库与导入</em></a>
      <a href="#settings-bangumi"><b>04</b><em>Bangumi</em></a>
      <a href="#settings-player"><b>05</b><em>番剧播放器</em></a>
      <a href="#settings-advanced"><b>06</b><em>高级</em></a>
      <small>宽屏目录支持快速跳转。</small>
    </aside>
    <main class="stg-content" bind:this={contentEl}>

    <!-- 常用 -->
    <span class="section-anchor" id="settings-appearance" aria-hidden="true"></span>
    <Card class="s-section" padding="lg" ariaLabel="settings-appearance">
      <div class="s-head">
        <h2 class="s-title"><Icon name="eye" size={17} className="s-title-ic" /> 常用</h2>
      </div>

      <div class="theme-pack-grid" aria-label="二次元主题包">
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
            <span class="theme-pack-card__copy"><b>{pack.label}</b><small>{pack.description}</small></span>
          </button>
        {/each}
      </div>

      <div class="s-row">
        <div class="s-info"><span class="s-label">显示模式</span><span class="s-desc">覆盖主题包默认的明暗模式</span></div>
        <SegmentControl options={colorModes.map(m => ({ value: m.id, label: m.label }))} value={normalizeAppearance(settingsStore.settings.appearance).color_mode} onChange={(v) => updateAppearance({ color_mode: v })} size="sm" />
      </div>

      <div class="s-row wallpaper-setting">
        <div class="s-info"><span class="s-label">主题壁纸</span><span class="s-desc">启动随机、固定内置壁纸，或导入本地图片</span></div>
        <div class="wallpaper-actions">
          <Button variant="ghost" size="sm" press={rotateWallpaper}>换一张</Button>
          <Button variant="ghost" size="sm" press={() => updateAppearance({ wallpaper_rotation: "startup-random", fixed_wallpaper_id: undefined, custom_wallpaper_path: undefined })}>启动随机</Button>
          <Button variant="ghost" size="sm" press={() => pickCustomImage("wallpaper")}>导入壁纸</Button>
          <Button variant="ghost" size="sm" press={openWallpaperAttribution}>关于壁纸</Button>
        </div>
      </div>

      <div class="s-row">
        <div class="s-info"><span class="s-label">动态装饰</span><span class="s-desc">樱花、光点或数字雨；减少动态时自动关闭</span></div>
        <Switch checked={normalizeAppearance(settingsStore.settings.appearance).decorative_effects} onchange={() => updateAppearance({ decorative_effects: !normalizeAppearance(settingsStore.settings.appearance).decorative_effects })} />
      </div>

      <div class="s-row">
        <div class="s-info"><span class="s-label">在线精选图库</span><span class="s-desc">后台同步经过许可审核的官方壁纸索引</span></div>
        <Switch checked={normalizeAppearance(settingsStore.settings.appearance).online_gallery_enabled} onchange={() => updateAppearance({ online_gallery_enabled: !normalizeAppearance(settingsStore.settings.appearance).online_gallery_enabled })} />
      </div>

      <div class="s-row" style="align-items:flex-start">
        <div class="s-info"><span class="s-label">精选看板娘</span><span class="s-desc">只在首页、空状态和主题预览出现</span></div>
        <div class="s-col">
          <Switch checked={normalizeAppearance(settingsStore.settings.appearance).mascot_enabled} onchange={() => updateAppearance({ mascot_enabled: !normalizeAppearance(settingsStore.settings.appearance).mascot_enabled })} />
          {#if normalizeAppearance(settingsStore.settings.appearance).mascot_enabled}
            <div class="wallpaper-actions"><Button variant="ghost" size="sm" press={() => pickCustomImage("mascot")}>自定义立绘</Button><Button variant="ghost" size="sm" press={() => updateAppearance({ custom_mascot_path: undefined })}>恢复主题立绘</Button></div>
          {/if}
        </div>
      </div>

      {#if showWallpaperAttribution}
        <div class="wallpaper-attribution" role="status">
          <div><b>{wallpaperAttribution?.title ?? "暂无归属信息"}</b>{#if wallpaperAttribution}<span>{wallpaperAttribution.author} · {wallpaperAttribution.license_id}</span>{/if}</div>
          <button type="button" aria-label="关闭壁纸信息" onclick={() => (showWallpaperAttribution = false)}><Icon name="x" size={14} /></button>
        </div>
      {/if}

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
            <span class="mode-desc">{mode.desc}</span>
          </Card>
        {/each}
      </div>
      <p class="s-note">普通 / 全屏：选择后立即生效；大屏：下次启动进入大屏中心。</p>
    </Card>

    <!-- 数据源 -->
    <Card class="s-section" padding="lg" ariaLabel="settings-scrape">
      <div class="s-head">
        <h2 class="s-title"><Icon name="layers" size={17} className="s-title-ic" /> 数据刮削</h2>
        <div class="s-head-actions">
          <Button variant="ghost" size="sm" press={() => setAllSources(true)}>全开</Button>
          <Button variant="ghost" size="sm" press={() => setAllSources(false)}>全关</Button>
        </div>
      </div>
      <p class="s-note">选择从哪些数据库获取游戏元数据（封面、简介、标签等）</p>
      <div class="src-grid">
        {#each scrapeSources as src}
          <div class="src-item">
            <div class="src-info">
              <span class="src-name">{src.label}</span>
              <span class="src-desc">{src.desc}</span>
            </div>
            <Switch checked={isSourceEnabled(src.key)} onchange={() => toggleScrapeSetting(src.key)} />
          </div>
        {/each}
      </div>
      <div class="s-divider"></div>
      <div class="src-item">
        <div class="src-info">
          <span class="src-name">自动刮削</span>
          <span class="src-desc">添加游戏时自动搜索并填充元数据</span>
        </div>
        <Switch checked={settingsStore.settings.auto_scrape} onchange={() => toggleScrapeSetting("auto_scrape")} />
      </div>
      <div class="s-divider"></div>
      <div class="s-row" style="flex-direction: column; align-items: stretch; gap: 6px;">
        <div class="s-info">
          <span class="s-label">HTTP 代理</span>
          <span class="s-desc">刮削数据源时使用的代理地址，留空则使用系统代理</span>
        </div>
        <Input
          bind:value={settingsStore.settings.scraper_proxy}
          onblur={save}
          placeholder="如 http://127.0.0.1:7890（留空 = 系统代理）"
        />
      </div>
    </Card>

    <!-- 库与导入 -->
    <span class="section-anchor" id="settings-library" aria-hidden="true"></span>
    <Card class="s-section" padding="lg" ariaLabel="settings-library">
      <div class="s-head">
        <h2 class="s-title"><Icon name="folder" size={17} className="s-title-ic" /> 库与导入</h2>
      </div>

      <div class="s-info" style="padding-bottom: 10px;">
        <span class="s-label">扫描目录</span>
        <span class="s-desc">萌游会监视这些目录中的游戏</span>
      </div>
      {#if settingsStore.settings.watch_dirs.length > 0}
        <div class="dir-list">
          {#each settingsStore.settings.watch_dirs as dir}
            <Card class="dir-item" padding="sm" hoverable>
              <span class="dir-path">{dir}</span>
              <button class="dir-remove" onclick={() => settingsStore.removeWatchDir(dir)} title="移除" type="button">
                <Icon name="x" size={14} />
              </button>
            </Card>
          {/each}
        </div>
      {:else}
        <div class="s-empty">
          <Icon name="folder" size={28} />
          <span>尚未添加扫描目录</span>
        </div>
      {/if}
      <div style="padding: 12px 0 4px;">
        <Button variant="secondary" press={() => settingsStore.addWatchDir()}>添加目录</Button>
      </div>

      <div class="s-divider"></div>
      <div class="ops-list">
        <div class="ops-item">
          <div class="ops-info">
            <Icon name="steam" size={18} className="ops-icon" />
            <div>
              <span class="s-label">Steam / Epic 导入</span>
              <span class="s-desc">扫描本机已安装游戏，或通过登录同步完整库</span>
            </div>
          </div>
          <Button variant="primary" size="sm" press={() => uiStore.currentView = "steam-import"}>打开</Button>
        </div>
        <div class="ops-item">
          <div class="ops-info">
            <Icon name="star" size={18} className="ops-icon" />
            <div>
              <span class="s-label">同步 Steam 成就</span>
              <span class="s-desc">从 Steam Web API 拉取成就数据（需 API Key + SteamID）</span>
            </div>
          </div>
          <Button variant="primary" size="sm" press={handleSyncAchievements} disabled={syncingAchievements}>
            {syncingAchievements ? "同步中..." : "同步"}
          </Button>
        </div>
        {#if achievementMsg}
          <div class="ops-msg">{achievementMsg}</div>
        {/if}
      </div>
    </Card>

    <!-- Bangumi 收藏同步 -->
    <span class="section-anchor" id="settings-bangumi" aria-hidden="true"></span>
    <Card class="s-section" padding="lg" ariaLabel="settings-bangumi">
      <div class="s-head">
        <h2 class="s-title"><Icon name="heart" size={17} className="s-title-ic" /> Bangumi 收藏同步</h2>
      </div>
      <p class="s-note">连接你的 Bangumi 账号，自动同步番剧收藏状态</p>

      <div class="ops-list">
        <div class="ops-item" style="flex-direction: column; align-items: stretch; gap: 10px;">
          <div class="s-info">
            <span class="s-label">Access Token</span>
            <span class="s-desc">从 Bangumi 个人设置 → 开发者 → Access Token 获取</span>
          </div>
          <div style="display: flex; gap: 8px; align-items: center;">
            <div style="flex: 1;">
              <Input
                type="password"
                bind:value={bangumiTokenInput}
                placeholder="粘贴你的 Bangumi Access Token"
              />
            </div>
            {#if animeStore.bangumiConnected}
              <Button variant="ghost" size="sm" press={handleBangumiDisconnect}>断开</Button>
            {:else}
              <Button variant="primary" size="sm" press={handleBangumiConnect} disabled={bangumiConnecting}>
                {bangumiConnecting ? "连接中..." : "连接"}
              </Button>
            {/if}
          </div>
          {#if bangumiConnectMsg}
            <div class="ops-msg">{bangumiConnectMsg}</div>
          {/if}
          {#if animeStore.bangumiConnected}
            <div class="ops-msg" style="color: #58d68d;">
              ✓ 已连接: {animeStore.bangumiUsername}
            </div>
          {/if}
        </div>
      </div>

      {#if animeStore.bangumiConnected}
        <div class="s-divider"></div>

        <div class="ops-list">
          <div class="ops-item">
            <div class="ops-info">
              <Icon name="refresh" size={18} className="ops-icon" />
              <div>
                <span class="s-label">同步收藏</span>
                <span class="s-desc">拉取 Bangumi 收藏并与本地合并</span>
              </div>
            </div>
            <Button variant="primary" size="sm" press={handleBangumiSync} disabled={bangumiSyncing}>
              {bangumiSyncing ? "同步中..." : "同步"}
            </Button>
          </div>

          <div class="ops-item">
            <div class="ops-info">
              <span class="s-label" style="margin-left: 26px;">同步优先级</span>
              <span class="s-desc">冲突时以哪方为准</span>
            </div>
            <SegmentControl
              options={[
                { value: "0", label: "本地优先" },
                { value: "1", label: "Bangumi 优先" },
              ]}
              value={String(animeStore.bangumiSyncPriority)}
              onChange={(v) => animeStore.bangumiSyncPriority = parseInt(v)}
              size="sm"
            />
          </div>
        </div>

        {#if animeStore.bangumiSyncProgress}
          <div class="ops-msg">{animeStore.bangumiSyncProgress}</div>
        {/if}
        {#if animeStore.bangumiSyncError}
          <div class="ops-msg" style="color: #e8557f;">{animeStore.bangumiSyncError}</div>
        {/if}
      {/if}
    </Card>

    <!-- 番剧播放器 -->
    <span class="section-anchor" id="settings-player" aria-hidden="true"></span>
    <Card class="s-section" padding="lg" ariaLabel="settings-player">
      <div class="s-head">
        <h2 class="s-title"><Icon name="film" size={17} className="s-title-ic" /> 番剧播放器</h2>
      </div>

      <div class="src-item">
        <div class="src-info">
          <span class="src-name">自动连播</span>
          <span class="src-desc">一集播完后自动播放下一集</span>
        </div>
        <Switch checked={animeStore.autoNext} onchange={() => animeStore.autoNext = !animeStore.autoNext} />
      </div>

      <div class="src-item">
        <div class="src-info">
          <span class="src-name">默认开启弹幕</span>
          <span class="src-desc">进入播放器时自动加载弹幕</span>
        </div>
        <Switch checked={animeStore.danmakuEnabled} onchange={() => animeStore.danmakuEnabled = !animeStore.danmakuEnabled} />
      </div>

      <div class="s-divider"></div>

      <div class="s-row">
        <div class="s-info">
          <span class="s-label">跳过片头（秒）</span>
          <span class="s-desc">每集开始自动跳到第 N 秒，0 表示不跳</span>
        </div>
        <Input
          class="num-input"
          type="number"
          min="0"
          max="300"
          step="1"
          value={String(animeStore.skipOpening)}
          oninput={(e) => animeStore.skipOpening = Math.max(0, parseInt((e.target as HTMLInputElement).value) || 0)}
        />
      </div>

      <div class="s-row">
        <div class="s-info">
          <span class="s-label">跳过片尾（秒）</span>
          <span class="s-desc">距结尾 N 秒时自动跳下一集，0 表示不跳</span>
        </div>
        <Input
          class="num-input"
          type="number"
          min="0"
          max="300"
          step="1"
          value={String(animeStore.skipEnding)}
          oninput={(e) => animeStore.skipEnding = Math.max(0, parseInt((e.target as HTMLInputElement).value) || 0)}
        />
      </div>

      <div class="s-divider"></div>

      <div class="s-row" style="flex-direction: column; align-items: flex-start; gap: 8px;">
        <div class="s-info">
          <span class="s-label">默认倍速</span>
          <span class="s-desc">进入播放器时的初始倍速</span>
        </div>
        <SegmentControl
          options={[0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0].map(r => ({ value: String(r), label: `${r}x` }))}
          value={String(animeStore.playbackRate)}
          onChange={(v) => animeStore.playbackRate = parseFloat(v)}
          size="sm"
        />
      </div>

      <div class="s-row" style="flex-direction: column; align-items: flex-start; gap: 8px;">
        <div class="s-info">
          <span class="s-label">长按倍速</span>
          <span class="s-desc">长按画面时临时切换到的倍速</span>
        </div>
        <SegmentControl
          options={[1.5, 2.0, 3.0, 4.0].map(r => ({ value: String(r), label: `${r}x` }))}
          value={String(animeStore.longPressRate)}
          onChange={(v) => animeStore.longPressRate = parseFloat(v)}
          size="sm"
        />
      </div>
    </Card>

    <!-- 高级 -->
    <span class="section-anchor" id="settings-advanced" aria-hidden="true"></span>
    <Card class="s-section" padding="lg" ariaLabel="settings-advanced">
      <div class="s-head">
        <h2 class="s-title"><Icon name="toolbox" size={17} className="s-title-ic" /> 高级</h2>
      </div>

      <div class="src-item">
        <div class="src-info">
          <span class="src-name">启用 AI 增强</span>
          <span class="src-desc">使用大语言模型辅助刮削和补全</span>
        </div>
        <Switch checked={settingsStore.settings.ai_enabled} onchange={save} />
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

      <div class="s-divider"></div>
      <div class="about-block">
        <div class="about-name">
          <span class="about-brand">萌游</span>
          <Tag variant="accent" size="sm" class="about-ver">v0.13.6</Tag>
        </div>
        <p class="about-tagline">可爱的游戏管理器</p>
        <div class="about-stack">
          <Tag variant="muted" size="sm">Tauri v2</Tag>
          <Tag variant="muted" size="sm">Svelte 5</Tag>
          <Tag variant="muted" size="sm">Rust</Tag>
        </div>
        <Button variant="ghost" size="sm" press={() => showUpdateDialog = true}>
          <Icon name="download" size={14} /> 检查更新
        </Button>
      </div>
    </Card>

    </main>
  </div>
</section>

<UpdateDialog bind:open={showUpdateDialog} />

<style>
  .stg {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--text-primary);
  }

  /* ── Header ── */
  .stg-head {
    position: relative;
    width: 100%;
    max-width: 1280px;
    margin: 0 auto;
    padding: 26px 28px 14px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex-shrink: 0;
  }
  .stg-head-accent {
    position: absolute;
    left: 0; top: 30px; bottom: 12px;
    width: 3px;
    background: linear-gradient(180deg, var(--accent), var(--accent-hi));
    border-radius: 0 2px 2px 0;
  }
  .stg h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 26px;
    font-weight: 750;
    line-height: 1.15;
    padding-left: 14px;
  }
  .stg-subtitle {
    margin: 0;
    padding-left: 14px;
    color: var(--text-muted);
    font-size: 13px;
  }

  /* ── Wide settings workspace ── */
  .stg-workspace { flex:1; min-height:0; width:100%; max-width:1280px; margin:0 auto; padding:0 28px 36px; display:grid; grid-template-columns:220px minmax(0,1fr); gap:28px; overflow:hidden; }
  .stg-index { align-self:start; position:sticky; top:0; display:grid; padding-top:10px; border-top:1px solid var(--border-hover); }
  .stg-index>span { padding:0 0 14px; color:var(--accent); font:700 8px/1 var(--font-mono); letter-spacing:.16em; }
  .stg-index a { display:grid; grid-template-columns:32px 1fr; align-items:center; min-height:42px; border-top:1px solid var(--border); color:var(--text-secondary); text-decoration:none; transition:padding-left .28s var(--ui-ease-out),color .2s ease,background .2s ease; }
  .stg-index a:last-of-type { border-bottom:1px solid var(--border); }
  .stg-index a:hover,.stg-index a:focus-visible { padding-left:7px; color:var(--text-primary); background:color-mix(in srgb,var(--accent) 6%,transparent); outline:0; }
  .stg-index b { color:var(--text-dim); font:700 8px/1 var(--font-mono); }
  .stg-index em { font:650 12px/1 var(--font-ui); font-style:normal; }
  .stg-index small { margin-top:16px; color:var(--text-dim); font-size:10px; line-height:1.6; }
  .stg-content { min-height:0; overflow-y:auto; width:100%; padding:8px 2px 40px; display:flex; flex-direction:column; gap:14px; scroll-behavior:smooth; }
  .section-anchor { display:block; height:1px; margin-top:-1px; scroll-margin-top:8px; }

  /* ── Section panel ── */
  :global(.s-section) { transition: border-color 0.3s ease; }
  :global(.s-section):hover { border-color: var(--border-hover); }

  /* ── Section head ── */
  .s-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 14px;
    margin-bottom: 4px;
    border-bottom: 1px solid var(--aura-line, rgba(255, 255, 255, 0.06));
  }
  .s-title {
    display: flex;
    align-items: center;
    gap: 9px;
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
  }
  :global(.s-title-ic) { color: var(--accent); opacity: 0.85; }
  .s-head-actions { display: flex; gap: 6px; }

  /* ── Row / Info ── */
  .s-row {
    display: grid;
    grid-template-columns: minmax(0,1fr) auto;
    gap: 16px;
    align-items: center;
    padding: 14px 0;
    border-bottom: 1px solid var(--aura-line, rgba(255, 255, 255, 0.06));
  }
  .s-info { display: flex; flex-direction: column; gap: 2px; }
  .s-label { font-size: 13.5px; font-weight: 650; color: var(--text-primary); }
  .s-desc { font-size: 12px; color: var(--text-muted); line-height: 1.4; }
  .s-col { display: flex; flex-direction: column; gap: 10px; align-items: flex-end; }
  .s-note { margin: 0; padding: 8px 0 2px; font-size: 12.5px; color: var(--text-muted); line-height: 1.5; }
  .s-divider { height: 1px; background: var(--aura-line, rgba(255, 255, 255, 0.06)); margin: 14px 0; }
  .s-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 28px 0;
    color: var(--text-dim);
    font-size: 13px;
  }

  /* ── Number input for player settings ── */
  :global(.num-input) { max-width: 90px; text-align: center; }

  /* ── Source grid ── */
  .src-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0,1fr));
    gap: 2px 22px;
  }
  .src-item {
    display: grid;
    grid-template-columns: minmax(0,1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 11px 0;
    border-bottom: 1px solid var(--aura-line, rgba(255, 255, 255, 0.06));
  }
  .src-item:last-child { border-bottom: none; }
  .src-info { display: flex; flex-direction: column; gap: 1px; }
  .src-name { font-size: 13px; font-weight: 650; color: var(--text-primary); }
  .src-desc { font-size: 11.5px; color: var(--text-muted); }

  /* ── Startup mode cards ── */
  .mode-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0,1fr));
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
    transition: all 0.25s cubic-bezier(0.16,1,0.3,1);
  }
  :global(.mode-card):active { transform: translateY(0) scale(0.98); }
  :global(.mode-card.active) {
    border-color: var(--accent-ring);
    background: var(--accent-lo);
    color: var(--text-primary);
    box-shadow: 0 0 0 1px var(--accent-ring), inset 0 1px 0 rgba(255,255,255,0.06);
  }
  .mode-icon {
    width: 40px; height: 40px;
    display: grid; place-items: center;
    border-radius: 10px;
    background: rgba(255,255,255,0.04);
    color: var(--text-muted);
    transition: color 0.2s, background 0.2s;
  }
  :global(.mode-card.active) .mode-icon {
    background: var(--accent);
    color: #fff;
  }
  .mode-label { font-size: 13px; font-weight: 700; }
  .mode-desc {
    font-size: 11px;
    color: var(--text-dim);
    text-align: center;
    line-height: 1.35;
  }
  :global(.mode-card.active) .mode-desc { color: var(--text-muted); }

  /* ── AI form ── */
  .ai-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 14px;
  }
  .ai-field {
    display: grid;
    grid-template-columns: 80px minmax(0,1fr);
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

  /* ── Directory list ── */
  .dir-list { display: flex; flex-direction: column; gap: 6px; }
  :global(.dir-item) {
    display: grid;
    grid-template-columns: minmax(0,1fr) auto;
    gap: 12px;
    align-items: center;
  }
  .dir-path {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dir-remove {
    width: 28px; height: 28px;
    display: grid; place-items: center;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s;
  }
  .dir-remove:hover {
    color: var(--color-error);
    background: rgba(248,113,113,0.08);
    border-color: rgba(248,113,113,0.15);
  }

  /* ── Operations ── */
  .ops-list { display: flex; flex-direction: column; }
  .ops-item {
    display: grid;
    grid-template-columns: minmax(0,1fr) auto;
    gap: 16px;
    align-items: center;
    padding: 14px 0;
    border-bottom: 1px solid var(--aura-line, rgba(255, 255, 255, 0.06));
  }
  .ops-item:last-child { border-bottom: none; padding-bottom: 0; }
  .ops-info { display: flex; gap: 12px; align-items: center; }
  .ops-info > div { display: flex; flex-direction: column; gap: 2px; }
  :global(.ops-icon) { color: var(--text-muted); }
  .ops-msg { padding: 6px 12px; font-size: 0.82rem; color: var(--text-muted); border-left: 2px solid var(--accent, #e8557f); margin: -4px 0 4px 30px; }

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
  .about-tagline { margin: 0; font-size: 13px; color: var(--text-muted); }
  .about-stack { display: flex; gap: 6px; padding-top: 4px; }

  /* ── Responsive ── */
  @media (max-width: 720px) {
    .src-grid { grid-template-columns: 1fr; }
    .mode-grid { grid-template-columns: 1fr; }
    .ai-field { grid-template-columns: 1fr; }
    .ai-secret-actions { grid-column: 1; }
    .s-row { grid-template-columns: 1fr; gap: 10px; }
    .stg-content { padding: 8px 16px 36px; }
    .stg-head { padding: 20px 16px 12px; }
  }

  .theme-pack-grid { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:12px; margin:4px 0 14px; }
  .theme-pack-card { position:relative; min-height:148px; overflow:hidden; padding:0; border:1px solid var(--border); border-radius:16px; background:var(--bg-card); color:white; cursor:pointer; text-align:left; transition:transform .18s var(--ease-enter),border-color .18s,box-shadow .18s; }
  .theme-pack-card:hover { transform:translateY(-3px); border-color:var(--border-hover); }
  .theme-pack-card.active { border-color:var(--accent); box-shadow:0 0 0 2px var(--accent-lo),0 16px 34px rgba(0,0,0,.28); }
  .theme-pack-card:focus-visible { outline:none; box-shadow:var(--focus-ring); }
  .theme-pack-card img,.theme-pack-card__scrim { position:absolute; inset:0; width:100%; height:100%; object-fit:cover; }
  .theme-pack-card__scrim { background:linear-gradient(0deg,rgba(3,5,12,.9),rgba(3,5,12,.05) 75%); }
  .theme-pack-card__copy { position:absolute; left:14px; right:14px; bottom:12px; display:flex; flex-direction:column; gap:3px; }
  .theme-pack-card__copy b { font-size:14px; letter-spacing:.02em; }
  .theme-pack-card__copy small { color:rgba(255,255,255,.7); font-size:10.5px; line-height:1.3; }
  .wallpaper-actions { display:flex; gap:7px; flex-wrap:wrap; justify-content:flex-end; }
  @media (max-width:760px) { .theme-pack-grid{grid-template-columns:1fr}.theme-pack-card{min-height:120px}.wallpaper-setting{align-items:flex-start}.wallpaper-actions{justify-content:flex-start} }
  .wallpaper-attribution { display:flex; align-items:center; justify-content:space-between; gap:12px; margin:8px 0 2px; padding:12px 14px; border:1px solid var(--border); border-radius:12px; background:var(--bg-hover); }
  .wallpaper-attribution div { display:flex; flex-direction:column; gap:2px; }
  .wallpaper-attribution b { font-size:13px; }
  .wallpaper-attribution span { font-size:11px; color:var(--text-muted); }
  .wallpaper-attribution button { border:0; background:transparent; color:var(--text-muted); cursor:pointer; }

  @media (max-width: 980px) { .stg-workspace { grid-template-columns:1fr; padding-inline:18px; } .stg-index { position:static; grid-template-columns:repeat(3,minmax(0,1fr)); gap:0; padding:0; } .stg-index>span,.stg-index small{grid-column:1/-1}.stg-index a{padding-inline:8px}.stg-head{padding-inline:18px} }
  @media (max-width: 620px) { .stg-index{grid-template-columns:repeat(2,minmax(0,1fr))}.stg-workspace{padding-inline:12px}.stg-head{padding-inline:12px} }
</style>

