<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    getPlatformImportStatus,
    importPlatformLibrary,
    importSteamSessionGames,
    openUrl,
    resolveSteamId,
    scanPlatformLibrary,
    secretDelete,
    steamDetectLocal,
    steamLoginOpenid,
    syncSteamAchievements,
    validateSteamApiKey,
    type PlatformGameCandidate,
    type PlatformImportResult,
    type PlatformImportStatus,
    type PlatformScanResult,
    type SteamSessionGame,
    type SyncAchievementsResult,
  } from "../api";
  import { gameStore } from "../stores/games.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import defaultLibraryBackdrop from "../assets/default-library-backdrop.png";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, Input, StatBlock, Tag } from "./ui";
  import type { ApplyImportResponse, PreviewImportRequest } from "../features/library";
  import LibraryFeatureToggle from "./library/LibraryFeatureToggle.svelte";
  import LibraryV2ImportPanel from "./library/LibraryV2ImportPanel.svelte";
  import { readLibraryV2Flag } from "./library/feature-flag";
  import {
    platformCandidatesToPreviewRequest,
    steamSessionGamesToPreviewRequest,
  } from "./library/platform-candidates";

  type SectionKey = "steamAccount" | "steamLocal" | "epicLocal";
  type AggregateImportSummary = {
    imported: number;
    updated: number;
    skipped: number;
    failed: number;
    total: number;
    sections: string[];
    errors: string[];
  };
  type CandidateListProps = {
    title: string;
    scan: PlatformScanResult | null;
    selected: Set<string>;
    importing: boolean;
    result: PlatformImportResult | null;
    section: SectionKey;
    showInstalled?: boolean;
    previewMode?: boolean;
    onToggle: (section: SectionKey, game: PlatformGameCandidate) => void;
    onToggleAll: (section: SectionKey, games: PlatformGameCandidate[]) => void;
    onImport: (section: SectionKey) => Promise<void>;
  };

  let status = $state<PlatformImportStatus | null>(null);
  let steamIdInput = $state("");
  let steamProfileInput = $state("");
  let apiKeyInput = $state("");
  let steamLoginMessage = $state("未连接");
  let apiKeyMessage = $state("");
  let statusError = $state("");

  let loadingStatus = $state(false);
  let detectingSteam = $state(false);
  let resolvingSteam = $state(false);
  let verifyingKey = $state(false);
  let openingLogin = $state(false);
  let syncingSteam = $state(false);
  let importingAll = $state(false);
  let allImportSummary = $state<AggregateImportSummary | null>(null);
  let allImportError = $state("");

  let scanningSteamLocal = $state(false);
  let importingSteamLocal = $state(false);
  let steamLocalScan = $state<PlatformScanResult | null>(null);
  let steamLocalSelected = $state<Set<string>>(new Set());
  let steamLocalImport = $state<PlatformImportResult | null>(null);
  let steamLocalError = $state("");

  let steamAccountScan = $state<PlatformScanResult | null>(null);
  let steamAccountSelected = $state<Set<string>>(new Set());
  let steamAccountImport = $state<PlatformImportResult | null>(null);
  let steamAccountError = $state("");

  let scanningEpic = $state(false);
  let importingEpic = $state(false);
  let epicScan = $state<PlatformScanResult | null>(null);
  let epicSelected = $state<Set<string>>(new Set());
  let epicImport = $state<PlatformImportResult | null>(null);
  let epicError = $state("");

  let syncingAchievements = $state(false);
  let achievementResult = $state<SyncAchievementsResult | null>(null);
  let achievementError = $state("");

  let libraryV2Enabled = $state(false);
  let libraryV2Request = $state<PreviewImportRequest | null>(null);
  let libraryV2Title = $state("Library v2 导入预览");

  const steamConnectionLabel = $derived.by(() => {
    if (syncingSteam) return "同步中";
    if (steamIdInput.trim() && status?.steam_api_key_validated) return "可同步";
    if (steamIdInput.trim()) return "缺 API Key";
    return "未连接";
  });

  const steamConnectionTone = $derived.by(() => {
    if (steamConnectionLabel === "可同步") return "ok";
    if (steamConnectionLabel === "缺 API Key") return "warn";
    if (steamConnectionLabel === "同步中") return "busy";
    return "idle";
  });

  onMount(() => {
    libraryV2Enabled = readLibraryV2Flag();
    const cleanups: Array<() => void> = [];
    void (async () => {
      await loadInitialState();
      cleanups.push(await listen<{ status: string; message: string }>("moe://steam-progress", (event) => {
        steamLoginMessage = event.payload.message;
        const s = event.payload.status;
        if (s === "timeout" || s === "closed" || s === "scrape_timeout") {
          openingLogin = false;
          syncingSteam = false;
          if (s === "timeout") {
            steamAccountError = event.payload.message;
          }
        }
      }));
      cleanups.push(await listen<{ steam_id: string; profile_url: string }>("moe://steam-login", async (event) => {
        steamIdInput = event.payload.steam_id;
        steamLoginMessage = "登录成功，正在读取游戏库…";
        openingLogin = false;
        syncingSteam = true; // 紧接着会话抓取 → moe://steam-session-games 导入
        await saveSteamSettings({ steam_id: event.payload.steam_id });
      }));
      // Playnite 式：登录后用已认证会话抓到的全库（无需 API Key），直接导入
      cleanups.push(await listen<{ steam_id: string; games: SteamSessionGame[] }>("moe://steam-session-games", async (event) => {
        const games = event.payload.games ?? [];
        openingLogin = false;
        if (!games.length) {
          syncingSteam = false;
          steamLoginMessage = "未读取到游戏，请改用本机扫描或 API Key";
          return;
        }
        if (libraryV2Enabled) {
          steamLoginMessage = `已读取 ${games.length} 款 Steam 游戏，等待确认 diff`;
          libraryV2Title = "Steam 登录会话导入预览";
          libraryV2Request = steamSessionGamesToPreviewRequest(games);
          syncingSteam = false;
          return;
        }
        syncingSteam = true;
        steamLoginMessage = `正在导入 ${games.length} 款 Steam 游戏…`;
        try {
          const result = await importSteamSessionGames(games);
          steamAccountImport = result;
          await refreshLibrary();
          steamLoginMessage = `Steam 全库已导入：新增 ${result.imported}，更新 ${result.updated}`;
          uiStore.notify(`Steam 导入完成：新增 ${result.imported}，更新 ${result.updated}（共 ${result.total}）`, "success");
          autoSyncAchievementsQuietly();
        } catch (e) {
          steamAccountError = String(e);
          uiStore.notify("Steam 导入失败：" + String(e), "error");
        } finally {
          syncingSteam = false;
        }
      }));
    })();
    return () => cleanups.forEach((cleanup) => cleanup());
  });

  async function loadInitialState() {
    loadingStatus = true;
    statusError = "";
    try {
      await settingsStore.load();
      status = await getPlatformImportStatus();
      steamIdInput = settingsStore.settings.steam_id || status.steam_id || "";
      apiKeyInput = "";
      steamLoginMessage = steamIdInput ? "已保存 SteamID" : "未连接";
      apiKeyMessage = status.steam_api_key_validated ? "API Key 已验证" : (status.has_steam_api_key ? "已保存 API Key，等待验证" : "");
    } catch (e) {
      statusError = String(e);
    } finally {
      loadingStatus = false;
    }
  }

  async function saveSteamSettings(patch: { steam_id?: string }) {
    await settingsStore.save({
      ...settingsStore.settings,
      ...patch,
    });
  }

  function setLibraryV2Enabled(enabled: boolean) {
    libraryV2Enabled = enabled;
    if (!enabled) libraryV2Request = null;
  }

  function openLibraryV2Preview(source: string, candidates: PlatformGameCandidate[], title: string) {
    const available = uniqueCandidates(candidates);
    if (!available.length) {
      allImportError = "没有可供 Library v2 预览的候选。";
      return false;
    }
    libraryV2Title = title;
    libraryV2Request = platformCandidatesToPreviewRequest(source, available);
    return true;
  }

  async function handleLibraryV2Applied(result: ApplyImportResponse) {
    await refreshLibrary();
    const changed = result.results.filter((item) => ["created", "updated", "merged"].includes(item.status)).length;
    const failed = result.results.filter((item) => item.status === "failed" || item.status === "conflict").length;
    uiStore.notify(`Library v2 应用完成：写入 ${changed}，需处理 ${failed}`, failed ? "error" : "success");
  }

  function candidateKey(game: PlatformGameCandidate) {
    return `${game.source}:${game.library_id}`;
  }

  function selectAll(games: PlatformGameCandidate[]) {
    return new Set(games.filter((g) => !g.skip_reason).map(candidateKey));
  }

  function selectedCandidates(games: PlatformGameCandidate[], selected: Set<string>) {
    return games.filter((game) => selected.has(candidateKey(game)));
  }

  function uniqueCandidates(games: PlatformGameCandidate[]) {
    const byKey = new Map<string, PlatformGameCandidate>();
    for (const game of games) {
      if (game.skip_reason) continue;
      byKey.set(candidateKey(game), game);
    }
    return [...byKey.values()];
  }

  function mergeImportSummary(results: Array<{ label: string; result: PlatformImportResult }>): AggregateImportSummary {
    return results.reduce<AggregateImportSummary>(
      (summary, item) => ({
        imported: summary.imported + item.result.imported,
        updated: summary.updated + item.result.updated,
        skipped: summary.skipped + item.result.skipped,
        failed: summary.failed + item.result.failed,
        total: summary.total + item.result.total,
        sections: [...summary.sections, item.label],
        errors: [...summary.errors, ...item.result.errors],
      }),
      { imported: 0, updated: 0, skipped: 0, failed: 0, total: 0, sections: [], errors: [] },
    );
  }

  function steamAccountReady() {
    const steamId = steamIdInput.trim() || status?.steam_id || "";
    if (!steamId || !status?.steam_api_key_validated) return null;
    return { steamId };
  }

  function setSelected(section: SectionKey, next: Set<string>) {
    if (section === "steamAccount") steamAccountSelected = next;
    else if (section === "steamLocal") steamLocalSelected = next;
    else epicSelected = next;
  }

  function toggleCandidate(section: SectionKey, game: PlatformGameCandidate) {
    const current =
      section === "steamAccount" ? steamAccountSelected :
      section === "steamLocal" ? steamLocalSelected :
      epicSelected;
    const next = new Set(current);
    const key = candidateKey(game);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    setSelected(section, next);
  }

  function toggleAll(section: SectionKey, games: PlatformGameCandidate[]) {
    const current =
      section === "steamAccount" ? steamAccountSelected :
      section === "steamLocal" ? steamLocalSelected :
      epicSelected;
    const available = games.filter((g) => !g.skip_reason);
    setSelected(section, current.size === available.length ? new Set() : selectAll(available));
  }

  async function openApiKeyPage() {
    await openUrl("https://steamcommunity.com/dev/apikey");
  }

  async function detectLocalSteam() {
    detectingSteam = true;
    steamAccountError = "";
    try {
      const sid = await steamDetectLocal();
      if (!sid) {
        steamLoginMessage = "未检测到已登录的 Steam 客户端";
        steamAccountError = "请先启动 Steam 客户端并登录，或使用网页登录/手动输入 SteamID。";
        return "";
      }
      steamIdInput = sid;
      steamLoginMessage = "已检测到本地 Steam 账号";
      await saveSteamSettings({ steam_id: sid });
      status = await getPlatformImportStatus();
      return sid;
    } catch (e) {
      steamAccountError = String(e);
      return "";
    } finally {
      detectingSteam = false;
    }
  }

  async function verifyKey() {
    const key = apiKeyInput.trim();
    apiKeyMessage = "";
    steamAccountError = "";
    if (!key) {
      steamAccountError = "请输入 Steam Web API Key。";
      return false;
    }
    verifyingKey = true;
    try {
      apiKeyMessage = await validateSteamApiKey(key);
      apiKeyInput = "";
      status = await getPlatformImportStatus();
      return true;
    } catch (e) {
      apiKeyMessage = "";
      steamAccountError = String(e);
      return false;
    } finally {
      verifyingKey = false;
    }
  }

  async function deleteSteamKey() {
    verifyingKey = true;
    steamAccountError = "";
    try {
      await secretDelete("steam_api_key");
      apiKeyInput = "";
      apiKeyMessage = "API Key 已删除";
      status = await getPlatformImportStatus();
    } catch (e) {
      steamAccountError = String(e);
    } finally {
      verifyingKey = false;
    }
  }

  async function resolveSteamProfile() {
    const input = steamProfileInput.trim() || steamIdInput.trim();
    resolvingSteam = true;
    steamAccountError = "";
    if (!input) {
      steamAccountError = "请输入 SteamID64 或 Steam 个人主页 URL。";
      resolvingSteam = false;
      return "";
    }
    try {
      if (apiKeyInput.trim() && !status?.steam_api_key_validated) {
        const verified = await verifyKey();
        if (!verified) return "";
      }
      const result = await resolveSteamId(input);
      steamIdInput = result.steam_id;
      steamLoginMessage = result.personaname ? `已连接 ${result.personaname}` : "SteamID 已保存";
      await settingsStore.load();
      status = await getPlatformImportStatus();
      return result.steam_id;
    } catch (e) {
      steamAccountError = String(e);
      return "";
    } finally {
      resolvingSteam = false;
    }
  }

  async function openSteamLogin() {
    openingLogin = true;
    steamLoginMessage = "等待 Steam 网页/扫码登录";
    steamAccountError = "";
    try {
      await steamLoginOpenid();
    } catch (e) {
      openingLogin = false;
      steamAccountError = String(e);
    }
  }

  async function ensureSteamReady() {
    const keyOk = status?.steam_api_key_validated || await verifyKey();
    if (!keyOk) return null;

    let sid = steamIdInput.trim();
    if (!sid) sid = await detectLocalSteam();
    if (!sid) {
      await openSteamLogin();
      return null;
    }
    return { steamId: sid };
  }

  async function previewSteamAccount() {
    const ready = await ensureSteamReady();
    if (!ready) return;
    syncingSteam = true;
    steamAccountError = "";
    steamAccountImport = null;
    try {
      steamAccountScan = await scanPlatformLibrary("steam", "combined", ready.steamId);
      steamAccountSelected = selectAll(steamAccountScan.candidates);
      steamLoginMessage = `已获取 ${steamAccountScan.candidates.length} 款 Steam 游戏`;
    } catch (e) {
      steamAccountError = String(e);
    } finally {
      syncingSteam = false;
    }
  }

  async function syncSteamAccount(fromLogin = false) {
    const ready = await ensureSteamReady();
    if (!ready) return;
    syncingSteam = true;
    steamAccountError = "";
    steamAccountImport = null;
    try {
      const scan = await scanPlatformLibrary("steam", "combined", ready.steamId);
      const selected = selectAll(scan.candidates);
      steamAccountScan = scan;
      steamAccountSelected = selected;
      const candidates = selectedCandidates(scan.candidates, selected);
      if (libraryV2Enabled) {
        openLibraryV2Preview("steam", candidates, "Steam 全库导入预览");
        steamLoginMessage = `已生成 ${candidates.length} 项候选，等待确认 diff`;
        return;
      }
      steamAccountImport = await importPlatformLibrary("steam", candidates);
      await refreshLibrary();
      steamLoginMessage = fromLogin ? "登录成功，Steam 全库已同步" : "Steam 全库同步完成";
      uiStore.notify(`Steam 同步完成：新增 ${steamAccountImport.imported}，更新 ${steamAccountImport.updated}`, "success");
    } catch (e) {
      steamAccountError = String(e);
    } finally {
      syncingSteam = false;
    }
  }

  async function scanSteamLocal() {
    scanningSteamLocal = true;
    steamLocalError = "";
    steamLocalImport = null;
    try {
      steamLocalScan = await scanPlatformLibrary("steam", "local");
      steamLocalSelected = selectAll(steamLocalScan.candidates);
      if (steamLocalScan.candidates.length === 0) steamLocalError = "未发现本机已安装 Steam 游戏。";
    } catch (e) {
      steamLocalError = String(e);
    } finally {
      scanningSteamLocal = false;
    }
  }

  async function scanEpicLocal() {
    scanningEpic = true;
    epicError = "";
    epicImport = null;
    try {
      epicScan = await scanPlatformLibrary("epic", "local");
      epicSelected = selectAll(epicScan.candidates);
      if (epicScan.candidates.length === 0) epicError = "未发现 Epic 本机安装游戏。";
    } catch (e) {
      epicError = String(e);
    } finally {
      scanningEpic = false;
    }
  }

  async function importSelected(section: SectionKey) {
    const scan =
      section === "steamAccount" ? steamAccountScan :
      section === "steamLocal" ? steamLocalScan :
      epicScan;
    if (!scan) return;
    const selected =
      section === "steamAccount" ? steamAccountSelected :
      section === "steamLocal" ? steamLocalSelected :
      epicSelected;
    const candidates = selectedCandidates(scan.candidates, selected);
    if (!candidates.length) return;

    if (section === "steamLocal") importingSteamLocal = true;
    else if (section === "epicLocal") importingEpic = true;
    else syncingSteam = true;

    try {
      const source = section === "epicLocal" ? "epic" : "steam";
      if (libraryV2Enabled) {
        const label = section === "steamAccount" ? "Steam 全库" : section === "steamLocal" ? "Steam 本机目录" : "Epic 本机目录";
        openLibraryV2Preview(source, candidates, `${label}导入预览`);
        return;
      }
      const result = await importPlatformLibrary(source, candidates);
      if (section === "steamLocal") steamLocalImport = result;
      else if (section === "epicLocal") epicImport = result;
      else steamAccountImport = result;
      await refreshLibrary();
      uiStore.notify(`${source === "steam" ? "Steam" : "Epic"} 导入完成：新增 ${result.imported}，更新 ${result.updated}`, "success");
    } catch (e) {
      if (section === "steamLocal") steamLocalError = String(e);
      else if (section === "epicLocal") epicError = String(e);
      else steamAccountError = String(e);
    } finally {
      importingSteamLocal = false;
      importingEpic = false;
      syncingSteam = false;
    }
  }

  async function previewAllAvailable(forceRescan = false) {
    importingAll = true;
    allImportSummary = null;
    allImportError = "";
    try {
      const combined: PlatformGameCandidate[] = [];
      const ready = steamAccountReady();
      if (ready) {
        syncingSteam = true;
        const scan = forceRescan || !steamAccountScan
          ? await scanPlatformLibrary("steam", "combined", ready.steamId)
          : steamAccountScan;
        const candidates = uniqueCandidates(scan.candidates);
        steamAccountScan = scan;
        steamAccountSelected = new Set(candidates.map(candidateKey));
        combined.push(...candidates);
      } else {
        scanningSteamLocal = true;
        const scan = forceRescan || !steamLocalScan
          ? await scanPlatformLibrary("steam", "local")
          : steamLocalScan;
        const candidates = uniqueCandidates(scan.candidates);
        steamLocalScan = scan;
        steamLocalSelected = new Set(candidates.map(candidateKey));
        combined.push(...candidates);
      }

      scanningEpic = true;
      const scan = forceRescan || !epicScan
        ? await scanPlatformLibrary("epic", "local")
        : epicScan;
      const epicCandidates = uniqueCandidates(scan.candidates);
      epicScan = scan;
      epicSelected = new Set(epicCandidates.map(candidateKey));
      combined.push(...epicCandidates);

      if (!openLibraryV2Preview("platform_sync", combined, "一键平台同步预览")) return;
      uiStore.notify(`Library v2 已生成 ${uniqueCandidates(combined).length} 项 diff，尚未写入游戏库`);
    } catch (e) {
      allImportError = String(e);
    } finally {
      importingAll = false;
      syncingSteam = false;
      scanningSteamLocal = false;
      scanningEpic = false;
    }
  }

  async function importAllAvailable(forceRescan = false) {
    if (libraryV2Enabled) {
      await previewAllAvailable(forceRescan);
      return;
    }
    importingAll = true;
    allImportSummary = null;
    allImportError = "";
    steamAccountError = "";
    steamLocalError = "";
    epicError = "";

    try {
      const imported: Array<{ label: string; result: PlatformImportResult }> = [];
      const ready = steamAccountReady();

      if (ready) {
        syncingSteam = true;
        const scan = forceRescan || !steamAccountScan
          ? await scanPlatformLibrary("steam", "combined", ready.steamId)
          : steamAccountScan;
        const candidates = uniqueCandidates(scan.candidates);
        steamAccountScan = scan;
        steamAccountSelected = new Set(candidates.map(candidateKey));
        if (candidates.length) {
          steamAccountImport = await importPlatformLibrary("steam", candidates);
          imported.push({ label: "Steam 全库", result: steamAccountImport });
        }
      } else {
        scanningSteamLocal = true;
        const scan = forceRescan || !steamLocalScan
          ? await scanPlatformLibrary("steam", "local")
          : steamLocalScan;
        const candidates = uniqueCandidates(scan.candidates);
        steamLocalScan = scan;
        steamLocalSelected = new Set(candidates.map(candidateKey));
        if (candidates.length) {
          steamLocalImport = await importPlatformLibrary("steam", candidates);
          imported.push({ label: "Steam 本地", result: steamLocalImport });
        }
      }

      scanningEpic = true;
      const epicLocal = forceRescan || !epicScan
        ? await scanPlatformLibrary("epic", "local")
        : epicScan;
      const epicCandidates = uniqueCandidates(epicLocal.candidates);
      epicScan = epicLocal;
      epicSelected = new Set(epicCandidates.map(candidateKey));
      if (epicCandidates.length) {
        epicImport = await importPlatformLibrary("epic", epicCandidates);
        imported.push({ label: "Epic 本地", result: epicImport });
      }

      if (!imported.length) {
        allImportError = "未发现可导入的平台游戏。Steam 账号全库需要 SteamID64 与 Web API Key；本次已按要求跳过 Epic 账号全库，仅保留 Epic 本地清单。";
        return;
      }

      allImportSummary = mergeImportSummary(imported);
      await refreshLibrary();
      uiStore.notify(
        `平台同步完成：新增 ${allImportSummary.imported}，更新 ${allImportSummary.updated}，跳过 ${allImportSummary.skipped}`,
        "success",
      );
      autoSyncAchievementsQuietly();
    } catch (e) {
      allImportError = String(e);
    } finally {
      importingAll = false;
      syncingSteam = false;
      scanningSteamLocal = false;
      scanningEpic = false;
    }
  }

  async function refreshLibrary() {
    await gameStore.load();
    gameStore.searchQuery = "";
    gameStore.quickFilter = null;
    gameStore.filterTag = null;
  }

  function goLibrary() {
    gameStore.searchQuery = "";
    gameStore.quickFilter = null;
    gameStore.filterTag = null;
    uiStore.currentView = "home";
  }

  async function handleSyncAchievements() {
    syncingAchievements = true;
    achievementError = "";
    achievementResult = null;
    try {
      achievementResult = await syncSteamAchievements();
      await gameStore.load();
    } catch (e: any) {
      achievementError = e?.message ?? String(e);
    } finally {
      syncingAchievements = false;
    }
  }

  function autoSyncAchievementsQuietly() {
    const s = settingsStore.settings;
    if (!status?.steam_api_key_validated || !s.steam_id?.trim()) return;
    syncSteamAchievements()
      .then(r => {
        if (r.synced > 0) {
          uiStore.notify(`成就已同步 ${r.synced} 款游戏`, "success");
          gameStore.load();
        }
      })
      .catch(() => {});
  }

  function formatPlaytime(minutes: number | null | undefined) {
    if (!minutes) return "未记录";
    if (minutes < 60) return `${minutes} 分钟`;
    return `${Math.round(minutes / 6) / 10} 小时`;
  }
