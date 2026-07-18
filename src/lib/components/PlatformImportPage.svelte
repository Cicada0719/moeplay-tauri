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
  import { i18n } from "../stores/i18n.svelte";
  import { settingsStore } from "../stores/settings.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import Icon from "./Icon.svelte";
  import { Button, Card, EmptyState, Input, StatBlock, Tag } from "./ui";
  import { PageShell, PageHeader, StateBoundary, type ViewState } from "./ui-v2";
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

  // 三态统一：初始平台状态的加载 / 失败收敛到 StateBoundary；分区操作反馈仍走各自 banner。
  const statusViewState = $derived<ViewState>(
    loadingStatus && !status ? "loading" : statusError ? "error" : "ready",
  );

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

<PageShell as="div" width="full" scrollable={false} class="platform-import-v2-shell" labelledBy="platform-import-page-title" ariaLabel={i18n.t("platform_import.title")}>
  <div class="platform-page">
    <div class="v2-grain pi-grain" aria-hidden="true"></div>

    <PageHeader
      id="platform-import-page-title"
      class="pi-header"
      eyebrow="インポート / IMPORT"
      title={i18n.t("platform_import.title")}
      description={i18n.t("platform_import.subtitle")}
    >
      {#snippet actions()}
        <div class="pi-header-actions">
          <Button variant="ghost" class="back" press={goLibrary} ariaLabel={i18n.t("platform_import.back_aria")}>
            <Icon name="arrowLeft" size={18} />
          </Button>
          <Card class={`connection ${steamConnectionTone}`} padding="sm">
            <span>Steam</span>
            <strong>{steamConnectionLabel}</strong>
          </Card>
          <LibraryFeatureToggle enabled={libraryV2Enabled} onChange={setLibraryV2Enabled} />
        </div>
      {/snippet}
    </PageHeader>

    <nav class="step-strip" aria-label={i18n.t("platform_import.steps_aria")}>
      <div class="step-item active">
        <span class="step-num">01</span>
        <strong>{i18n.t("platform_import.step_connect")}</strong>
        <small>{i18n.t("platform_import.step_connect_hint")}</small>
      </div>
      <div class="step-item">
        <span class="step-num">02</span>
        <strong>{i18n.t("platform_import.step_preview")}</strong>
        <small>{i18n.t("platform_import.step_preview_hint")}</small>
      </div>
      <div class="step-item">
        <span class="step-num">03</span>
        <strong>{i18n.t("platform_import.step_import")}</strong>
        <small>{libraryV2Enabled ? i18n.t("platform_import.step_import_hint_v2") : i18n.t("platform_import.step_import_hint_legacy")}</small>
      </div>
    </nav>

    <Card class="aggregate-bar" padding="md">
      <div>
        <strong>{i18n.t("platform_import.aggregate_title")}</strong>
        <span>{libraryV2Enabled ? i18n.t("platform_import.aggregate_desc_v2") : i18n.t("platform_import.aggregate_desc_legacy")}</span>
      </div>
      <div class="aggregate-actions">
        <Button variant="secondary" press={() => importAllAvailable(false)} disabled={importingAll || syncingSteam || scanningSteamLocal || scanningEpic}>
          <Icon name="download" size={16} />{importingAll ? i18n.t("platform_import.syncing") : (libraryV2Enabled ? i18n.t("platform_import.preview_all") : i18n.t("platform_import.import_all"))}
        </Button>
        <Button press={() => importAllAvailable(true)} disabled={importingAll || syncingSteam || scanningSteamLocal || scanningEpic}>
          <Icon name="refresh" size={16} />{i18n.t("platform_import.resync")}
        </Button>
      </div>
    </Card>

    {#if allImportError}
      <div class="banner error aggregate-banner">{allImportError}</div>
    {/if}
    {#if allImportSummary}
      <div class="banner ok aggregate-banner">
        {i18n.t("platform_import.aggregate_summary", { sections: allImportSummary.sections.join(" / "), total: allImportSummary.total, imported: allImportSummary.imported, updated: allImportSummary.updated, skipped: allImportSummary.skipped, failed: allImportSummary.failed })}
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

    <StateBoundary
      class="pi-boundary"
      state={statusViewState}
      onRetry={loadInitialState}
      retryLabel={i18n.t("button.retry")}
      title={i18n.t("platform_import.error_title")}
      description={statusError || undefined}
      loadingRows={4}
    >
      <div class="layout">
        <Card class="panel account-panel steam-card" padding="lg">
          <div class="panel-head">
            <div>
              <p class="eyebrow">Steam Account</p>
              <h2>{i18n.t("platform_import.steam_account_title")}</h2>
            </div>
            <span class="state">{steamLoginMessage}</span>
          </div>

          <div class="status-grid">
            <StatBlock label={i18n.t("platform_import.stat_local_steam")} value={status?.steam_path || i18n.t("platform_import.stat_not_detected")} />
            <StatBlock label="SteamID64" value={steamIdInput || i18n.t("platform_import.stat_not_connected")} />
            <StatBlock label="API Key" value={status?.steam_api_key_validated ? i18n.t("platform_import.stat_key_validated") : (apiKeyInput ? i18n.t("platform_import.stat_key_pending") : (status?.has_steam_api_key ? i18n.t("platform_import.stat_key_saved") : i18n.t("platform_import.stat_key_empty")))} />
          </div>

          <div class="field-row">
            <label>
              <span>Steam Web API Key</span>
              <Input type="password" bind:value={apiKeyInput} autocomplete="off" placeholder={i18n.t("platform_import.apikey_placeholder")} />
            </label>
            <div class="actions">
              <Button variant="secondary" press={openApiKeyPage}><Icon name="globe" size={16} />{i18n.t("platform_import.apikey_open")}</Button>
              <Button variant="secondary" press={verifyKey} disabled={verifyingKey}><Icon name="check" size={16} />{verifyingKey ? i18n.t("platform_import.apikey_verifying") : i18n.t("platform_import.apikey_verify")}</Button>
              {#if status?.has_steam_api_key}
                <Button variant="ghost" press={deleteSteamKey} disabled={verifyingKey}>{i18n.t("platform_import.apikey_delete")}</Button>
              {/if}
            </div>
          </div>

          <div class="field-row">
            <label>
              <span>{i18n.t("platform_import.steamid_label")}</span>
              <Input bind:value={steamProfileInput} placeholder="7656119... 或 https://steamcommunity.com/profiles/..." onkeydown={(e) => { if (e.key === "Enter") resolveSteamProfile(); }} />
            </label>
            <div class="actions">
              <Button variant="secondary" press={detectLocalSteam} disabled={detectingSteam}><Icon name="search" size={16} />{detectingSteam ? i18n.t("platform_import.detecting") : i18n.t("platform_import.detect_local")}</Button>
              <Button variant="secondary" press={openSteamLogin} disabled={openingLogin || syncingSteam}><Icon name="globe" size={16} />{openingLogin ? i18n.t("platform_import.login_waiting") : i18n.t("platform_import.login_web")}</Button>
              <Button variant="secondary" press={resolveSteamProfile} disabled={resolvingSteam}><Icon name="check" size={16} />{resolvingSteam ? i18n.t("platform_import.resolving") : i18n.t("platform_import.resolve")}</Button>
            </div>
          </div>

          {#if apiKeyMessage}
            <div class="banner ok">{apiKeyMessage}</div>
          {/if}
          {#if steamAccountError}
            <div class="banner error">{steamAccountError}</div>
          {/if}

          <div class="field-row achievement-sync-row">
            <div class="achievement-copy">
              <span>{i18n.t("platform_import.achievement_title")}</span>
              <small>{i18n.t("platform_import.achievement_desc")}</small>
            </div>
            <div class="actions">
              <Button variant="secondary" press={handleSyncAchievements} disabled={syncingAchievements}>
                <Icon name="star" size={16} />{syncingAchievements ? i18n.t("platform_import.achievement_syncing") : i18n.t("platform_import.achievement_sync")}
              </Button>
            </div>
          </div>

          {#if achievementError}
            <div class="banner error">{achievementError}</div>
          {/if}
          {#if achievementResult}
            <div class="banner ok">
              {i18n.t("platform_import.achievement_result", { synced: achievementResult.synced, skipped: achievementResult.skipped, failed: achievementResult.failed })}
            </div>
          {/if}

          <div class="primary-row">
            <Button variant="secondary" press={previewSteamAccount} disabled={syncingSteam || openingLogin}>
              <Icon name="search" size={16} />{i18n.t("platform_import.preview_library")}
            </Button>
            <Button press={() => syncSteamAccount(false)} disabled={syncingSteam || openingLogin}>
              <Icon name="download" size={17} />{syncingSteam ? i18n.t("platform_import.achievement_syncing") : (libraryV2Enabled ? i18n.t("platform_import.scan_preview_library") : i18n.t("platform_import.sync_import_library"))}
            </Button>
          </div>

          {#if syncingSteam}
            <div class="progress"><span></span></div>
          {/if}

          {@render CandidateList({
            title: i18n.t("platform_import.candidates_steam_account"),
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
              <h2>{i18n.t("platform_import.local_installed")}</h2>
            </div>
            <Button variant="secondary" press={scanSteamLocal} disabled={scanningSteamLocal}>
              <Icon name="search" size={16} />{scanningSteamLocal ? i18n.t("platform_import.scanning") : i18n.t("platform_import.scan")}
            </Button>
          </div>
          {#if steamLocalError}<div class="banner error">{steamLocalError}</div>{/if}
          {@render CandidateList({
            title: i18n.t("platform_import.candidates_steam_local"),
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
              <h2>{i18n.t("platform_import.local_installed")}</h2>
            </div>
            <Button variant="secondary" press={scanEpicLocal} disabled={scanningEpic}>
              <Icon name="search" size={16} />{scanningEpic ? i18n.t("platform_import.scanning") : i18n.t("platform_import.scan")}
            </Button>
          </div>
          <div class="path-line">{status?.epic_manifest_path || i18n.t("platform_import.epic_path_missing")}</div>
          {#if epicError}<div class="banner error">{epicError}</div>{/if}
          {@render CandidateList({
            title: i18n.t("platform_import.candidates_epic"),
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
          <span>{i18n.t("platform_import.done_refreshed")}</span>
          <Button press={goLibrary}><Icon name="collection" size={16} />{i18n.t("platform_import.view_library")}</Button>
        </Card>
      {/if}
    </StateBoundary>
  </div>
</PageShell>

{#snippet resultLine(result: PlatformImportResult)}
  <div class="result-line">
    <Tag>{i18n.t("platform_import.result_added", { count: result.imported })}</Tag>
    <Tag>{i18n.t("platform_import.result_updated", { count: result.updated })}</Tag>
    <Tag>{i18n.t("platform_import.result_skipped", { count: result.skipped })}</Tag>
    <Tag variant={result.failed > 0 ? "accent" : "neutral"}>{i18n.t("platform_import.result_failed", { count: result.failed })}</Tag>
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
    <EmptyState icon="folder" title={i18n.t("platform_import.empty_none_title")} description={scan.skipped.length ? scan.skipped[0] : undefined} />
  {:else}
    <EmptyState icon="search" title={i18n.t("platform_import.empty_waiting_title")} description={i18n.t("platform_import.empty_waiting_desc")} />
  {/if}
{/snippet}

{#snippet CandidateList(props: CandidateListProps)}
  <div class="candidate-box">
    <div class="candidate-head">
      <span>{props.title}</span>
      {#if props.scan?.candidates.length}
        <div class="candidate-actions">
          <Button variant="quiet" press={() => props.onToggleAll(props.section, props.scan!.candidates)}>
            {props.selected.size === props.scan.candidates.length ? i18n.t("platform_import.deselect_all") : i18n.t("platform_import.select_all")}
          </Button>
          <Button press={() => props.onImport(props.section)} disabled={props.importing || props.selected.size === 0}>
            <Icon name="download" size={15} />{props.importing ? i18n.t("platform_import.processing") : i18n.t(props.previewMode ? "platform_import.preview_selected" : "platform_import.import_selected", { count: props.selected.size })}
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
              <Tag variant={game.installed ? "accent" : "neutral"}>{game.installed ? i18n.t("platform_import.tag_installed") : i18n.t("platform_import.tag_account")}</Tag>
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
  :global(.platform-import-v2-shell) { height: 100%; }
  :global(.platform-import-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .platform-page {
    position: relative;
    height: 100%;
    overflow: hidden;
    background: var(--bg-void);
    color: var(--text-primary);
    display: flex;
    flex-direction: column;
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .pi-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.pi-header) {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    padding: 22px 28px 12px;
  }
  .pi-header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  :global(.ui-button.back.back) {
    width: 38px;
    height: 38px;
    padding: 0;
  }
  .eyebrow {
    display: block;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent);
    letter-spacing: 0;
    text-transform: uppercase;
    margin-bottom: 5px;
  }
  h2 {
    margin: 0;
    color: #fff;
    letter-spacing: 0;
    font-size: 18px;
  }
  :global(.ui-card.connection) {
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

  .step-strip,
  :global(.ui-card.aggregate-bar),
  .aggregate-banner,
  :global(.pi-boundary),
  .layout,
  :global(.ui-card.done-bar) {
    position: relative;
    z-index: 1;
  }
  .step-strip {
    margin: 14px 28px 0;
    padding: 10px;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-deep);
  }
  .step-item {
    min-width: 0;
    min-height: 58px;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 2px 10px;
    align-items: center;
    background: var(--bg-void);
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

  :global(.pi-boundary) {
    flex: 1;
    min-height: 0;
    margin: 20px 28px;
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
  .achievement-sync-row {
    align-items: center;
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }
  .achievement-copy span {
    display: block;
    color: rgba(255,255,255,0.52);
    font-size: 11px;
    font-weight: 700;
    margin-bottom: 7px;
  }
  .achievement-copy small {
    color: rgba(255,255,255,0.58);
    font-size: 12px;
    line-height: 1.5;
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
    background: linear-gradient(90deg, var(--accent), rgba(232, 85, 127, 0.62));
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
  @media (max-width: 1180px) {
    .layout {
      grid-template-columns: 1fr;
    }
    :global(.ui-card.account-panel) {
      grid-row: auto;
    }
  }
  @media (max-width: 760px) {
    .step-strip,
    :global(.ui-card.aggregate-bar),
    .aggregate-banner,
    :global(.pi-boundary),
    .layout {
      margin-left: 16px;
      margin-right: 16px;
    }
    :global(.pi-header),
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

  @media (prefers-reduced-motion: reduce) {
    .progress span { animation: none; }
  }
  :global([data-motion="reduce"]) .progress span { animation: none; }
</style>
