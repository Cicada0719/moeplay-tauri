<script lang="ts">
  // 掌机模式（手持主机壳）：PSP/XMB 风格频道 + 内容选择器。
  // 全内容整合：「继续」聚合 / 番剧 / 漫画 / 小说 / 游戏平台共用一套掌机首页。
  // 手柄优先（左右切频道 / 上下选内容 / A 打开 / X 收藏 / LB·RB 快选），触屏与键盘同效。
  // 视觉全面接入 v5 设计令牌（--bg-* / --text-* / --accent* / --glass-*），随主题包联动。
  import { onDestroy, onMount } from "svelte";
  import { gameStore } from "../../stores/games.svelte";
  import type { Game } from "../../stores/games.svelte";
  import { animeStore } from "../../stores/anime.svelte";
  import { comicStore } from "../../stores/comic.svelte";
  import { novelStore } from "../novel/store.svelte";
  import { collectionStore } from "../anime-home/collection.svelte";
  import { historyStore } from "../anime-player/historyStore.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { closeOverlay, navigateTo, openOverlay } from "../../stores/router.svelte";
  import { platformStore } from "../../platform/runtime.svelte";
  import { orientationStore } from "../../platform/android/orientation.svelte";
  import { fileSrc } from "../../utils";
  import { coverOf, gameLastPlayed, gameTotalSeconds, heroImageOf } from "../../utils/game";
  import { formatPlayTime } from "../../api";
  import { setHandheldSystemBars } from "./api";
  import { readHandheldImmersivePreference, onHandheldPrefsChanged } from "../../platform/handheld";
  import {
    groupGamesBySystem,
    buildHandheldChannels,
    buildHandheldGameSystems,
    handheldViewKey,
    migrateHandheldMemory,
    wrappedIndex,
    mergeAnimeItems,
    mergeComicItems,
    buildNovelItems,
    buildHandheldQuickNavGroups,
    sortRecentItems,
    type HandheldChannelId,
    type HandheldMemoryV3,
    type HandheldUnifiedItem,
  } from "./systems";
  import { attachGamepad, type GamepadAttachment } from "../../components/switch/useGamepad.svelte";
  import { openUnifiedMediaHistory } from "../media-history/open";
  import Icon from "../../components/Icon.svelte";
  import HandheldRail, { type HandheldRailItem } from "./HandheldRail.svelte";
  import HandheldGameWheel from "./HandheldGameWheel.svelte";
  import HandheldStatePanel from "./HandheldStatePanel.svelte";
  import HandheldArtworkStage from "./HandheldArtworkStage.svelte";
  import homeAmbient from "../../assets/handheld/home-ambient.webp";

  const MEMORY_KEY = "moeplay-handheld-memory-v3";

  /* ---- 数据：游戏系统 + 三类媒体条目 + 「继续」聚合 ---- */

  const gameSystems = $derived(groupGamesBySystem(gameStore.allGames));

  const animeItems = $derived(mergeAnimeItems(animeStore.history, animeStore.collection));
  const comicItems = $derived(mergeComicItems(comicStore.readHistory, comicStore.favorites));
  const novelItems = $derived(
    buildNovelItems(
      novelStore.history.map((e) => ({
        key: e.key,
        title: e.book.title,
        coverUrl: e.book.coverUrl,
        chapterTitle: e.chapterTitle,
        progress: e.progress,
        updatedAt: e.updatedAt,
      })),
    ),
  );

  /** 「继续」：跨类别最近使用（游戏按最后游玩、媒体按历史时间）。 */
  const recentItems = $derived.by(() => {
    const entries: HandheldUnifiedItem[] = [];
    for (const g of gameStore.allGames) {
      if (g.hidden) continue;
      const raw = gameLastPlayed(g);
      const time = raw ? new Date(raw).getTime() : 0;
      if (!time || Number.isNaN(time)) continue;
      entries.push({
        kind: "game",
        key: g.id,
        title: g.name,
        cover: fileSrc(coverOf(g)) ?? "",
        subtitle: "",
        time,
        fromHistory: true,
      });
    }
    for (const h of animeStore.history.slice(0, 8)) {
      entries.push({
        kind: "anime",
        key: h.key || h.name,
        title: h.name,
        cover: animeStore.getImg(h.image) || h.image,
        subtitle: h.lastEpisodeName ?? "",
        time: Date.parse(h.updatedAt) || 0,
        fromHistory: true,
      });
    }
    for (const r of comicStore.readHistory.slice(0, 8)) {
      entries.push({
        kind: "comic",
        key: r.id,
        title: r.title,
        cover: r.thumb_url ?? "",
        subtitle: r.last_title ?? "",
        time: r.ts ?? 0,
        fromHistory: true,
      });
    }
    for (const n of novelStore.history.slice(0, 8)) {
      entries.push({
        kind: "novel",
        key: n.key,
        title: n.book.title,
        cover: n.book.coverUrl ?? "",
        subtitle: n.chapterTitle ?? "",
        time: n.updatedAt ?? 0,
        fromHistory: true,
      });
    }
    return sortRecentItems(entries, 16);
  });

  const gameCount = $derived(gameSystems.reduce((total, system) => total + system.games.length, 0));
  const sections = $derived(
    buildHandheldChannels({
      recentCount: recentItems.length,
      animeCount: animeItems.length,
      comicCount: comicItems.length,
      novelCount: novelItems.length,
      gameCount,
    }),
  );
  const gameSystemOptions = $derived(
    buildHandheldGameSystems(gameSystems.map((s) => ({ id: s.id, label: s.label, count: s.games.length }))),
  );

  /* ---- 焦点模型：分区索引 + 每分区焦点位置记忆 ---- */

  let sectionIdx = $state(0);
  let focusMap = $state<Record<string, number>>({});
  let gameSystemIdx = $state(0);
  let padConnected = $state(false);
  let launching = $state<string | null>(null);
  let now = $state(new Date());
  let battery = $state<{ level: number; charging: boolean } | null>(null);
  let quickMenuOpen = $state(false);
  let quickMenuIndex = $state(0);
  let welcomeActionIndex = $state(0);
  let categoryRailVisible = $state(true);
  let categoryRailHideTimer: number | undefined;

  const QUICK_MENU_OVERLAY_ID = "handheld-quick-menu";
  const quickNavGroups = buildHandheldQuickNavGroups();
  const quickNavItems = quickNavGroups.flatMap((group) => group.items);
  const welcomeActions = [
    { id: "import", label: "模拟器导入", description: "扫描 ROM 与掌机档案", icon: "database", run: () => navigateTo("handheld-import") },
    { id: "games", label: "游戏库", description: "打开全部游戏", icon: "gamepad", run: () => navigateTo("game-library") },
    { id: "anime", label: "番剧", description: "搜索并开始播放", icon: "tv", run: () => navigateTo("anime") },
    { id: "comic", label: "漫画", description: "浏览来源与书架", icon: "image", run: () => navigateTo("comic") },
    { id: "novel", label: "小说", description: "搜索并开始阅读", icon: "book", run: () => navigateTo("novel") },
  ] as const;

  const section = $derived(sections[Math.min(sectionIdx, Math.max(sections.length - 1, 0))] ?? null);
  const gameSystemId = $derived(gameSystemOptions[Math.min(gameSystemIdx, Math.max(gameSystemOptions.length - 1, 0))]?.id ?? "all");
  const focusViewKey = $derived(handheldViewKey(section?.id ?? "recent", gameSystemId));
  const focusIdx = $derived(Math.max(focusMap[focusViewKey] ?? 0, 0));
  const hasAnyContent = $derived(
    recentItems.length > 0
      || animeItems.length > 0
      || comicItems.length > 0
      || novelItems.length > 0
      || gameSystems.some((system) => system.games.length > 0),
  );

  type Cell = { type: "game"; game: Game } | { type: "media"; item: HandheldUnifiedItem };

  const cells = $derived.by<Cell[]>(() => {
    if (!section) return [];
    if (section.kind === "games") {
      const games = gameSystemId === "all"
        ? gameSystems.flatMap((system) => system.games)
        : gameSystems.find((s) => s.id === gameSystemId)?.games ?? [];
      return games.map((g) => ({ type: "game", game: g }));
    }
    const list =
      section.id === "recent" ? recentItems
      : section.id === "anime" ? animeItems
      : section.id === "comic" ? comicItems
      : section.id === "novel" ? novelItems
      : [];
    return list.map((item) => ({ type: "media", item }));
  });

  const focusCell = $derived(cells[Math.min(focusIdx, Math.max(cells.length - 1, 0))] ?? null);
  const gameWheelGames = $derived(
    cells.filter((cell): cell is { type: "game"; game: Game } => cell.type === "game").map((cell) => cell.game),
  );

  /** 背景铺底：聚焦条目的封面（番剧走本地代理缓存，其余直用）。 */
  const backdropSrc = $derived.by(() => {
    const cell = focusCell;
    if (!cell) return "";
    if (cell.type === "game") return fileSrc(heroImageOf(cell.game)) || fileSrc(coverOf(cell.game));
    const cover = cell.item.cover;
    if (!cover) return "";
    return cell.item.kind === "anime" ? animeStore.getImg(cover) || cover : cover;
  });

  const clock = $derived(now.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }));

  let contentRailEl: HTMLElement | undefined = $state();
  let railEl: HTMLElement | undefined = $state();
  let gamePlatformRailEl: HTMLElement | undefined = $state();
  let quickMenuEl: HTMLElement | undefined = $state();
  let pad: GamepadAttachment | null = null;
  let immersiveSystemBars = readHandheldImmersivePreference();

  const SECTION_ICONS: Record<string, string> = { recent: "clock", games: "gamepad", anime: "tv", comic: "image", novel: "book" };
  const KIND_LABELS: Record<string, string> = { game: "游玩", anime: "番剧", comic: "漫画", novel: "小说" };

  const railItems = $derived.by<HandheldRailItem[]>(() => cells.map((cell) => ({
    id: cell.type === "game" ? cell.game.id : cell.item.key,
    title: cell.type === "game" ? cell.game.name : cell.item.title,
    cover: coverOfCell(cell),
    subtitle: cellSubtitle(cell),
    kind: cell.type === "game" ? "game" : cell.item.kind,
    progress: cell.type === "media" && cell.item.kind === "novel"
      ? Number(novelStore.history.find((entry) => entry.key === cell.item.key)?.progress ?? 0)
      : undefined,
  })));

  const heroTitle = $derived(focusCell ? (focusCell.type === "game" ? focusCell.game.name : focusCell.item.title) : "开始建立你的媒体档案");
  const heroSubtitle = $derived(focusCell
    ? (focusCell.type === "game" ? focusMetaLine() : (cellSubtitle(focusCell) || focusMetaLine()))
    : "游戏、番剧、漫画与小说，都从这里继续。");
  const heroKind = $derived<"game" | "anime" | "comic" | "novel">(focusCell?.type === "game" ? "game" : focusCell?.item.kind ?? "game");
  const heroProgress = $derived(focusCell?.type === "media" && focusCell.item.kind === "novel"
    ? Number(novelStore.history.find((entry) => entry.key === focusCell.item.key)?.progress ?? 0)
    : 0);

  function closeQuickMenu() {
    quickMenuOpen = false;
    closeOverlay(QUICK_MENU_OVERLAY_ID);
  }

  function focusQuickMenuItem() {
    requestAnimationFrame(() => {
      quickMenuEl
        ?.querySelector<HTMLButtonElement>(`[data-quick-index="${quickMenuIndex}"]`)
        ?.focus({ preventScroll: true });
    });
  }

  function openQuickMenu() {
    quickMenuIndex = 0;
    quickMenuOpen = true;
    openOverlay(
      { id: QUICK_MENU_OVERLAY_ID, kind: "drawer" },
      () => { quickMenuOpen = false; },
    );
    focusQuickMenuItem();
  }

  function toggleQuickMenu() {
    if (quickMenuOpen) closeQuickMenu();
    else openQuickMenu();
  }

  function moveQuickMenu(delta: number) {
    if (quickNavItems.length === 0) return;
    quickMenuIndex = (quickMenuIndex + delta + quickNavItems.length) % quickNavItems.length;
    focusQuickMenuItem();
  }

  function moveWelcomeAction(delta: number) {
    welcomeActionIndex = (welcomeActionIndex + delta + welcomeActions.length) % welcomeActions.length;
  }

  function activateWelcomeAction() {
    welcomeActions[welcomeActionIndex]?.run();
  }

  function activateQuickMenu() {
    const item = quickNavItems[quickMenuIndex];
    if (!item) return;
    closeQuickMenu();
    navigateTo(item.view);
  }

  /**
   * XMB 频道栏只在切频道时短暂出现，给游戏转盘和媒体主视觉留出更多空间。
   * LT/RT、键盘 Q/E 或触控点按频道都会重新显示并重新计时；隐藏时保留一个
   * 轻量恢复按钮，避免纯触控用户失去频道入口。
   */
  function revealCategoryRail() {
    categoryRailVisible = true;
    if (categoryRailHideTimer !== undefined) window.clearTimeout(categoryRailHideTimer);
    categoryRailHideTimer = window.setTimeout(() => {
      categoryRailVisible = false;
      categoryRailHideTimer = undefined;
    }, 3000);
  }

  /* ---- 生命周期 ---- */

  onMount(() => {
    const memory = readMemory();
    const timer = setInterval(() => (now = new Date()), 30_000);
    void refreshBattery();
    void gameStore.load().then(() => {
      restoreMemory(memory);
    });
    void loadComicBookshelf();
    // 掌机模式临时锁横屏，离开时不覆盖用户保存的方向偏好。
    if (platformStore.isAndroid) void orientationStore.enterMediaLandscape();
    const applySystemBars = () => {
      if (platformStore.isAndroid) void setHandheldSystemBars(immersiveSystemBars).catch(() => {});
    };
    applySystemBars();
    const stopHandheldPrefs = onHandheldPrefsChanged(() => {
      immersiveSystemBars = readHandheldImmersivePreference();
      applySystemBars();
    });
    const onVisibility = () => {
      if (document.visibilityState === "visible") applySystemBars();
    };
    document.addEventListener("visibilitychange", onVisibility);
    pad = attachGamepad(
      {
        left: () => quickMenuOpen ? moveQuickMenu(-1) : moveFocus(-1),
        right: () => quickMenuOpen ? moveQuickMenu(1) : moveFocus(1),
        up: () => quickMenuOpen ? moveQuickMenu(-3) : hasAnyContent ? moveFocus(-1) : moveWelcomeAction(-1),
        down: () => quickMenuOpen ? moveQuickMenu(3) : hasAnyContent ? moveFocus(1) : moveWelcomeAction(1),
        pageLeft: () => quickMenuOpen ? moveQuickMenu(-1) : section?.id === "games" ? switchGameSystem(-1) : undefined,
        pageRight: () => quickMenuOpen ? moveQuickMenu(1) : section?.id === "games" ? switchGameSystem(1) : undefined,
        categoryLeft: () => quickMenuOpen ? moveQuickMenu(-1) : switchSection(-1),
        categoryRight: () => quickMenuOpen ? moveQuickMenu(1) : switchSection(1),
        activate: () => quickMenuOpen ? activateQuickMenu() : hasAnyContent ? openFocusedDetails() : activateWelcomeAction(),
        launch: () => quickMenuOpen
          ? activateQuickMenu()
          : section?.id === "games" && cells.length === 0
            ? navigateTo("handheld-import")
            : hasAnyContent
              ? void openFocused()
              : activateWelcomeAction(),
        favorite: () => favoriteFocused(),
        filter: () => quickMenuOpen ? closeQuickMenu() : section?.id === "games" ? navigateTo("game-library") : openQuickMenu(),
        back: () => quickMenuOpen ? closeQuickMenu() : navigateTo("home"),
        start: () => quickMenuOpen ? closeQuickMenu() : navigateTo("handheld-import"),
      },
      { id: "handheld", zone: "content" },
    );
    if (typeof navigator !== "undefined" && navigator.getGamepads) {
      padConnected = navigator.getGamepads().some((p) => p && p.connected);
    }
    const onPad = (e: GamepadEvent) =>
      (padConnected = e.type === "gamepadconnected" ? true : navigator.getGamepads().some((p) => p && p.connected));
    window.addEventListener("gamepadconnected", onPad);
    window.addEventListener("gamepaddisconnected", onPad);
    // Android WebView 的 DPAD 事件可能先触发原生默认焦点移动；捕获阶段
    // 接管方向键，避免焦点从 XMB 频道跳到 Logo 或浏览器默认按钮。
    window.addEventListener("keydown", onKeydown, true);
    let focusTimer: number | undefined;
    if (platformStore.isAndroid) {
      // 路由恢复会先把焦点放到标题；掌机首页最终应落在 XMB 频道，
      // 让第一次 DPAD 输入和视觉焦点都从可操作频道开始。
      focusTimer = window.setTimeout(focusSectionButton, 180);
    }
    return () => {
      clearInterval(timer);
      if (categoryRailHideTimer !== undefined) window.clearTimeout(categoryRailHideTimer);
      if (focusTimer !== undefined) window.clearTimeout(focusTimer);
      window.removeEventListener("gamepadconnected", onPad);
      window.removeEventListener("gamepaddisconnected", onPad);
      window.removeEventListener("keydown", onKeydown, true);
      stopHandheldPrefs();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  });

  onDestroy(() => {
    pad?.();
    pad = null;
    closeOverlay(QUICK_MENU_OVERLAY_ID);
    if (platformStore.isAndroid) void orientationStore.exitMediaLandscape();
  });

  async function refreshBattery() {
    try {
      const nav = navigator as Navigator & {
        getBattery?: () => Promise<{
          level: number;
          charging: boolean;
          addEventListener?: (type: string, cb: () => void) => void;
        }>;
      };
      if (!nav.getBattery) return;
      const b = await nav.getBattery();
      const update = () => {
        battery = { level: Math.round(b.level * 100), charging: b.charging };
      };
      update();
      b.addEventListener?.("levelchange", update);
      b.addEventListener?.("chargingchange", update);
    } catch {
      /* 无电池 API（桌面）忽略 */
    }
  }

  /** 漫画书架依赖 PicACG 登录，登录后才拉取。 */
  async function loadComicBookshelf() {
    try {
      if (comicStore.isLoggedIn && comicStore.favorites.length === 0) {
        await comicStore.loadFavorites(1);
      }
    } catch {
      /* 书架不可用不影响历史 */
    }
  }

  /* ---- 位置记忆 ---- */

  function readMemory(): unknown {
    try {
      const current = localStorage.getItem(MEMORY_KEY);
      if (current) return JSON.parse(current) as unknown;
      // 旧版本的 key 仍然读取一次，由 migrateHandheldMemory 统一转换。
      return JSON.parse(localStorage.getItem("moeplay-handheld-memory-v2") ?? "{}") as unknown;
    } catch {
      return {};
    }
  }

  function persistMemory() {
    try {
      const memory = migrateHandheldMemory(readMemory(), gameSystemOptions.filter((item) => item.id !== "all").map((item) => item.id));
      const key = focusCell?.type === "game" ? focusCell.game.id : focusCell?.type === "media" ? focusCell.item.key : "";
      const viewKey = handheldViewKey(section?.id ?? "recent", gameSystemId);
      memory.channel = (section?.id ?? "recent") as HandheldChannelId;
      memory.gameSystemId = gameSystemId;
      memory.focusByView[viewKey] = focusIdx;
      if (key) memory.keyByView[viewKey] = key;
      localStorage.setItem(MEMORY_KEY, JSON.stringify(memory));
    } catch {
      /* ignore */
    }
  }

  function restoreMemory(raw: unknown) {
    const memory: HandheldMemoryV3 = migrateHandheldMemory(
      raw,
      gameSystemOptions.filter((item) => item.id !== "all").map((item) => item.id),
    );
    const channelIndex = sections.findIndex((item) => item.id === memory.channel);
    if (channelIndex >= 0) sectionIdx = channelIndex;
    const systemIndex = gameSystemOptions.findIndex((item) => item.id === memory.gameSystemId);
    if (systemIndex >= 0) gameSystemIdx = systemIndex;
    focusMap = { ...focusMap, ...memory.focusByView };
    const viewKey = handheldViewKey(memory.channel, memory.gameSystemId);
    const key = memory.keyByView[viewKey];
    if (key) {
      requestAnimationFrame(() => {
        const idx = cells.findIndex((c) =>
          c.type === "game" ? c.game.id === key : c.item.key === key,
        );
        if (idx >= 0) setFocus(idx);
      });
    }
  }

  /* ---- 焦点与导航 ---- */

  function setFocus(idx: number) {
    if (!section || cells.length === 0) return;
    focusMap[focusViewKey] = Math.min(Math.max(idx, 0), cells.length - 1);
  }

  function moveFocus(delta: number) {
    if (cells.length === 0) return;
    setFocus((focusIdx + delta + cells.length * 10) % cells.length);
    persistMemory();
    scrollFocusIntoView();
    focusContentCard();
  }

  function switchSection(delta: number) {
    if (sections.length === 0) return;
    sectionIdx = wrappedIndex(sectionIdx, delta, sections.length);
    revealCategoryRail();
    persistMemory();
    scrollRailIntoView();
    focusSectionButton();
  }

  function switchGameSystem(delta: number) {
    if (gameSystemOptions.length === 0) return;
    persistMemory();
    gameSystemIdx = wrappedIndex(gameSystemIdx, delta, gameSystemOptions.length);
    requestAnimationFrame(() => {
      focusContentCard();
      scrollFocusIntoView();
    });
    persistMemory();
  }

  function selectGameSystem(index: number) {
    if (index < 0 || index >= gameSystemOptions.length) return;
    persistMemory();
    gameSystemIdx = index;
    requestAnimationFrame(() => {
      focusContentCard();
      scrollFocusIntoView();
    });
    persistMemory();
  }

  function selectSection(idx: number) {
    sectionIdx = idx;
    revealCategoryRail();
    persistMemory();
    focusSectionButton();
  }

  function onCellTap(i: number) {
    if (i === focusIdx) void openFocused();
    else {
      setFocus(i);
      persistMemory();
    }
  }

  function gridColumns(): number {
    // 转盘和媒体轨道都是单轴浏览；上下键也采用单步，方便掌机单手操作。
    return 1;
  }

  function scrollFocusIntoView() {
    requestAnimationFrame(() => {
      contentRailEl
        ?.querySelector(`[data-rail-index="${Math.min(focusIdx, Math.max(cells.length - 1, 0))}"], [data-game-index="${Math.min(focusIdx, Math.max(cells.length - 1, 0))}"]`)
        ?.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
    });
  }

  function focusSectionButton() {
    requestAnimationFrame(() => {
      railEl
        ?.querySelector<HTMLButtonElement>(`[data-sys-idx="${sectionIdx}"]`)
        ?.focus({ preventScroll: true });
    });
  }

  function focusContentCard() {
    requestAnimationFrame(() => {
      contentRailEl
        ?.querySelector<HTMLButtonElement>(`[data-rail-index="${Math.min(focusIdx, Math.max(cells.length - 1, 0))}"], [data-game-index="${Math.min(focusIdx, Math.max(cells.length - 1, 0))}"]`)
        ?.focus({ preventScroll: true });
    });
  }

  function scrollRailIntoView() {
    requestAnimationFrame(() => {
      railEl
        ?.querySelector(`[data-sys-idx="${sectionIdx}"]`)
        ?.scrollIntoView({ inline: "center", block: "nearest", behavior: "smooth" });
    });
  }

  /* ---- 打开（A 键） ---- */

  async function openFocused() {
    const cell = focusCell;
    if (!cell || launching) return;
    if (cell.type === "game") {
      launching = cell.game.id;
      uiStore.notify(`正在启动 ${cell.game.name}…`);
      try {
        await gameStore.launch(cell.game.id);
      } finally {
        launching = null;
      }
      return;
    }
    const item = cell.item;
    if (item.kind === "anime") return openAnime(item);
    if (item.kind === "comic") return openComic(item);
    return openNovel(item);
  }

  function openFocusedDetails() {
    const cell = focusCell;
    if (!cell) return;
    if (cell.type === "game") gameStore.selectGame(cell.game.id);
    navigateTo(cell.type === "game" ? "game-detail" : cell.item.kind);
  }

  async function openAnime(item: HandheldUnifiedItem) {
    const entry = historyStore.get(item.key) ?? animeStore.history.find((h) => h.name === item.title);
    if (entry) {
      await openUnifiedMediaHistory({ kind: "anime", payload: entry });
      return;
    }
    navigateTo("anime");
    const col = collectionStore.items.find((c) => c.name === item.title);
    if (col?.ruleSource && col.sourceUrl) {
      animeStore.openDetail(col.ruleSource, { name: col.name, url: col.sourceUrl }, col.image);
    } else {
      uiStore.notify("该番剧缺少可播放的来源信息，请到番剧页搜索后播放", "info");
    }
  }

  async function openComic(item: HandheldUnifiedItem) {
    const rec = comicStore.readHistory.find((r) => r.id === item.key);
    if (rec) await openUnifiedMediaHistory({ kind: "comic", payload: rec });
    else { navigateTo("comic"); await comicStore.openComic(item.key); }
  }

  async function openNovel(item: HandheldUnifiedItem) {
    const entry = novelStore.history.find((e) => e.key === item.key);
    if (entry) await openUnifiedMediaHistory({ kind: "novel", payload: entry });
    else navigateTo("novel");
  }

  /* ---- 收藏 / 追番（X 键） ---- */

  function favoriteFocused() {
    const cell = focusCell;
    if (!cell) return;
    if (cell.type === "game") {
      void gameStore.toggleFavorite(cell.game.id);
      return;
    }
    const item = cell.item;
    if (item.kind !== "anime") return;
    const current = collectionStore.getType(item.title);
    if (current > 0) {
      collectionStore.setCollect(item.title, 0, undefined, { image: item.cover, ruleName: "", sourceUrl: "" });
      uiStore.notify(`已取消追番「${item.title}」`);
    } else {
      const h = historyStore.get(item.key);
      collectionStore.setCollect(
        item.title,
        1,
        { image: item.cover },
        { image: item.cover, ruleName: h?.ruleName ?? "", sourceUrl: h?.sourceUrl ?? "" },
      );
      uiStore.notify(`已加入追番「${item.title}」`);
    }
  }

  function isAnimeCollected(cell: Cell | null): boolean {
    return cell?.type === "media" && cell.item.kind === "anime" && collectionStore.getType(cell.item.title) > 0;
  }

  /* ---- 键盘镜像 ---- */

  function onKeydown(e: KeyboardEvent) {
    if (e.defaultPrevented) return;
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return;
    const key = e.key;
    const keyCode = e.keyCode;
    const normalizedKey =
      key === "DPAD_LEFT" || key === "DpadLeft" || keyCode === 21 ? "ArrowLeft"
      : key === "DPAD_RIGHT" || key === "DpadRight" || keyCode === 22 ? "ArrowRight"
      : key === "DPAD_UP" || key === "DpadUp" || keyCode === 19 ? "ArrowUp"
      : key === "DPAD_DOWN" || key === "DpadDown" || keyCode === 20 ? "ArrowDown"
      : keyCode === 96 || key === "GamepadA" ? "Enter"
      : keyCode === 97 || key === "GamepadB" ? "Escape"
      : keyCode === 99 || key === "GamepadX" ? "f"
      : keyCode === 100 || key === "GamepadY" ? "y"
      : key;
    if (quickMenuOpen) {
      switch (normalizedKey) {
        case "ArrowLeft": moveQuickMenu(-1); break;
        case "ArrowRight": moveQuickMenu(1); break;
        case "ArrowUp": moveQuickMenu(-3); break;
        case "ArrowDown": moveQuickMenu(3); break;
        case "Enter": activateQuickMenu(); break;
        case "Escape": closeQuickMenu(); break;
        default: return;
      }
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    switch (normalizedKey) {
      case "ArrowLeft": hasAnyContent ? moveFocus(-1) : moveWelcomeAction(-1); break;
      case "ArrowRight": hasAnyContent ? moveFocus(1) : moveWelcomeAction(1); break;
      case "ArrowUp": hasAnyContent ? moveFocus(-gridColumns()) : moveWelcomeAction(-1); break;
      case "ArrowDown": hasAnyContent ? moveFocus(gridColumns()) : moveWelcomeAction(1); break;
      case "PageUp": section?.id === "games" ? switchGameSystem(-1) : moveFocus(-1); break;
      case "PageDown": section?.id === "games" ? switchGameSystem(1) : moveFocus(1); break;
      case "q": case "Q": switchSection(-1); break;
      case "e": case "E": switchSection(1); break;
      case "Enter": section?.id === "games" && cells.length === 0 ? navigateTo("handheld-import") : hasAnyContent ? void openFocused() : activateWelcomeAction(); break;
      case "y": case "Y": openFocusedDetails(); break;
      case "f": case "F": favoriteFocused(); break;
      case "s": case "S": section?.id === "games" ? navigateTo("game-library") : openQuickMenu(); break;
      case "Escape": navigateTo("home"); break;
      default: return;
    }
    e.preventDefault();
    e.stopPropagation();
  }

  /* ---- 展示辅助 ---- */

  function coverOfCell(cell: Cell | null): string {
    if (!cell) return "";
    if (cell.type === "game") return fileSrc(coverOf(cell.game)) ?? "";
    const cover = cell.item.cover;
    if (!cover) return "";
    return cell.item.kind === "anime" ? animeStore.getImg(cover) || cover : cover;
  }

  function cellSubtitle(cell: Cell): string {
    if (cell.type === "game") return "";
    return cell.item.subtitle;
  }

  function focusMetaLine(): string {
    const cell = focusCell;
    if (!cell) return "";
    if (cell.type === "game") {
      const raw = gameLastPlayed(cell.game);
      const played = raw ? new Date(raw) : null;
      const last = played && !Number.isNaN(played.getTime()) ? `最近 ${played.toLocaleDateString("zh-CN")}` : "未游玩";
      return `${last} · ${formatPlayTime(gameTotalSeconds(cell.game))}`;
    }
    return timeLabel(cell.item.time);
  }

  function timeLabel(ms: number): string {
    if (!ms) return "";
    const diff = Date.now() - ms;
    if (diff < 3_600_000) return `${Math.max(Math.round(diff / 60_000), 1)} 分钟前`;
    if (diff < 86_400_000) return `${Math.round(diff / 3_600_000)} 小时前`;
    if (diff < 7 * 86_400_000) return `${Math.round(diff / 86_400_000)} 天前`;
    return new Date(ms).toLocaleDateString("zh-CN");
  }

  function primaryActionLabel(): string {
    const cell = focusCell;
    if (!cell) return "";
    if (cell.type === "game") return launching === cell.game.id ? "启动中…" : "启动游戏";
    return cell.item.kind === "anime" ? "播放" : cell.item.kind === "comic" ? "继续阅读" : "继续阅读";
  }

  $effect(() => {
    sections.length;
    if (sectionIdx >= sections.length) sectionIdx = Math.max(sections.length - 1, 0);
  });
  $effect(() => {
    cells.length;
    if (focusMap[focusViewKey] >= cells.length) setFocus(Math.max(cells.length - 1, 0));
  });
  $effect(() => {
    sectionIdx;
    scrollRailIntoView();
  });
  $effect(() => {
    gameSystemIdx;
    gamePlatformRailEl
      ?.querySelector(`[data-platform-index="${gameSystemIdx}"]`)
      ?.scrollIntoView({ inline: "center", block: "nearest", behavior: "smooth" });
  });
</script>

<div class="handheld-page hh-xmb-page" data-testid="handheld-page">
  <!-- 背景：聚焦条目的封面驱动，缺图时使用原创掌机氛围素材。 -->
  <HandheldArtworkStage
    class="hh-backdrop"
    source={{ role: "home", cover: backdropSrc || null }}
    strength="immersive"
    fill
  />
  <div class="hh-xmb-vignette" aria-hidden="true"></div>
  <div class="hh-xmb-scanlines" aria-hidden="true"></div>

  <!-- PSP/XMB 顶栏：信息保持克制，真正的频道选择交给中间的大图标轨道。 -->
  <header class="hh-topbar hh-xmb-topbar">
    <div class="hh-brand">
      <button type="button" class="hh-brand-home" tabindex="-1" aria-label="返回掌机首页" onclick={() => navigateTo("home")}>
        <span class="hh-logo">萌游</span>
        <span class="hh-mode">PORTABLE / XMB</span>
      </button>
      <div class="hh-xmb-context" aria-live="polite">
        <span>主屏幕</span>
        <b>{section?.label ?? "欢迎"}</b>
      </div>
    </div>
    <!-- 频道与品牌/主屏幕/游戏库/导入同排，横屏时把内容舞台让给当前选择。 -->
    {#if categoryRailVisible}
      <nav class="hh-systems hh-xmb-categories" aria-label="掌机频道" bind:this={railEl}>
        <div class="hh-xmb-category-track">
          {#each sections as sys, i}
            <button
              type="button"
              class="hh-system hh-xmb-category"
              class:active={i === sectionIdx}
              data-sys-idx={i}
              aria-current={i === sectionIdx ? "page" : undefined}
              tabindex="-1"
              onfocus={() => { revealCategoryRail(); if (sectionIdx !== i) { sectionIdx = i; persistMemory(); } }}
              onclick={() => selectSection(i)}
            >
              <span class="hh-xmb-category-icon">
                {#if SECTION_ICONS[sys.id]}<Icon name={SECTION_ICONS[sys.id]} size={18} />{:else}<span>{sys.label.slice(0, 1)}</span>{/if}
              </span>
              <span class="hh-system-label">{sys.label}</span>
              <span class="hh-system-count">{sys.count}</span>
            </button>
          {/each}
        </div>
      </nav>
    {:else}
      <button type="button" class="hh-xmb-category-reveal" aria-label="显示掌机频道" onclick={revealCategoryRail}>
        <span>频道</span><b>LT · RT</b>
      </button>
    {/if}
    <div class="hh-status">
      <span class="hh-profile">PLAYER 01</span>
      {#if padConnected}<span class="hh-pad-dot" title="手柄已连接"></span>{/if}
      {#if battery}
        <span class="hh-batt" class:low={battery.level <= 20 && !battery.charging} title={battery.charging ? "充电中" : "电池"}>
          {#if battery.charging}<Icon name="zap" size={11} />{/if}
          {battery.level}%
        </span>
      {/if}
      <span class="hh-clock">{clock}</span>
      <button type="button" class="hh-import-btn hh-home-shortcut" data-testid="handheld-game-library-entry" tabindex="-1" aria-label="打开游戏库" onclick={() => navigateTo("game-library")}>
        <Icon name="gamepad" size={14} /> <span>游戏库</span>
      </button>
      <button type="button" class="hh-import-btn hh-home-shortcut" data-testid="handheld-emulator-import-entry" tabindex="-1" aria-label="打开模拟器导入" onclick={() => navigateTo("handheld-import")}>
        <Icon name="database" size={14} /> <span>导入</span>
      </button>
      <button type="button" class="hh-import-btn" tabindex="-1" aria-label="打开更多功能" onclick={toggleQuickMenu}>
        <Icon name="grid" size={14} /> <span>功能</span>
      </button>
    </div>
  </header>

  {#if section?.id === "games"}
    <nav class="hh-game-platforms" aria-label="模拟器平台" data-testid="handheld-game-platforms">
      <span class="hh-game-platforms__hint"><b>LB</b><i></i><small>平台</small></span>
      <div class="hh-game-platforms__track" bind:this={gamePlatformRailEl}>
        {#each gameSystemOptions as system, i (system.id)}
          <button
            type="button"
            class:active={i === gameSystemIdx}
            data-platform-index={i}
            aria-current={i === gameSystemIdx ? "page" : undefined}
            onclick={() => selectGameSystem(i)}
          >
            <strong>{system.label}</strong><small>{system.count}</small>
          </button>
        {/each}
      </div>
      <span class="hh-game-platforms__hint right"><small>切换</small><i></i><b>RB</b></span>
    </nav>
  {/if}

  {#if quickMenuOpen}
    <button class="hh-menu-scrim" type="button" aria-label="关闭全部功能" onclick={closeQuickMenu}></button>
    <dialog
      class="hh-menu-panel"
      bind:this={quickMenuEl}
      open
      aria-labelledby="hh-menu-title"
    >
      <header class="hh-menu-head">
        <div>
          <span class="hh-menu-kicker">MOEPLAY · QUICK ACCESS</span>
          <h2 id="hh-menu-title">全部功能</h2>
        </div>
        <button type="button" class="hh-menu-close" aria-label="关闭全部功能" onclick={closeQuickMenu}>
          <Icon name="x" size={18} />
        </button>
      </header>
      <div class="hh-menu-groups">
        {#each quickNavGroups as group (group.id)}
          <section class="hh-menu-group" aria-labelledby={`hh-menu-group-${group.id}`}>
            <h3 id={`hh-menu-group-${group.id}`}>{group.label}</h3>
            <div class="hh-menu-grid">
              {#each group.items as item (item.id)}
                {@const index = quickNavItems.indexOf(item)}
                <button
                  type="button"
                  class:active={index === quickMenuIndex}
                  data-quick-index={index}
                  onclick={() => { closeQuickMenu(); navigateTo(item.view); }}
                >
                  <span class="hh-menu-glyph"><Icon name={item.icon} size={19} /></span>
                  <span class="hh-menu-item-copy">
                    <strong>{item.label}</strong>
                    <small>{item.ariaLabel}</small>
                  </span>
                </button>
              {/each}
            </div>
          </section>
        {/each}
      </div>
      <footer class="hh-menu-foot"><span><b class="k">✥</b>选择</span><span><b class="k">A</b>打开</span><span><b class="k">B</b>关闭</span></footer>
    </dialog>
  {/if}

  {#if !hasAnyContent && section?.id === "recent"}
    <main class="hh-main hh-xmb-main hh-xmb-welcome-main">
      <section class="hh-xmb-welcome" data-testid="handheld-xmb-welcome" aria-label="欢迎使用萌游掌机模式">
        <div class="hh-xmb-welcome-art">
          <img src={homeAmbient} alt="" />
          <div class="hh-xmb-welcome-art-copy"><span>MOEPLAY</span><b>PORTABLE</b><small>YOUR MEDIA. YOUR WAY.</small></div>
        </div>
        <div class="hh-xmb-welcome-copy">
          <span class="hh-xmb-copy-kicker"><span>FIRST BOOT</span><i></i><small>READY TO PLAY</small></span>
          <h1>你的掌机世界，从这里开始。</h1>
          <p>导入游戏，或挑选一类媒体开始探索。之后所有最近游玩、观看与阅读记录都会回到这个首页。</p>
          <div class="hh-xmb-welcome-actions" role="listbox" aria-label="快速开始">
            {#each welcomeActions as action, index (action.id)}
              <button
                type="button"
                class:active={index === welcomeActionIndex}
                role="option"
                aria-selected={index === welcomeActionIndex}
                data-welcome-index={index}
                onclick={() => action.run()}
              >
                <span class="hh-xmb-welcome-icon"><Icon name={action.icon} size={18} /></span>
                <span><b>{action.label}</b><small>{action.description}</small></span>
              </button>
            {/each}
          </div>
          <div class="hh-xmb-welcome-tip"><b>A</b>确认当前入口 <span>·</span> <b>↑ ↓</b>切换入口 <span>·</span> <b>START</b>稍后导入</div>
        </div>
      </section>
    </main>
  {:else if section && cells.length === 0 && section.id !== "games"}
    <div class="hh-empty">
      {#if section.id === "recent"}
        <HandheldStatePanel state="empty" compact title="还没有最近活动" description="启动一款游戏，或播放/阅读任意内容后，这里会成为你的继续入口。" primaryAction={{ label: "打开游戏频道", run: () => { selectSection(1); } }} />
      {:else if section.id === "anime"}
        <HandheldStatePanel state="empty" compact title="暂无番剧活动" description="搜索并播放一集后，记录会自动出现在继续轨道。" primaryAction={{ label: "去看番剧", run: () => { navigateTo("anime"); } }} />
      {:else if section.id === "comic"}
        <HandheldStatePanel state="empty" compact title="暂无漫画活动" description="阅读任意章节后，历史和书架会汇总到这里。" primaryAction={{ label: "去看漫画", run: () => { navigateTo("comic"); } }} />
      {:else if section.id === "novel"}
        <HandheldStatePanel state="empty" compact title="暂无小说活动" description="开始阅读后，分页进度会显示在继续轨道。" primaryAction={{ label: "去看小说", run: () => { navigateTo("novel"); } }} />
      {:else}
        <HandheldStatePanel state="empty" compact title="这个游戏系统还没有档案" description="导入 ROM 或从游戏库补全你的掌机档案。" primaryAction={{ label: "模拟器导入", run: () => { navigateTo("handheld-import"); } }} secondaryAction={{ label: "打开游戏库", run: () => { navigateTo("game-library"); } }} />
      {/if}
    </div>
  {:else}
    <main class="hh-main hh-xmb-main">
      {#key section?.id}
        <div class="hh-stage hh-xmb-stage">
          {#if section?.id === "games"}
            <div class="hh-game-stage" bind:this={contentRailEl}>
              <HandheldGameWheel
                games={gameWheelGames}
                focusIdx={focusIdx}
                platformLabel={gameSystemOptions[gameSystemIdx]?.label ?? "全部游戏"}
                launching={launching}
                onSelect={(index) => { setFocus(index); persistMemory(); }}
                onActivate={() => void openFocused()}
                onFavorite={() => favoriteFocused()}
                onOpenImport={() => navigateTo("handheld-import")}
                onOpenLibrary={() => navigateTo("game-library")}
              />
            </div>
          {:else if focusCell}
            <section class="hh-xmb-selection" data-testid="handheld-xmb-selection" aria-label="当前选择">
              <div class="hh-xmb-selected-art">
                <div class="hh-xmb-art-frame">
                  {#if coverOfCell(focusCell)}
                    <img src={coverOfCell(focusCell)} alt="" />
                  {:else}
                    <span class="hh-cell-placeholder big">{heroTitle.slice(0, 2)}</span>
                  {/if}
                  <span class="hh-xmb-art-gloss" aria-hidden="true"></span>
                </div>
                <div class="hh-xmb-index"><b>{String(focusIdx + 1).padStart(2, "0")}</b><span>/ {String(cells.length).padStart(2, "0")}</span></div>
              </div>
              <div class="hh-xmb-copy">
                <div class="hh-xmb-copy-kicker"><span>{section?.label ?? "CONTINUE"}</span><i></i><small>{heroKind.toUpperCase()}</small></div>
                <h1>{heroTitle}</h1>
                <p class="hh-xmb-subtitle">{heroSubtitle || "准备好开始新的内容。"}</p>
                <div class="hh-xmb-meta">
                  <span class="hh-chip">{focusCell.type === "game" ? section?.label : KIND_LABELS[focusCell.item.kind]}</span>
                  {#if focusMetaLine()}<span>{focusMetaLine()}</span>{/if}
                  {#if focusCell.type === "media" && focusCell.item.subtitle}<span>{focusCell.item.subtitle}</span>{/if}
                </div>
                {#if heroProgress > 0}
                  <div class="hh-xmb-progress"><span>阅读进度</span><b>{Math.round(heroProgress * 100)}%</b><i><em style={`width:${heroProgress * 100}%`}></em></i></div>
                {/if}
                <div class="hh-xmb-actions">
                  <button type="button" class="hh-xmb-action primary" onclick={() => void openFocused()} disabled={focusCell.type === "game" && launching === focusCell.game.id}>
                    <b class="hh-button-letter">A</b><span>{primaryActionLabel()}</span>
                  </button>
                  <button type="button" class="hh-xmb-action" onclick={() => navigateTo(focusCell.type === "game" ? "game-detail" : focusCell.item.kind)}>
                    <b class="hh-button-letter">Y</b><span>详情</span>
                  </button>
                </div>
              </div>
              <aside class="hh-xmb-inspector" aria-label="当前栏目状态">
                <span class="hh-xmb-inspector-label">NOW SELECTED</span>
                <strong>{String(focusIdx + 1).padStart(2, "0")}</strong>
                <span class="hh-xmb-inspector-line"></span>
                <small>{section?.label ?? "内容"} · {cells.length} 项</small>
                <small>{padConnected ? "CONTROLLER READY" : "TOUCH READY"}</small>
              </aside>
            </section>
          {/if}
          {#if section?.id !== "games"}
            <div class="hh-xmb-rail-wrap" bind:this={contentRailEl}>
              <HandheldRail
                title={section?.label ?? "内容"}
                kicker={section?.id === "recent" ? "CONTINUE" : "MEDIA CHANNEL"}
                items={railItems}
                activeIndex={focusIdx}
                onSelect={(_, index) => { setFocus(index); persistMemory(); }}
                onOpen={() => void openFocused()}
              />
            </div>
          {/if}
        </div>
      {/key}
    </main>

  {/if}
  <!-- 底部手柄提示条：空态、加载态也始终告诉用户如何操作。 -->
  <footer class="hh-hints hh-xmb-hints">
    <span><b class="k">LT·RT</b>频道</span>
    <span><b class="k">← →</b>{hasAnyContent ? "选择" : "入口"}</span>
    {#if !hasAnyContent}
      <span><b class="k">A</b>打开</span>
    {:else if section?.id === "games"}
      <span><b class="k">A</b>启动</span>
      <span><b class="k">X</b>收藏</span>
    {:else if section?.id === "anime"}
      <span><b class="k">A</b>播放</span>
      <span><b class="k">X</b>追番</span>
    {:else if section?.id === "comic" || section?.id === "novel"}
      <span><b class="k">A</b>阅读</span>
    {:else}
      <span><b class="k">A</b>打开</span>
    {/if}
    {#if section?.id === "games"}<span><b class="k">LB·RB</b>平台</span>{/if}
    <span><b class="k">VIEW</b>游戏库</span>
    <span><b class="k">START</b>导入</span>
    <span><b class="k">B</b>返回</span>
  </footer>
</div>

<style>
  .handheld-page {
    --hh-pad: clamp(14px, 2vw, 26px);
    position: relative;
    isolation: isolate;
    height: 100dvh;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-void);
    color: var(--text-primary);
    font-family: var(--font-ui);
    user-select: none;
  }

  /* ---- 基础顶部信息 ---- */
  .hh-topbar { position: relative; z-index: 1; }
  .hh-brand { display: flex; align-items: center; min-width: 0; }
  .hh-brand-home { display: grid; gap: 3px; min-width: 54px; padding: 0; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .hh-brand-home:focus-visible { outline: 2px solid var(--accent-hi); outline-offset: 3px; }
  .hh-logo { font-family: var(--font-display); font-weight: 800; color: var(--accent); line-height: 1; }
  .hh-mode { color: var(--text-muted); }
  .hh-status { display: inline-flex; align-items: center; gap: 10px; }
  .hh-status > button { min-height: 36px; }
  .hh-batt { display: inline-flex; align-items: center; gap: 4px; padding: 5px 9px; border: 1px solid var(--border); border-radius: 99px; background: var(--glass-bg); color: var(--text-secondary); font: 600 .72rem/1 var(--font-mono); }
  .hh-batt.low { color: var(--color-warning); border-color: color-mix(in srgb, var(--color-warning) 42%, transparent); }
  .hh-pad-dot { width: 8px; height: 8px; border-radius: 99px; background: var(--color-success); box-shadow: 0 0 8px color-mix(in srgb, var(--color-success) 60%, transparent); }
  .hh-clock { color: var(--text-secondary); font: .82rem/1 var(--font-mono); }
  .hh-import-btn { display: inline-flex; align-items: center; gap: 5px; min-height: 36px; padding: 7px 12px; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--bg-elev); color: var(--text-primary); font: 600 .76rem/1 var(--font-ui); cursor: pointer; }
  .hh-import-btn:hover { border-color: var(--accent-ring); color: var(--accent-hi); }
  .hh-home-shortcut { border-color: color-mix(in srgb, var(--accent) 34%, var(--border)); background: color-mix(in srgb, var(--accent) 8%, var(--bg-elev)); }

  /* ---- 全功能抽屉：掌机全屏时替代桌面侧栏/移动端底栏 ---- */
  .hh-menu-scrim {
    position: fixed; inset: 0; z-index: 20;
    border: 0; background: rgb(2 3 7 / .68); cursor: default;
  }
  .hh-menu-panel {
    position: fixed; z-index: 21;
    top: clamp(68px, 9vh, 92px); right: var(--hh-pad);
    width: min(760px, calc(100vw - 2 * var(--hh-pad)));
    max-height: calc(100dvh - 132px); overflow-y: auto;
    margin: 0; padding: 0; color: var(--text-primary);
    border: 1px solid var(--glass-border); border-radius: var(--radius-xl);
    background: color-mix(in srgb, var(--bg-elev) 92%, transparent);
    backdrop-filter: blur(24px); -webkit-backdrop-filter: blur(24px);
    box-shadow: 0 24px 80px rgb(0 0 0 / .48), var(--glass-highlight);
    animation: hh-menu-enter .2s cubic-bezier(.22, 1, .36, 1);
  }
  @keyframes hh-menu-enter {
    from { opacity: 0; transform: translateY(-8px) scale(.985); }
    to { opacity: 1; transform: none; }
  }
  .hh-menu-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 18px 20px 15px; border-bottom: 1px solid var(--border);
  }
  .hh-menu-kicker { color: var(--accent); font: 650 .62rem/1 var(--font-mono); letter-spacing: .16em; }
  .hh-menu-head h2 { margin: 7px 0 0; font: 750 1.25rem/1 var(--font-display); }
  .hh-menu-close {
    display: grid; place-items: center; width: 40px; height: 40px;
    border: 1px solid var(--border); border-radius: var(--radius-md);
    background: var(--bg-card); color: var(--text-secondary); cursor: pointer;
  }
  .hh-menu-close:hover { border-color: var(--accent-ring); color: var(--text-primary); }
  .hh-menu-groups { display: grid; gap: 16px; padding: 16px 20px 10px; }
  .hh-menu-group h3 {
    margin: 0 0 8px; color: var(--text-muted);
    font: 700 .66rem/1 var(--font-mono); letter-spacing: .14em;
  }
  .hh-menu-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
  .hh-menu-grid > button {
    min-height: 68px; display: grid; grid-template-columns: 34px minmax(0, 1fr);
    align-items: center; gap: 9px; padding: 10px;
    border: 1px solid var(--border); border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--bg-card) 72%, transparent);
    color: var(--text-secondary); text-align: left; cursor: pointer;
    transition: transform .16s ease, border-color .16s ease, background .16s ease, color .16s ease;
  }
  .hh-menu-grid > button:focus-visible,
  .hh-menu-close:focus-visible,
  .hh-import-btn:focus-visible,
  .hh-system:focus-visible {
    outline: 2px solid var(--accent-hi);
    outline-offset: 2px;
  }
  .hh-menu-grid > button:hover, .hh-menu-grid > button.active {
    border-color: var(--accent-ring); background: var(--accent-lo); color: var(--text-primary);
    transform: translateY(-1px);
  }
  .hh-menu-glyph {
    display: grid; place-items: center; width: 34px; height: 34px;
    border: 1px solid var(--border); border-radius: 10px; color: var(--accent-hi);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .hh-menu-item-copy { min-width: 0; display: grid; gap: 5px; }
  .hh-menu-item-copy strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: .8rem; }
  .hh-menu-item-copy small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-muted); font-size: .62rem; }
  .hh-menu-foot {
    display: flex; justify-content: flex-end; gap: 12px; padding: 10px 20px 15px;
    color: var(--text-muted); font-size: .68rem;
  }
  .hh-menu-foot .k { margin-right: 4px; }

  /* ---- 主区 ---- */
  .hh-main {
    position: relative; z-index: 1;
    flex: 1; min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 18px;
    padding: 6px var(--hh-pad) 10px;
  }
  .hh-stage { height: 100%; min-height: 0; display: grid; grid-template-rows: minmax(0, 1fr) auto; gap: 10px; overflow: hidden; animation: hh-enter .32s cubic-bezier(.22, 1, .36, 1); }
  @keyframes hh-enter {
    from { opacity: 0; transform: translateY(10px); }
    to   { opacity: 1; transform: none; }
  }

  .hh-cell-placeholder { display: grid; width: 100%; height: 100%; place-items: center; background: linear-gradient(135deg, var(--bg-card), var(--bg-elev)); color: var(--text-dim); font: 800 1.5rem/1 var(--font-display); }
  .hh-cell-placeholder.big { font-size: 2.8rem; }
  .hh-chip {
    padding: 4px 10px; border-radius: 99px;
    background: var(--accent-lo); color: var(--accent-hi);
    font: 700 .66rem/1 var(--font-mono);
  }

  /* ---- 底部提示条 ---- */
  .hh-hints {
    position: relative; z-index: 1;
    display: flex; gap: 18px; justify-content: center; align-items: center;
    padding: 8px 16px calc(8px + env(safe-area-inset-bottom));
    font-size: .7rem; color: var(--text-muted);
  }
  .hh-hints .k {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 20px; padding: 2px 7px; margin-right: 5px;
    border-radius: 6px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font: 600 .66rem/1.4 var(--font-mono);
  }

  /* ---- 空态 ---- */
  .hh-empty {
    position: relative; z-index: 1;
    flex: 1;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 14px; text-align: center; padding: 40px;
  }

  /* 竖屏回退：详情面板压到网格下方 */
  @media (orientation: portrait) {
    .hh-menu-panel { left: var(--hh-pad); right: var(--hh-pad); width: auto; max-height: calc(100% - 100px); }
    .hh-menu-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @media (orientation: landscape) and (max-width: 1180px) {
    .hh-topbar { gap: 9px; }
    .hh-system { gap: 5px; padding: 7px 9px; font-size: .72rem; }
    .hh-system-count { padding-inline: 5px; font-size: .6rem; }
    .hh-status { gap: 6px; }
    .hh-clock { display: none; }
    .hh-xmb-page .hh-status .hh-profile { display: none; }
    .hh-xmb-page .hh-xmb-context { display: flex; align-items: center; gap: 4px; min-width: 0; padding-left: 8px; border-left-width: 1px; font-size: .5rem; letter-spacing: .08em; }
    .hh-xmb-page .hh-xmb-context b { font-size: .62rem; }
    .hh-xmb-page .hh-status .hh-import-btn { width: auto; min-width: 36px; justify-content: center; padding-inline: 7px; font-size: .62rem; }
    .hh-xmb-page .hh-status .hh-import-btn span { display: inline; }
    .hh-xmb-page .hh-status > .hh-import-btn:not(.hh-home-shortcut) { width: 36px; padding-inline: 6px; }
    .hh-xmb-page .hh-status > .hh-import-btn:not(.hh-home-shortcut) span { display: none; }
    .hh-main { grid-template-columns: minmax(0, 1fr) 270px; gap: 12px; }
    .hh-menu-panel { right: max(10px, env(safe-area-inset-right)); left: auto; width: min(780px, calc(100vw - 20px)); }
    .hh-menu-grid { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  }
  @media (orientation: landscape) and (max-height: 620px) {
    .hh-topbar { padding-block: max(8px, env(safe-area-inset-top)) 5px; }
    .hh-logo { font-size: 1.08rem; }
    .hh-mode { font-size: .5rem; }
    .hh-main { padding-top: 3px; padding-bottom: 5px; }
    .hh-stage { gap: 6px; }
    .hh-xmb-selection { grid-template-columns: minmax(120px, 20%) minmax(0, 1fr) 126px; min-height: 150px; max-height: min(230px, 46dvh); padding-block: 10px; gap: 14px; }
    .hh-xmb-art-frame { width: min(136px, 100%); height: min(176px, 38dvh); }
    .hh-hints { gap: 10px; padding-block: 5px; font-size: .62rem; }
    .hh-hints .k { min-width: 18px; padding-inline: 5px; }
    .hh-menu-panel {
      top: max(8px, env(safe-area-inset-top));
      right: max(8px, env(safe-area-inset-right));
      bottom: max(8px, env(safe-area-inset-bottom));
      left: max(8px, env(safe-area-inset-left));
      width: auto;
      height: calc(100dvh - 16px);
      min-height: 0;
      max-height: none;
      border-radius: var(--radius-lg);
    }
    .hh-menu-head { padding: 12px 14px 10px; }
    .hh-menu-head h2 { margin-top: 5px; font-size: 1.08rem; }
    .hh-menu-groups { gap: 10px; padding: 10px 14px 5px; }
    .hh-menu-group h3 { margin-bottom: 5px; }
    .hh-menu-grid { gap: 6px; }
    .hh-menu-grid > button { min-height: 50px; grid-template-columns: 28px minmax(0, 1fr); gap: 7px; padding: 7px; }
    .hh-menu-glyph { width: 28px; height: 28px; border-radius: 8px; }
    .hh-menu-glyph :global(svg) { width: 16px; height: 16px; }
    .hh-menu-item-copy { gap: 2px; }
    .hh-menu-item-copy strong { font-size: .72rem; }
    .hh-menu-item-copy small { font-size: .54rem; }
    .hh-menu-foot { gap: 9px; padding: 6px 14px 9px; font-size: .6rem; }
  }
  /* 某些 Android WebView 以物理横屏宽度计算 CSS：首列必须按导航内容保留宽度。 */
  @media (orientation: landscape) {
    .hh-brand { min-width: max-content; }
  }
  /* AIR_X 等窄横屏把主导航独占首行，避免被中间的内容分区轨道挤出视口。 */
  @media (orientation: landscape) and (max-width: 900px) {
    .hh-xmb-page .hh-xmb-topbar { grid-template-columns: max-content minmax(0, 1fr) max-content; }
    .hh-brand { min-width: 0; }
    .hh-xmb-page .hh-status { grid-column: auto; grid-row: auto; }
    .hh-xmb-page .hh-xmb-category-track { justify-content: start; }
  }
  @media (max-width: 720px) {
    .hh-xmb-page .hh-xmb-topbar { grid-template-columns: max-content minmax(0, 1fr) max-content; }
    .hh-xmb-page .hh-status { grid-column: auto; grid-row: auto; }
    .hh-xmb-page .hh-xmb-category-track { justify-content: start; }
    .hh-main { grid-template-columns: 1fr; grid-template-rows: minmax(0, 1fr); }
  }
  @media (prefers-reduced-motion: reduce) {
    .hh-system, .hh-stage, .hh-menu-panel { transition: none; animation: none; }
  }

  /* ========================================================================
     PSP / XMB 首页终版：频道是一级焦点，内容是二级焦点。
     这组规则只挂在 .hh-xmb-page 下，不改变桌面首页或媒体详情页。
     ======================================================================== */
  .hh-xmb-page {
    --xmb-line: color-mix(in srgb, var(--text-primary) 16%, transparent);
    --xmb-panel: color-mix(in srgb, var(--bg-void) 72%, var(--bg-card));
    --xmb-highlight: color-mix(in srgb, var(--accent-hi) 90%, white 10%);
    background: #07090d;
  }
  .hh-xmb-page :global(.hh-backdrop) { z-index: 0; opacity: .82; }
  /* AIR_X 的 WebView 在全屏背景上做实时大半径 blur 会明显拖慢首帧；
     用放大、饱和度和已有 wash 叠层保留影院氛围，避免持续 GPU 重采样。 */
  .hh-xmb-page :global(.hh-artwork-stage__image) { opacity: .48; filter: saturate(1.12); transform: scale(1.025); }
  .hh-xmb-page :global(.hh-artwork-stage__wash) {
    background:
      linear-gradient(90deg, rgb(5 7 12 / .94), rgb(5 7 12 / .54) 48%, rgb(5 7 12 / .78)),
      linear-gradient(180deg, rgb(5 7 12 / .7), rgb(5 7 12 / .95));
  }
  .hh-xmb-vignette, .hh-xmb-scanlines {
    position: absolute; inset: 0; z-index: 1; pointer-events: none;
  }
  .hh-xmb-vignette { background: radial-gradient(ellipse at 58% 46%, transparent 0 25%, rgb(0 0 0 / .2) 72%, rgb(0 0 0 / .52)); }
  .hh-xmb-scanlines { opacity: .065; background: repeating-linear-gradient(180deg, transparent 0 3px, rgb(255 255 255 / .045) 3px 4px); }

  .hh-xmb-page .hh-xmb-topbar {
    position: relative; z-index: 4; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center;
    gap: clamp(8px, 1.4vw, 20px); flex: 0 0 auto; min-height: 62px; padding: max(12px, env(safe-area-inset-top)) var(--hh-pad) 10px;
    border-bottom: 1px solid var(--xmb-line);
    background: linear-gradient(180deg, rgb(5 7 12 / .74), rgb(5 7 12 / .2));
  }
  .hh-xmb-page .hh-brand { align-items: center; gap: clamp(10px, 1.4vw, 22px); }
  .hh-xmb-page .hh-brand-home { min-width: 106px; }
  .hh-xmb-page .hh-logo { font-size: 1.35rem; letter-spacing: .06em; text-shadow: 0 0 18px color-mix(in srgb, var(--accent) 45%, transparent); }
  .hh-xmb-page .hh-mode { letter-spacing: .22em; }
  .hh-xmb-context { display: grid; gap: 4px; min-width: 110px; padding-left: 20px; border-left: 1px solid var(--xmb-line); color: var(--text-muted); font: 600 .62rem/1 var(--font-mono); letter-spacing: .12em; }
  .hh-xmb-context b { color: var(--text-primary); font: 750 .86rem/1 var(--font-ui); letter-spacing: 0; }
  .hh-xmb-page .hh-status { grid-column: auto; grid-row: auto; min-width: max-content; flex: 0 0 auto; gap: 12px; }
  .hh-profile { color: var(--text-muted); font: 700 .58rem/1 var(--font-mono); letter-spacing: .12em; }
  .hh-xmb-page .hh-import-btn { min-height: 38px; border-radius: 8px; background: rgb(255 255 255 / .06); }

  .hh-xmb-page .hh-xmb-categories {
    position: relative; z-index: 3; grid-column: auto; grid-row: auto; display: block; min-width: 0; overflow: hidden; padding: 0;
    border: 0; mask-image: none; background: transparent;
  }
  .hh-xmb-category-track { display: flex; align-items: center; justify-content: center; gap: clamp(3px, .7vw, 10px); min-width: 0; width: 100%; overflow-x: auto; scrollbar-width: none; }
  .hh-xmb-category-track::-webkit-scrollbar { display: none; }
  .hh-xmb-category-reveal {
    position: relative; z-index: 3; display: flex; align-items: center; justify-content: center; gap: 7px;
    min-width: 68px; min-height: 36px; padding: 3px 8px; border: 1px solid var(--xmb-line); border-radius: 7px;
    background: rgb(2 4 7 / .88); color: var(--text-muted); font: 700 .58rem/1 var(--font-mono); letter-spacing: .1em; cursor: pointer;
  }
  .hh-xmb-category-reveal b { color: var(--accent-hi); font-weight: 800; }
  .hh-xmb-category-reveal:focus-visible { outline: 2px solid var(--accent-hi); outline-offset: -2px; }
  .hh-xmb-page .hh-xmb-category {
    position: relative; display: grid; grid-template-columns: 30px minmax(0, auto); grid-template-rows: auto auto; align-items: center; justify-items: start; column-gap: 6px; row-gap: 2px;
    width: clamp(68px, 7.2vw, 88px); min-width: 68px; min-height: 44px; padding: 5px 6px; border: 1px solid transparent; border-radius: 8px;
    background: transparent; color: var(--text-muted); font-size: .68rem; cursor: pointer;
    transition: transform .2s ease, border-color .2s ease, background .2s ease, color .2s ease, box-shadow .2s ease;
  }
  .hh-xmb-page .hh-xmb-category:hover { color: var(--text-primary); border-color: var(--xmb-line); background: rgb(255 255 255 / .035); }
  .hh-xmb-page .hh-xmb-category.active {
    color: var(--text-primary); border-color: color-mix(in srgb, var(--accent) 72%, transparent);
    background: linear-gradient(180deg, color-mix(in srgb, var(--accent) 25%, transparent), color-mix(in srgb, var(--bg-card) 62%, transparent));
    box-shadow: 0 10px 30px rgb(0 0 0 / .2), 0 0 0 1px color-mix(in srgb, var(--accent-hi) 24%, transparent) inset;
    transform: translateY(-3px) scale(1.035);
  }
  .hh-xmb-category.active::after { position: absolute; right: 8px; bottom: -6px; left: 8px; height: 2px; content: ""; background: var(--xmb-highlight); box-shadow: 0 0 12px var(--accent); }
  .hh-xmb-category-icon { display: grid; place-items: center; grid-row: 1 / -1; width: 30px; height: 30px; border: 1px solid var(--xmb-line); border-radius: 7px; background: rgb(4 6 10 / .46); color: var(--text-secondary); font: 800 .92rem/1 var(--font-display); }
  .hh-xmb-category.active .hh-xmb-category-icon { border-color: color-mix(in srgb, var(--accent-hi) 70%, transparent); background: color-mix(in srgb, var(--accent) 22%, var(--bg-elev)); color: var(--accent-hi); box-shadow: 0 0 18px color-mix(in srgb, var(--accent) 22%, transparent); }
  .hh-xmb-page .hh-xmb-category .hh-system-label { overflow: hidden; max-width: 100%; text-overflow: ellipsis; white-space: nowrap; }
  .hh-xmb-page .hh-xmb-category .hh-system-count { padding: 2px 5px; background: rgb(255 255 255 / .08); color: var(--text-muted); font-size: .52rem; }
  .hh-xmb-page .hh-xmb-category.active .hh-system-count { background: var(--accent); color: #fff; }

  .hh-game-platforms {
    position: relative; z-index: 3; display: flex; align-items: center; gap: 8px; min-width: 0; flex: 0 0 auto;
    padding: 5px var(--hh-pad); border-bottom: 1px solid var(--xmb-line); background: rgb(5 7 12 / .46);
  }
  .hh-game-platforms__hint { display: inline-flex; align-items: center; gap: 5px; flex: 0 0 auto; color: var(--text-muted); font: 700 8px/1 var(--font-mono); letter-spacing: .12em; }
  .hh-game-platforms__hint b { display: grid; min-width: 25px; height: 20px; place-items: center; border: 1px solid color-mix(in srgb, var(--accent) 44%, transparent); border-radius: 5px; color: var(--accent-hi); background: rgb(255 255 255 / .05); }
  .hh-game-platforms__hint i { width: 14px; height: 1px; background: var(--xmb-line); }
  .hh-game-platforms__track { display: flex; min-width: 0; flex: 1; gap: 5px; overflow-x: auto; scrollbar-width: none; scroll-snap-type: x proximity; }
  .hh-game-platforms__track::-webkit-scrollbar { display: none; }
  .hh-game-platforms__track button { display: inline-flex; align-items: center; gap: 5px; min-height: 28px; padding: 0 9px; border: 1px solid transparent; border-radius: 5px; background: transparent; color: var(--text-muted); white-space: nowrap; cursor: pointer; scroll-snap-align: center; }
  .hh-game-platforms__track button strong { font-size: .66rem; }
  .hh-game-platforms__track button small { min-width: 14px; padding: 2px 4px; border-radius: 4px; background: rgb(255 255 255 / .06); color: var(--text-muted); font: 700 .55rem/1 var(--font-mono); }
  .hh-game-platforms__track button.active { border-color: color-mix(in srgb, var(--accent) 68%, transparent); background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--text-primary); box-shadow: 0 0 16px color-mix(in srgb, var(--accent) 15%, transparent); }
  .hh-game-platforms__track button.active small { background: var(--accent); color: #fff; }
  .hh-game-stage { flex: 1; min-height: 0; overflow: hidden; }

  .hh-xmb-page .hh-xmb-main {
    position: relative; z-index: 2; display: block; flex: 1; min-height: 0; padding: 10px var(--hh-pad) 5px;
  }
  .hh-xmb-page .hh-xmb-stage { display: flex; flex-direction: column; gap: 10px; height: 100%; min-height: 0; animation: hh-xmb-enter .26s ease-out; }
  @keyframes hh-xmb-enter { from { opacity: .65; transform: translateY(5px); } to { opacity: 1; transform: none; } }
  .hh-xmb-selection {
    display: grid; grid-template-columns: minmax(132px, 18%) minmax(0, 1fr) 154px; align-items: center; gap: clamp(14px, 2.5vw, 30px);
    flex: 1 1 auto; min-height: 190px; max-height: min(344px, 55dvh); padding: 14px clamp(14px, 2.6vw, 34px);
    border-top: 1px solid var(--xmb-line); border-bottom: 1px solid var(--xmb-line);
    background: linear-gradient(100deg, rgb(8 10 16 / .72), color-mix(in srgb, var(--accent) 8%, transparent) 58%, rgb(8 10 16 / .38));
  }
  .hh-xmb-selected-art { display: grid; justify-items: center; align-content: center; gap: 8px; min-width: 0; }
  .hh-xmb-art-frame { position: relative; display: grid; place-items: center; width: min(150px, 100%); height: min(220px, 43dvh); overflow: hidden; border: 1px solid color-mix(in srgb, var(--accent-hi) 70%, transparent); background: linear-gradient(145deg, rgb(255 255 255 / .1), rgb(0 0 0 / .26)); box-shadow: 7px 7px 0 rgb(0 0 0 / .16), 0 0 28px color-mix(in srgb, var(--accent) 22%, transparent); }
  .hh-xmb-art-frame::before { position: absolute; inset: 5px; border: 1px solid rgb(255 255 255 / .16); content: ""; pointer-events: none; }
  .hh-xmb-art-frame img { width: 100%; height: 100%; object-fit: cover; }
  .hh-xmb-art-gloss { position: absolute; inset: 0; background: linear-gradient(125deg, rgb(255 255 255 / .14), transparent 22% 72%, rgb(0 0 0 / .22)); pointer-events: none; }
  .hh-xmb-index { color: var(--text-muted); font: 700 .58rem/1 var(--font-mono); letter-spacing: .08em; }
  .hh-xmb-index b { color: var(--accent-hi); font-size: .84rem; }
  .hh-xmb-copy { display: flex; min-width: 0; flex-direction: column; align-items: flex-start; gap: 8px; }
  .hh-xmb-copy-kicker { display: flex; align-items: center; gap: 8px; color: var(--accent-hi); font: 700 .62rem/1 var(--font-mono); letter-spacing: .15em; }
  .hh-xmb-copy-kicker i { width: 22px; height: 1px; background: var(--accent); }
  .hh-xmb-copy-kicker small { color: var(--text-muted); font-size: .55rem; letter-spacing: .1em; }
  .hh-xmb-copy h1 { max-width: 100%; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); font: 850 clamp(1.35rem, 3vw, 2.25rem)/1.08 var(--font-display); letter-spacing: -.045em; text-shadow: 0 3px 20px rgb(0 0 0 / .26); }
  .hh-xmb-subtitle { max-width: 650px; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-secondary); font-size: .78rem; }
  .hh-xmb-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; min-height: 20px; color: var(--text-muted); font: 600 .6rem/1.3 var(--font-mono); }
  .hh-xmb-meta > span:not(.hh-chip) { padding-left: 8px; border-left: 1px solid var(--xmb-line); }
  .hh-xmb-page .hh-chip { padding: 4px 8px; }
  .hh-xmb-progress { display: grid; grid-template-columns: auto auto; align-items: center; gap: 4px 8px; width: min(330px, 100%); color: var(--text-muted); font: 600 .58rem/1 var(--font-mono); }
  .hh-xmb-progress b { color: var(--text-primary); }
  .hh-xmb-progress i { grid-column: 1 / -1; display: block; height: 3px; overflow: hidden; background: rgb(255 255 255 / .14); }
  .hh-xmb-progress em { display: block; height: 100%; background: var(--accent-hi); box-shadow: 0 0 12px var(--accent); }
  .hh-xmb-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 2px; }
  .hh-xmb-action { display: inline-flex; align-items: center; gap: 8px; min-height: 40px; padding: 0 13px; border: 1px solid var(--xmb-line); border-radius: 6px; background: rgb(255 255 255 / .06); color: var(--text-primary); font: 700 .72rem/1 var(--font-ui); cursor: pointer; }
  .hh-xmb-action.primary { border-color: var(--accent); background: var(--accent); color: #fff; box-shadow: 0 6px 18px color-mix(in srgb, var(--accent) 22%, transparent); }
  .hh-xmb-action:hover, .hh-xmb-action:focus-visible { border-color: var(--accent-hi); }
  .hh-button-letter { display: grid; place-items: center; min-width: 20px; height: 20px; border: 1px solid currentColor; border-radius: 50%; font: 700 .56rem/1 var(--font-mono); }
  .hh-xmb-inspector { display: grid; align-content: center; justify-items: end; gap: 7px; min-width: 0; padding-left: 18px; border-left: 1px solid var(--xmb-line); text-align: right; }
  .hh-xmb-inspector-label { color: var(--text-muted); font: 700 .54rem/1 var(--font-mono); letter-spacing: .13em; }
  .hh-xmb-inspector strong { color: var(--accent-hi); font: 800 2.7rem/.85 var(--font-display); }
  .hh-xmb-inspector-line { width: 38px; height: 2px; background: var(--accent); }
  .hh-xmb-inspector small { max-width: 150px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-muted); font: 600 .56rem/1.2 var(--font-mono); }
  .hh-xmb-rail-wrap { flex: 0 0 auto; min-height: 0; }
  .hh-xmb-page :global(.hh-rail) { min-width: 0; }
  .hh-xmb-page :global(.hh-rail__header) { gap: 9px; padding: 0 2px 6px; border-bottom: 1px solid var(--xmb-line); }
  .hh-xmb-page :global(.hh-rail__header h2) { font-size: .9rem; }
  .hh-xmb-page :global(.hh-rail__header small) { font-size: .54rem; }
  .hh-xmb-page :global(.hh-rail__track) { gap: 10px; padding: 8px 2px 4px; }
  .hh-xmb-page :global(.hh-rail__card) { flex-basis: clamp(78px, 10vw, 122px); gap: 4px; }
  .hh-xmb-page :global(.hh-rail__cover) { border-width: 1px; }
  .hh-xmb-page :global(.hh-rail__card.active .hh-rail__cover) { border-width: 2px; box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent), 0 7px 18px rgb(0 0 0 / .24); }
  .hh-xmb-page :global(.hh-rail__card strong) { font-size: .65rem; }
  .hh-xmb-page :global(.hh-rail__card small) { font-size: .55rem; }
  .hh-xmb-page .hh-xmb-hints { position: relative; z-index: 3; justify-content: flex-end; gap: 13px; min-height: 30px; border-top: 1px solid var(--xmb-line); background: rgb(4 6 10 / .28); }
  .hh-xmb-page .hh-xmb-hints .k { min-width: 22px; padding-inline: 5px; border-color: color-mix(in srgb, var(--accent) 32%, var(--border)); background: rgb(255 255 255 / .06); color: var(--text-secondary); }
  .hh-xmb-page .hh-empty { position: relative; z-index: 2; min-height: 0; padding: 24px var(--hh-pad); }
  .hh-xmb-page :global(.hh-state-panel) { border-color: var(--xmb-line); background: linear-gradient(105deg, rgb(10 13 20 / .8), color-mix(in srgb, var(--accent) 10%, transparent)); box-shadow: 0 18px 48px rgb(0 0 0 / .2); }
  .hh-xmb-welcome-main { display: flex !important; align-items: center; justify-content: center; }
  .hh-xmb-welcome { display: grid; grid-template-columns: minmax(180px, 36%) minmax(0, 1fr); width: min(1040px, 100%); min-height: min(300px, 52dvh); overflow: hidden; border: 1px solid var(--xmb-line); background: linear-gradient(112deg, rgb(7 10 16 / .88), color-mix(in srgb, var(--accent) 10%, rgb(7 10 16 / .82))); box-shadow: 0 24px 72px rgb(0 0 0 / .28); }
  .hh-xmb-welcome-art { position: relative; min-height: 100%; overflow: hidden; border-right: 1px solid var(--xmb-line); background: #0b0d13; }
  .hh-xmb-welcome-art img { width: 100%; height: 100%; object-fit: cover; opacity: .75; filter: saturate(1.18); }
  .hh-xmb-welcome-art::after { position: absolute; inset: 0; content: ""; background: linear-gradient(90deg, rgb(7 9 14 / .06), rgb(7 9 14 / .76)), linear-gradient(180deg, transparent 32%, rgb(7 9 14 / .82)); }
  .hh-xmb-welcome-art-copy { position: absolute; right: 18px; bottom: 18px; left: 18px; z-index: 1; display: grid; gap: 3px; }
  .hh-xmb-welcome-art-copy span { color: var(--accent-hi); font: 800 .62rem/1 var(--font-mono); letter-spacing: .2em; }
  .hh-xmb-welcome-art-copy b { color: var(--text-primary); font: 800 1.6rem/.9 var(--font-display); letter-spacing: -.04em; }
  .hh-xmb-welcome-art-copy small { color: var(--text-muted); font: 600 .5rem/1 var(--font-mono); letter-spacing: .12em; }
  .hh-xmb-welcome-copy { display: flex; min-width: 0; flex-direction: column; align-items: flex-start; justify-content: center; gap: 12px; padding: clamp(20px, 4vw, 44px); }
  .hh-xmb-welcome-copy h1 { max-width: 660px; margin: 0; color: var(--text-primary); font: 850 clamp(1.5rem, 3vw, 2.5rem)/1.1 var(--font-display); letter-spacing: -.055em; }
  .hh-xmb-welcome-copy > p { max-width: 620px; margin: 0; color: var(--text-secondary); font-size: .82rem; line-height: 1.75; }
  .hh-xmb-welcome-actions { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; width: 100%; max-width: 690px; }
  .hh-xmb-welcome-actions button { display: flex; align-items: center; gap: 9px; min-width: 0; min-height: 58px; padding: 9px; border: 1px solid var(--xmb-line); background: rgb(255 255 255 / .045); color: var(--text-secondary); text-align: left; cursor: pointer; transition: border-color .18s ease, background .18s ease, transform .18s ease, color .18s ease; }
  .hh-xmb-welcome-actions button:hover, .hh-xmb-welcome-actions button.active, .hh-xmb-welcome-actions button:focus-visible { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--text-primary); transform: translateY(-2px); }
  .hh-xmb-welcome-actions button:nth-child(1) { grid-column: span 2; }
  .hh-xmb-welcome-icon { display: grid; place-items: center; flex: 0 0 32px; width: 32px; height: 32px; border: 1px solid var(--xmb-line); color: var(--accent-hi); }
  .hh-xmb-welcome-actions button > span:last-child { display: grid; min-width: 0; gap: 4px; }
  .hh-xmb-welcome-actions b, .hh-xmb-welcome-actions small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hh-xmb-welcome-actions b { font-size: .72rem; }
  .hh-xmb-welcome-actions small { color: var(--text-muted); font-size: .55rem; }
  .hh-xmb-welcome-tip { color: var(--text-muted); font: 600 .58rem/1.3 var(--font-mono); }
  .hh-xmb-welcome-tip b { color: var(--accent-hi); }
  .hh-xmb-welcome-tip span { padding-inline: 5px; color: var(--xmb-line); }

  @media (orientation: landscape) and (max-width: 900px) {
    .hh-xmb-page .hh-xmb-topbar { min-height: 52px; gap: 12px; padding-block: max(8px, env(safe-area-inset-top)) 7px; }
    .hh-xmb-page .hh-brand { gap: 12px; }
    .hh-xmb-page .hh-brand-home { min-width: 94px; }
    .hh-xmb-context { min-width: 82px; padding-left: 12px; }
    .hh-profile, .hh-clock { display: none; }
    .hh-xmb-page .hh-status { gap: 7px; }
    .hh-xmb-page .hh-import-btn { min-height: 32px; padding-inline: 9px; }
    .hh-xmb-page .hh-home-shortcut { padding-inline: 8px; }
    .hh-xmb-page .hh-xmb-categories { padding-block: 8px 9px; }
    .hh-xmb-category-track { gap: 6px; }
    .hh-xmb-page .hh-xmb-category { width: 68px; min-width: 68px; min-height: 64px; grid-template-rows: 34px auto auto; gap: 3px; padding: 5px 4px; font-size: .63rem; }
    .hh-xmb-category-icon { width: 34px; height: 34px; }
    .hh-xmb-category-icon :global(svg) { width: 18px; height: 18px; }
    .hh-xmb-page .hh-xmb-main { padding-top: 6px; }
    .hh-xmb-page .hh-xmb-stage { gap: 6px; }
    .hh-xmb-selection { grid-template-columns: 80px minmax(0, 1fr); gap: 14px; min-height: 132px; max-height: 196px; padding: 10px 14px; }
    .hh-xmb-art-frame { width: 74px; height: 108px; }
    .hh-xmb-copy { gap: 5px; }
    .hh-xmb-copy h1 { font-size: 1.28rem; }
    .hh-xmb-subtitle { font-size: .66rem; }
    .hh-xmb-actions { gap: 6px; }
    .hh-xmb-action { min-height: 34px; padding-inline: 9px; font-size: .64rem; }
    .hh-xmb-inspector { display: none; }
    .hh-xmb-welcome { grid-template-columns: 140px minmax(0, 1fr); min-height: 166px; }
    .hh-xmb-welcome-copy { gap: 7px; padding: 14px 18px; }
    .hh-xmb-welcome-copy h1 { font-size: 1.34rem; }
    .hh-xmb-welcome-copy > p { font-size: .66rem; line-height: 1.5; }
    .hh-xmb-welcome-actions { gap: 5px; }
    .hh-xmb-welcome-actions button { min-height: 42px; padding: 6px; }
    .hh-xmb-welcome-icon { flex-basis: 24px; width: 24px; height: 24px; }
    .hh-xmb-welcome-icon :global(svg) { width: 14px; height: 14px; }
    .hh-xmb-welcome-actions b { font-size: .59rem; }
    .hh-xmb-welcome-actions small { font-size: .49rem; }
    .hh-xmb-welcome-tip { font-size: .5rem; }
    .hh-xmb-page :global(.hh-rail__track) { padding-block: 6px 2px; }
    .hh-xmb-page :global(.hh-rail__card) { flex-basis: 70px; }
    .hh-xmb-page :global(.hh-rail__card strong) { font-size: .56rem; }
    .hh-xmb-page .hh-xmb-hints { gap: 8px; min-height: 25px; padding-block: 3px; font-size: .56rem; }
    .hh-xmb-page .hh-xmb-hints .k { min-width: 18px; padding-inline: 4px; }
  }
  @media (max-width: 620px) {
    .hh-xmb-context { display: none; }
    .hh-xmb-page .hh-brand-home { min-width: 76px; }
    .hh-xmb-page .hh-logo { font-size: 1.12rem; }
    .hh-xmb-page .hh-mode { font-size: .48rem; }
    .hh-xmb-page .hh-home-shortcut span { display: none; }
    .hh-xmb-page .hh-home-shortcut { width: 34px; justify-content: center; padding-inline: 0; }
    .hh-xmb-selection { grid-template-columns: 68px minmax(0, 1fr); gap: 10px; }
    .hh-xmb-art-frame { width: 62px; height: 92px; }
    .hh-xmb-meta > span:not(.hh-chip) { display: none; }
    .hh-xmb-page :global(.hh-rail__header small) { display: none; }
    .hh-xmb-page .hh-xmb-hints span:nth-child(4) { display: none; }
    .hh-xmb-welcome { grid-template-columns: 92px minmax(0, 1fr); }
    .hh-xmb-welcome-art-copy { right: 8px; bottom: 8px; left: 8px; }
    .hh-xmb-welcome-art-copy b { font-size: 1rem; }
    .hh-xmb-welcome-copy { padding: 10px; }
    .hh-xmb-welcome-copy h1 { font-size: 1.05rem; }
    .hh-xmb-welcome-copy > p { display: none; }
    .hh-xmb-welcome-actions { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .hh-xmb-welcome-actions button:nth-child(1) { grid-column: span 2; }
    .hh-xmb-welcome-actions button { min-height: 36px; }
    .hh-xmb-welcome-actions small { display: none; }
    .hh-xmb-welcome-tip { display: none; }
  }
  @media (prefers-reduced-motion: reduce) {
    .hh-xmb-page .hh-xmb-category, .hh-xmb-page .hh-xmb-stage { transition: none; animation: none; }
  }

  /* AIR_X 808×454 终版横屏基线：频道保持一行，内容舞台获得优先空间。 */
  @media (orientation: landscape) and (max-width: 900px) {
    .hh-xmb-page .hh-xmb-topbar {
      min-height: 56px;
      padding-block: max(5px, env(safe-area-inset-top)) 5px;
      gap: 7px;
    }
    .hh-xmb-page .hh-xmb-categories { padding-block: 0; }
    .hh-xmb-page .hh-xmb-category-track { justify-content: center; gap: 3px; }
    .hh-xmb-page .hh-xmb-category {
      grid-template-columns: 25px minmax(0, auto);
      grid-template-rows: auto auto;
      width: auto;
      min-width: 57px;
      min-height: 44px;
      padding: 4px 5px;
      column-gap: 5px;
      row-gap: 2px;
    }
    .hh-xmb-category-icon { width: 25px; height: 25px; border-radius: 6px; }
    .hh-xmb-page .hh-xmb-category-icon :global(svg) { width: 15px; height: 15px; }
    .hh-xmb-page .hh-xmb-category .hh-system-label { font-size: .6rem; }
    .hh-xmb-page .hh-xmb-category .hh-system-count { padding: 1px 4px; font-size: .48rem; }
    .hh-xmb-page .hh-status { gap: 5px; }
    .hh-xmb-page .hh-status .hh-import-btn { min-height: 34px; padding-inline: 7px; font-size: .58rem; }
    .hh-xmb-page .hh-status > .hh-import-btn:not(.hh-home-shortcut) { width: 34px; }
    .hh-game-platforms { padding-block: 3px; }
    .hh-game-platforms__track button { min-height: 26px; }
    .hh-xmb-page .hh-xmb-main { padding-top: 5px; }
    .hh-xmb-selection {
      grid-template-columns: 126px minmax(0, 1fr);
      min-height: 174px;
      max-height: min(202px, 48dvh);
      gap: 14px;
      padding-block: 9px;
    }
    .hh-xmb-art-frame { width: 112px; height: 162px; }
    .hh-xmb-page :global(.hh-rail__track) { padding-block: 5px 2px; }
    .hh-xmb-page :global(.hh-rail__card) { flex-basis: 82px; }
  }
</style>