</script>

<section class="platform-page aura-page" data-aura-echo="IMPORT">
  <div class="backdrop" style={`background-image: url("${defaultLibraryBackdrop}")`}></div>

  <header class="page-head aura-head">
    <Button variant="ghost" class="back" press={goLibrary} ariaLabel="返回游戏库">
      <Icon name="arrowLeft" size={18} />
    </Button>
    <div class="title-block">
      <span class="aura-kicker">Platform Import</span>
      <h1 class="aura-title">平台导入</h1>
    </div>
    <Card class={`connection ${steamConnectionTone}`} padding="sm">
      <span>Steam</span>
      <strong>{steamConnectionLabel}</strong>
    </Card>
    <LibraryFeatureToggle enabled={libraryV2Enabled} onChange={setLibraryV2Enabled} />
  </header>

  <nav class="step-strip aura-panel" aria-label="导入步骤">
    <div class="step-item active">
      <span class="step-num">01</span>
      <strong>连接</strong>
      <small>读取 Steam / Epic 本机状态</small>
    </div>
    <div class="step-item">
      <span class="step-num">02</span>
      <strong>预览</strong>
      <small>勾选候选与去重结果</small>
    </div>
    <div class="step-item">
      <span class="step-num">03</span>
      <strong>导入</strong>
      <small>{libraryV2Enabled ? "确认后增量写入" : "旧流程直接写入"}</small>
    </div>
  </nav>

  <Card class="aggregate-bar" padding="md">
    <div>
      <strong>一键平台同步</strong>
      <span>{libraryV2Enabled ? "自动聚合候选并生成 create / update / conflict / ignore diff；确认前零写入。" : "旧流程：按平台 ID 去重后直接增量写入，可随时切回 Library v2。"}</span>
    </div>
    <div class="aggregate-actions">
      <Button variant="secondary" press={() => importAllAvailable(false)} disabled={importingAll || syncingSteam || scanningSteamLocal || scanningEpic}>
        <Icon name="download" size={16} />{importingAll ? "同步中" : (libraryV2Enabled ? "预览全部可用" : "导入全部可用")}
      </Button>
      <Button press={() => importAllAvailable(true)} disabled={importingAll || syncingSteam || scanningSteamLocal || scanningEpic}>
        <Icon name="refresh" size={16} />重新同步
      </Button>
    </div>
  </Card>

  {#if allImportError}
    <div class="banner error aggregate-banner">{allImportError}</div>
  {/if}
  {#if allImportSummary}
    <div class="banner ok aggregate-banner">
      {allImportSummary.sections.join(" / ")} 完成：共处理 {allImportSummary.total}，新增 {allImportSummary.imported}，更新 {allImportSummary.updated}，跳过 {allImportSummary.skipped}，失败 {allImportSummary.failed}
    </div>
  {/if}

  {#if libraryV2Enabled && libraryV2Request}
    <LibraryV2ImportPanel
      request={libraryV2Request}
      title={libraryV2Title}
      onApplied={handleLibraryV2Applied}
      onClose={() => (libraryV2Request = null)}
    />
  {/if}

  <Card class="aggregate-bar" padding="md">
    <div>
      <strong>Steam 成就同步</strong>
      <span>从 Steam Web API 拉取所有 Steam 游戏的成就数据（需要 API Key + SteamID）。</span>
    </div>
    <div class="aggregate-actions">
      <Button variant="secondary" press={handleSyncAchievements} disabled={syncingAchievements}>
        <Icon name="star" size={16} />{syncingAchievements ? "同步中..." : "同步成就数据"}
      </Button>
    </div>
  </Card>

  {#if achievementError}
    <div class="banner error aggregate-banner">{achievementError}</div>
  {/if}
  {#if achievementResult}
    <div class="banner ok aggregate-banner">
      成就同步完成：已同步 {achievementResult.synced}，跳过 {achievementResult.skipped}，失败 {achievementResult.failed}
    </div>
  {/if}

  {#if statusError}
    <div class="banner error">{statusError}</div>
  {/if}

  <div class="layout">
    <Card class="panel account-panel steam-card" padding="lg">
      <div class="panel-head">
        <div>
          <p class="eyebrow">Steam Account</p>
          <h2>账号全库同步</h2>
        </div>
        <span class="state">{steamLoginMessage}</span>
      </div>

      <div class="status-grid">
        <StatBlock label="本机 Steam" value={status?.steam_path || "未检测到"} />
        <StatBlock label="SteamID64" value={steamIdInput || "未连接"} />
        <StatBlock label="API Key" value={status?.steam_api_key_validated ? "已验证" : (apiKeyInput ? "待验证" : (status?.has_steam_api_key ? "已保存待验证" : "未填写"))} />
      </div>

      <div class="field-row">
        <label>
          <span>Steam Web API Key</span>
          <Input type="password" bind:value={apiKeyInput} autocomplete="off" placeholder="粘贴 Steam Web API Key" />
        </label>
        <div class="actions">
          <Button variant="secondary" press={openApiKeyPage}><Icon name="globe" size={16} />打开 Key 页面</Button>
          <Button variant="secondary" press={verifyKey} disabled={verifyingKey}><Icon name="check" size={16} />{verifyingKey ? "验证中" : "保存并验证"}</Button>
          {#if status?.has_steam_api_key}
            <Button variant="ghost" press={deleteSteamKey} disabled={verifyingKey}>删除 Key</Button>
          {/if}
        </div>
      </div>

      <div class="field-row">
        <label>
          <span>SteamID64 / 个人主页</span>
          <Input bind:value={steamProfileInput} placeholder="7656119... 或 https://steamcommunity.com/profiles/..." onkeydown={(e) => { if (e.key === "Enter") resolveSteamProfile(); }} />
        </label>
        <div class="actions">
          <Button variant="secondary" press={detectLocalSteam} disabled={detectingSteam}><Icon name="search" size={16} />{detectingSteam ? "检测中" : "检测本地"}</Button>
          <Button variant="secondary" press={openSteamLogin} disabled={openingLogin || syncingSteam}><Icon name="globe" size={16} />{openingLogin ? "等待登录" : "网页登录/扫码"}</Button>
          <Button variant="secondary" press={resolveSteamProfile} disabled={resolvingSteam}><Icon name="check" size={16} />{resolvingSteam ? "解析中" : "解析"}</Button>
        </div>
      </div>

      {#if apiKeyMessage}
        <div class="banner ok">{apiKeyMessage}</div>
      {/if}
      {#if steamAccountError}
        <div class="banner error">{steamAccountError}</div>
      {/if}

      <div class="primary-row">
        <Button variant="secondary" press={previewSteamAccount} disabled={syncingSteam || openingLogin}>
          <Icon name="search" size={16} />预览全库
        </Button>
        <Button press={() => syncSteamAccount(false)} disabled={syncingSteam || openingLogin}>
<Icon name="download" size={17} />{syncingSteam ? "同步中..." : (libraryV2Enabled ? "扫描并预览 Steam 全库" : "同步并导入 Steam 全库")}
        </Button>
      </div>

      {#if syncingSteam}
        <div class="progress"><span></span></div>
      {/if}

      {@render CandidateList({
        title: "Steam 全库候选",
        scan: steamAccountScan,
        selected: steamAccountSelected,
        importing: syncingSteam,
        result: steamAccountImport,
        section: "steamAccount",
        onToggle: toggleCandidate,
        onToggleAll: toggleAll,
        onImport: importSelected,
        showInstalled: true,
        previewMode: libraryV2Enabled,
      })}
    </Card>

    <Card class="panel steam-local-card" padding="lg">
      <div class="panel-head">
        <div>
          <p class="eyebrow">Steam Local</p>
          <h2>本机已安装</h2>
        </div>
        <Button variant="secondary" press={scanSteamLocal} disabled={scanningSteamLocal}>
          <Icon name="search" size={16} />{scanningSteamLocal ? "扫描中" : "扫描"}
        </Button>
      </div>
      {#if steamLocalError}<div class="banner error">{steamLocalError}</div>{/if}
      {@render CandidateList({
        title: "Steam 本机候选",
        scan: steamLocalScan,
        selected: steamLocalSelected,
        importing: importingSteamLocal,
        result: steamLocalImport,
        section: "steamLocal",
        onToggle: toggleCandidate,
        onToggleAll: toggleAll,
        onImport: importSelected,
        previewMode: libraryV2Enabled,
      })}
    </Card>

    <Card class="panel epic-card" padding="lg">
      <div class="panel-head">
        <div>
          <p class="eyebrow">Epic Local</p>
          <h2>本机已安装</h2>
        </div>
        <Button variant="secondary" press={scanEpicLocal} disabled={scanningEpic}>
          <Icon name="search" size={16} />{scanningEpic ? "扫描中" : "扫描"}
        </Button>
      </div>
      <div class="path-line">{status?.epic_manifest_path || "未检测到 Epic Launcher 本机清单目录"}</div>
      {#if epicError}<div class="banner error">{epicError}</div>{/if}
      {@render CandidateList({
        title: "Epic 本机候选",
        scan: epicScan,
        selected: epicSelected,
        importing: importingEpic,
        result: epicImport,
        section: "epicLocal",
        onToggle: toggleCandidate,
        onToggleAll: toggleAll,
        onImport: importSelected,
        previewMode: libraryV2Enabled,
      })}
    </Card>
  </div>

  {#if steamAccountImport || steamLocalImport || epicImport}
    <Card class="done-bar" padding="md">
      <span>游戏库已刷新</span>
      <Button press={goLibrary}><Icon name="collection" size={16} />查看游戏库</Button>
    </Card>
  {/if}

  {#if loadingStatus}
    <Card class="loading-cover" padding="sm"><Icon name="refresh" size={18} />读取平台状态...</Card>
  {/if}
</section>

{#snippet resultLine(result: PlatformImportResult)}
  <div class="result-line">
    <Tag>新增 {result.imported}</Tag>
    <Tag>更新 {result.updated}</Tag>
    <Tag>跳过 {result.skipped}</Tag>
    <Tag variant={result.failed > 0 ? "accent" : "neutral"}>失败 {result.failed}</Tag>
  </div>
  {#if result.errors.length}
    <div class="error-list">
      {#each result.errors.slice(0, 4) as error}
        <p>{error}</p>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet emptyState(scan: PlatformScanResult | null)}
  {#if scan}
    <EmptyState icon="folder" title="没有候选游戏" description={scan.skipped.length ? scan.skipped[0] : undefined} />
  {:else}
    <EmptyState icon="search" title="等待扫描" description="扫描后会在这里显示候选列表。" />
  {/if}
{/snippet}

{#snippet CandidateList(props: CandidateListProps)}
  <div class="candidate-box">
    <div class="candidate-head">
      <span>{props.title}</span>
      {#if props.scan?.candidates.length}
        <div class="candidate-actions">
          <Button variant="quiet" press={() => props.onToggleAll(props.section, props.scan!.candidates)}>
            {props.selected.size === props.scan.candidates.length ? "取消全选" : "全选"}
          </Button>
          <Button press={() => props.onImport(props.section)} disabled={props.importing || props.selected.size === 0}>
            <Icon name="download" size={15} />{props.importing ? "处理中" : `${props.previewMode ? "预览" : "导入"}选中 ${props.selected.size}`}
          </Button>
        </div>
      {/if}
    </div>

    {#if props.scan?.candidates.length}
      <div class="candidate-list">
        {#each props.scan.candidates as game (candidateKey(game))}
          <label class="candidate-row" class:muted={!!game.skip_reason}>
            <input
              type="checkbox"
              checked={props.selected.has(candidateKey(game))}
              disabled={!!game.skip_reason}
              onchange={() => props.onToggle(props.section, game)}
            />
            <span class="candidate-main">
              <strong>{game.name}</strong>
              <small>{game.install_dir || game.launch_uri}</small>
            </span>
            {#if props.showInstalled}
              <Tag variant={game.installed ? "accent" : "neutral"}>{game.installed ? "已安装" : "账号库"}</Tag>
            {/if}
            <code>{game.library_id}</code>
            <span class="playtime">{formatPlaytime(game.playtime_minutes)}</span>
          </label>
        {/each}
      </div>
    {:else}
      {@render emptyState(props.scan)}
    {/if}

    {#if props.result}
      {@render resultLine(props.result)}
    {/if}
  </div>
{/snippet}

<style>
  .platform-page {
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: hidden;
    background: var(--bg-void);
    color: var(--text-primary);
    display: flex;
    flex-direction: column;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: 0;
    background-size: cover;
    background-position: center;
    display: none;
    opacity: 0;
  }
  .platform-page::after {
    content: attr(data-aura-echo);
    position: absolute;
    z-index: 0;
    right: clamp(18px, 4vw, 56px);
    bottom: clamp(16px, 3vw, 42px);
    color: var(--aura-echo);
    font-family: var(--font-display);
    font-size: clamp(44px, 10vw, 132px);
    font-weight: 800;
    line-height: 0.85;
    letter-spacing: 0;
    text-transform: uppercase;
    pointer-events: none;
  }
  .page-head,
  .step-strip,
  :global(.ui-card.aggregate-bar),
  .aggregate-banner,
  .layout,
  :global(.ui-card.done-bar),
  :global(.ui-card.loading-cover) {
    position: relative;
    z-index: 1;
  }
  .page-head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 24px 28px 0;
    padding: 18px 20px;
  }
  :global(.ui-button.back.back) {
    width: 38px;
    height: 38px;
    padding: 0;
  }
  .title-block {
    min-width: 0;
  }
  .title-block span,
  .aura-kicker,
  .eyebrow {
    display: block;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent);
    letter-spacing: 0;
    text-transform: uppercase;
    margin-bottom: 5px;
  }
  h1,
  h2 {
    margin: 0;
    color: #fff;
    letter-spacing: 0;
  }
  h1 { font-size: 28px; }
  h2 { font-size: 18px; }
  :global(.ui-card.connection) {
    margin-left: auto;
    min-width: 128px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    align-items: flex-start;
  }
  :global(.ui-card.connection) span {
    color: rgba(255,255,255,0.52);
    font-size: 11px;
  }
  :global(.ui-card.connection) strong {
    color: #fff;
    font-size: 13px;
  }
  :global(.ui-card.connection.ok) strong { color: var(--color-success); }
  :global(.ui-card.connection.warn) strong { color: #fbbf24; }
  :global(.ui-card.connection.busy) strong { color: #93c5fd; }

  .step-strip {
    margin: 14px 28px 0;
    padding: 10px;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }
  .step-item {
    min-width: 0;
    min-height: 58px;
    border: 1px solid var(--aura-border);
    border-radius: 8px;
    padding: 10px 12px;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 2px 10px;
    align-items: center;
    background: var(--aura-inset);
  }
  .step-item.active {
    border-color: rgba(232,85,127,0.34);
    background: rgba(232,85,127,0.1);
  }
  .step-num {
    grid-row: span 2;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
  }
  .step-item strong,
  .step-item small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .step-item strong {
    color: #fff;
    font-size: 13px;
  }
  .step-item small {
    color: rgba(255,255,255,0.56);
    font-size: 11px;
  }

  :global(.ui-card.aggregate-bar) {
    margin: 14px 28px 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  :global(.ui-card.aggregate-bar) strong {
    display: block;
    color: #fff;
    font-size: 14px;
    margin-bottom: 4px;
  }
  :global(.ui-card.aggregate-bar) span {
    display: block;
    color: rgba(255,255,255,0.58);
    font-size: 12px;
    line-height: 1.5;
  }
  .aggregate-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .aggregate-banner {
    position: relative;
    z-index: 1;
    margin: 12px 28px 0;
  }

  .layout {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: minmax(520px, 1.4fr) minmax(360px, 1fr);
    grid-auto-rows: min-content;
    gap: 18px;
    padding: 20px 28px 94px;
  }
  :global(.ui-card.panel) {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  :global(.ui-card.account-panel) {
    grid-row: span 2;
  }
  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .state,
  .path-line {
    color: rgba(255,255,255,0.58);
    font-size: 12px;
  }
  .status-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  label span {
    display: block;
    color: rgba(255,255,255,0.52);
    font-size: 11px;
    font-weight: 700;
    margin-bottom: 7px;
  }
  .field-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 10px;
    align-items: end;
  }
  label {
    min-width: 0;
  }
  .candidate-row input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
    cursor: pointer;
  }
  .actions,
  .primary-row,
  .candidate-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .primary-row {
    justify-content: flex-start;
  }
  .banner {
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 12px;
    line-height: 1.5;
    border: 1px solid var(--border);
    background: var(--bg-deep);
  }
  .banner.error {
    color: #fecaca;
    border-color: rgba(248,113,113,0.32);
    background: rgba(127,29,29,0.2);
  }
  .banner.ok {
    color: #bbf7d0;
    border-color: rgba(34,197,94,0.3);
    background: rgba(22,101,52,0.18);
  }
  .progress {
    height: 6px;
    border-radius: 3px;
    background: rgba(255,255,255,0.08);
    overflow: hidden;
  }
  .progress span {
    display: block;
    width: 38%;
    height: 100%;
    background: linear-gradient(90deg, var(--aura-data-a), var(--aura-data-b));
    animation: slide 1.1s ease-in-out infinite alternate;
  }
  @keyframes slide {
    from { transform: translateX(-20%); }
    to { transform: translateX(190%); }
  }
  .candidate-box {
    min-width: 0;
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }
  .candidate-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 10px;
  }
  .candidate-head > span {
    font-weight: 700;
    color: #fff;
  }
  .candidate-list {
    max-height: 360px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding-right: 4px;
  }
  .candidate-row {
    min-height: 56px;
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto auto auto;
    gap: 10px;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-deep);
    padding: 10px;
  }
  .candidate-row.muted {
    opacity: 0.58;
  }
  .candidate-main {
    min-width: 0;
  }
  .candidate-main strong,
  .candidate-main small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .candidate-main strong {
    color: #fff;
    font-size: 13px;
  }
  .candidate-main small {
    color: rgba(255,255,255,0.52);
    font-size: 11px;
    margin-top: 3px;
  }
  code,
  .playtime {
    font-family: var(--font-mono);
    font-size: 11px;
    color: rgba(255,255,255,0.58);
  }
  .result-line {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 10px;
  }
  .error-list {
    margin-top: 8px;
    color: #fecaca;
    font-size: 12px;
  }
  .error-list p {
    margin: 3px 0;
  }
  :global(.ui-card.done-bar) {
    position: absolute;
    left: 28px;
    right: 28px;
    bottom: 18px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  :global(.ui-card.done-bar) span {
    color: rgba(255,255,255,0.74);
    font-weight: 700;
  }
  :global(.ui-card.loading-cover) {
    position: absolute;
    right: 28px;
    top: 92px;
    display: flex;
    gap: 8px;
    align-items: center;
    color: rgba(255,255,255,0.74);
    font-size: 12px;
  }
  @media (max-width: 1180px) {
    .layout {
      grid-template-columns: 1fr;
    }
    :global(.ui-card.account-panel) {
      grid-row: auto;
    }
  }
  @media (max-width: 760px) {
    .page-head,
    .step-strip,
    :global(.ui-card.aggregate-bar),
    .aggregate-banner,
    .layout {
      margin-left: 16px;
      margin-right: 16px;
    }
    .page-head,
    .layout {
      padding-left: 16px;
      padding-right: 16px;
    }
    .step-strip {
      grid-template-columns: 1fr;
    }
    .field-row,
    .status-grid {
      grid-template-columns: 1fr;
    }
    .candidate-row {
      grid-template-columns: 20px minmax(0, 1fr);
    }
    code,
    .playtime {
      display: none;
    }
  }
</style>
