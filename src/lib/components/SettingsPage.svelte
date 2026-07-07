<script lang="ts">
  import { onMount, tick } from "svelte";
  import gsap from "gsap";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { APP_THEMES } from "../utils/theme";
  import { updateNsfwDisplayMode, setAutostart, syncSteamAchievements, pickImageFile, type NsfwDisplayMode } from "../api";
  import { gameStore } from "../stores/games.svelte";
  import { animeStore } from "../stores/anime.svelte";
  import Button from "./ui/Button.svelte";
  import Card from "./ui/Card.svelte";
  import SegmentControl from "./ui/SegmentControl.svelte";
  import Switch from "./ui/Switch.svelte";
  import Input from "./ui/Input.svelte";
  import Tag from "./ui/Tag.svelte";
  import Icon from "./Icon.svelte";
  import UpdateDialog from "./UpdateDialog.svelte";

  let showUpdateDialog = $state(false);

  const nsfwModes: { id: NsfwDisplayMode; label: string }[] = [
    { id: "blur", label: "模糊" },
    { id: "show", label: "显示" },
    { id: "hide", label: "隐藏" },
  ];

  const themes = APP_THEMES;

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

  // Bangumi settings state
  let bangumiTokenInput = $state("");
  let bangumiConnecting = $state(false);
  let bangumiConnectMsg = $state("");
  let bangumiSyncing = $state(false);

  onMount(async () => {
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

  async function handleThemeChange(theme: string) {
    await settingsStore.save({ ...settingsStore.settings, theme });
    uiStore.notify("主题已切换", "success");
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

  <main class="stg-content" bind:this={contentEl}>

    <!-- 常用 -->
    <Card class="s-section" padding="lg">
      <div class="s-head">
        <h2 class="s-title"><Icon name="eye" size={17} className="s-title-ic" /> 常用</h2>
      </div>

      <div class="s-row">
        <div class="s-info">
          <span class="s-label">主题</span>
          <span class="s-desc">界面色彩方案</span>
        </div>
        <SegmentControl
          options={themes.map(t => ({ value: t.id, label: t.label }))}
          value={settingsStore.settings.theme ?? ""}
          onChange={handleThemeChange}
          size="sm"
        />
      </div>

      <div class="s-row">
        <div class="s-info">
          <span class="s-label">敏感内容</span>
          <span class="s-desc">NSFW 封面显示方式</span>
        </div>
        <SegmentControl
          options={nsfwModes.map(m => ({ value: m.id, label: m.label }))}
          value={settingsStore.settings.nsfw_display_mode ?? ""}
          onChange={(v) => setNsfw(v as NsfwDisplayMode)}
          size="sm"
        />
      </div>

      <div class="s-row" style="align-items: flex-start;">
        <div class="s-info">
          <span class="s-label">首页看板娘</span>
          <span class="s-desc">在最近游戏右下角显示二次元角色立绘</span>
        </div>
        <div class="s-col">
          <Switch
            checked={settingsStore.settings.home_mascot_enabled ?? true}
            onchange={async () => {
              settingsStore.settings.home_mascot_enabled = !(settingsStore.settings.home_mascot_enabled ?? true);
              await save();
            }}
          />
          {#if settingsStore.settings.home_mascot_enabled !== false}
            <div class="mascot-path-row">
              <Input
                value={settingsStore.settings.home_mascot_path ?? ""}
                placeholder="使用默认立绘"
                onblur={async (e: Event) => {
                  settingsStore.settings.home_mascot_path = (e.target as HTMLInputElement).value;
                  await save();
                }}
              />
              <Button variant="ghost" size="sm" onclick={async () => {
                try {
                  const path = await pickImageFile();
                  settingsStore.settings.home_mascot_path = path;
                  await save();
                } catch { /* 取消 */ }
              }}>选择图片</Button>
            </div>
          {/if}
        </div>
      </div>

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
    <Card class="s-section" padding="lg">
      <div class="s-head">
        <h2 class="s-title"><Icon name="layers" size={17} className="s-title-ic" /> 数据刮削</h2>
        <div class="s-head-actions">
          <Button variant="ghost" size="sm" onclick={() => setAllSources(true)}>全开</Button>
          <Button variant="ghost" size="sm" onclick={() => setAllSources(false)}>全关</Button>
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
    <Card class="s-section" padding="lg">
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
        <Button variant="secondary" onclick={() => settingsStore.addWatchDir()}>添加目录</Button>
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
          <Button variant="primary" size="sm" onclick={() => uiStore.currentView = "steam-import"}>打开</Button>
        </div>
        <div class="ops-item">
          <div class="ops-info">
            <Icon name="star" size={18} className="ops-icon" />
            <div>
              <span class="s-label">同步 Steam 成就</span>
              <span class="s-desc">从 Steam Web API 拉取成就数据（需 API Key + SteamID）</span>
            </div>
          </div>
          <Button variant="primary" size="sm" onclick={handleSyncAchievements} disabled={syncingAchievements}>
            {syncingAchievements ? "同步中..." : "同步"}
          </Button>
        </div>
        {#if achievementMsg}
          <div class="ops-msg">{achievementMsg}</div>
        {/if}
        <div class="ops-item">
          <div class="ops-info">
            <Icon name="refresh" size={18} className="ops-icon" />
            <div>
              <span class="s-label">旧版萌游迁移</span>
              <span class="s-desc">从 C# 版萌游一键迁移游戏库和元数据</span>
            </div>
          </div>
          <Button variant="primary" size="sm" onclick={() => uiStore.currentView = "migration"}>迁移</Button>
        </div>
      </div>
    </Card>

    <!-- Bangumi 收藏同步 -->
    <Card class="s-section" padding="lg">
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
              <Button variant="ghost" size="sm" onclick={handleBangumiDisconnect}>断开</Button>
            {:else}
              <Button variant="primary" size="sm" onclick={handleBangumiConnect} disabled={bangumiConnecting}>
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
            <Button variant="primary" size="sm" onclick={handleBangumiSync} disabled={bangumiSyncing}>
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
    <Card class="s-section" padding="lg">
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
    <Card class="s-section" padding="lg">
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
              onblur={save}
              placeholder="https://api.openai.com/v1/chat/completions"
            />
          </label>
          <label class="ai-field">
            <span class="ai-label">API Key</span>
            <Input
              type="password"
              bind:value={settingsStore.settings.ai_api_key}
              onblur={save}
              placeholder="sk-..."
            />
          </label>
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
          <Tag variant="accent" size="sm" class="about-ver">v0.11.3</Tag>
        </div>
        <p class="about-tagline">可爱的游戏管理器</p>
        <div class="about-stack">
          <Tag variant="muted" size="sm">Tauri v2</Tag>
          <Tag variant="muted" size="sm">Svelte 5</Tag>
          <Tag variant="muted" size="sm">Rust</Tag>
        </div>
        <Button variant="ghost" size="sm" onclick={() => showUpdateDialog = true}>
          <Icon name="download" size={14} /> 检查更新
        </Button>
      </div>
    </Card>

  </main>
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
    max-width: 880px;
    margin: 0 auto;
    padding: 26px 24px 14px;
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

  /* ── Content (single column, centered) ── */
  .stg-content {
    flex: 1;
    overflow-y: auto;
    width: 100%;
    max-width: 880px;
    margin: 0 auto;
    padding: 8px 24px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-behavior: smooth;
  }

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
  .mascot-path-row { display: flex; gap: 8px; align-items: center; width: 260px; }
  .mascot-path-row :global(.ui-input) { flex: 1; font-size: 12px; padding: 8px 10px; }
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
    .s-row { grid-template-columns: 1fr; gap: 10px; }
    .stg-content { padding: 8px 16px 36px; }
    .stg-head { padding: 20px 16px 12px; }
  }
</style>
