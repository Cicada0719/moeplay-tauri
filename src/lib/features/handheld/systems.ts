// 掌机视图：把游戏库按「系统/平台」分组（ES-DE 的 system 维度）。
// 纯函数便于单测；HandheldPage 只负责渲染与交互。

import { DOCK_ITEMS, TOOL_ITEMS, type NavItem } from "../../nav";

export interface HandheldGameLike {
  id: string;
  name?: string | null;
  game_type?: string | null;
  favorite?: boolean | null;
  hidden?: boolean | null;
}

export interface HandheldSystem<T extends HandheldGameLike = HandheldGameLike> {
  /** 规范化平台 id（小写），如 psp / gba / windows */
  id: string;
  /** 展示名 */
  label: string;
  games: T[];
}

/** 平台 id → 中文/常用展示名；未知平台原样大写展示。 */
const PLATFORM_LABELS: Record<string, string> = {
  psp: "PSP",
  ps1: "PlayStation",
  ps2: "PlayStation 2",
  psvita: "PS Vita",
  gamecube: "GameCube",
  wii: "Wii",
  switch: "Switch",
  nes: "FC / NES",
  snes: "SFC / SNES",
  n64: "N64",
  gb: "Game Boy",
  gbc: "Game Boy Color",
  gba: "GBA",
  nds: "NDS",
  "3ds": "3DS",
  md: "MD / Genesis",
  saturn: "SS / Saturn",
  dreamcast: "DC / Dreamcast",
  arcade: "街机",
  bbk: "步步高 BBK",
  j2me: "J2ME",
  msx: "MSX",
  pcengine: "PCE",
  pc98: "PC-98",
  virtualboy: "VB",
  wsc: "WonderSwan",
  ngpc: "NGP Color",
  pokemini: "Pokémon mini",
  gameandwatch: "Game & Watch",
  arduboy: "Arduboy",
  atari7800: "Atari 7800",
  supervision: "Supervision",
  vectrex: "Vectrex",
  zxspectrum: "ZX Spectrum",
  dos: "DOS",
  easyrpg: "EasyRPG",
  pico8: "PICO-8",
  tic80: "TIC-80",
  onscripter: "ONS 文字冒险",
  cdi: "CD-i",
  windows: "PC",
  steam: "Steam",
  epic: "Epic",
};

/**
 * 平台 → 启动模拟器包名优先级（首个命中已安装的即选用）。
 * 与插件 KNOWN_EMULATORS 清单对应；未列出的平台回退 RetroArch。
 */
const EMULATOR_PREFERENCES: Record<string, string[]> = {
  psp: ["org.ppsspp.ppsspp", "org.ppsspp.ppssppgold"],
  ps1: ["com.github.stenzek.duckstation", "com.epsxe.ePSXe", "com.emulator.fpse"],
  ps2: ["xyz.aethersx2.android"],
  nds: ["com.dsemu.drastic", "me.magnum.melonds"],
  n64: ["org.mupen64plusae.v3.fzurita.pro", "org.mupen64plusae.v3.fzurita", "com.nostalgiaemulators.n64"],
  "3ds": ["org.citra.emu", "org.citra.citra_emu", "org.citra.citra_canary", "io.github.lime3ds.android"],
  gba: ["it.dbtecno.pizzaboygbapro", "it.dbtecno.pizzaboygba", "com.fastemulator.gba"],
  gb: ["com.fastemulator.gbc"],
  gbc: ["com.fastemulator.gbc"],
  nes: ["com.nostalgiaemulators.nes1"],
  snes: ["com.nostalgiaemulators.snes1"],
  gamecube: ["org.dolphinemu.dolphinemu", "org.dolphinemu.mmjr"],
  wii: ["org.dolphinemu.dolphinemu", "org.dolphinemu.mmjr"],
  dreamcast: ["com.flycast.emulator", "io.recompiled.redream"],
  psvita: ["org.vita3k.emulator"],
  switch: ["org.yuzu.yuzu_emu", "skyline.emu"],
  onscripter: ["com.onscripter.plus"],
};

/** 全能型模拟器（RetroArch / Lemuroid），作为所有平台的兜底。 */
const FALLBACK_EMULATORS = [
  "com.retroarch.aarch64",
  "com.retroarch",
  "com.swordfish.lemuroid",
];

export interface EmulatorLike {
  packageName: string;
  label: string;
}

export interface EmulatorAssignment {
  /** platform → 选中的模拟器包名（一定存在） */
  assignments: Record<string, string>;
  /** platform → 模拟器显示名 */
  labels: Record<string, string>;
  /** 只能走兜底模拟器的平台 */
  fallback: string[];
  /** 连兜底模拟器都未安装 */
  missing: string[];
}

const OTHER_PLATFORM = "other";

/** game_type 归一化：Emulator: xxx / 大小写 / 空白统一成稳定平台 id。 */
export function normalizePlatform(gameType: string | null | undefined): string {
  const raw = (gameType ?? "").trim().toLowerCase();
  if (!raw) return OTHER_PLATFORM;
  const cleaned = raw.replace(/^emulator:\s*/, "").trim();
  return cleaned || OTHER_PLATFORM;
}

export function platformLabel(id: string): string {
  return PLATFORM_LABELS[id] ?? id.toUpperCase();
}

/**
 * 给一组平台自动分配启动模拟器：
 * 专用模拟器优先（PSP→PPSSPP、NDS→DraStic…），其次 RetroArch 兜底，最后第一个已安装模拟器。
 */
export function resolveEmulatorAssignments(
  platforms: string[],
  installed: EmulatorLike[],
): EmulatorAssignment {
  const byPackage = new Map(installed.map((e) => [e.packageName, e]));
  const fallbackPkg =
    FALLBACK_EMULATORS.find((p) => byPackage.has(p)) ?? installed[0]?.packageName ?? null;

  const assignments: Record<string, string> = {};
  const labels: Record<string, string> = {};
  const fallback: string[] = [];
  const missing: string[] = [];

  for (const platform of platforms) {
    const preferred = (EMULATOR_PREFERENCES[platform] ?? []).find((p) => byPackage.has(p));
    const chosen = preferred ?? fallbackPkg;
    if (!chosen) {
      missing.push(platform);
      continue;
    }
    assignments[platform] = chosen;
    labels[platform] = byPackage.get(chosen)?.label ?? chosen;
    if (!preferred) fallback.push(platform);
  }
  return { assignments, labels, fallback, missing };
}

/**
 * 游戏库 → 系统列表。按游戏数降序、再按名称；收藏优先策略交给排序函数。
 */
export function groupGamesBySystem<T extends HandheldGameLike>(games: T[]): HandheldSystem<T>[] {
  const buckets = new Map<string, T[]>();
  for (const game of games) {
    if (game.hidden) continue;
    const id = normalizePlatform(game.game_type);
    const bucket = buckets.get(id);
    if (bucket) bucket.push(game);
    else buckets.set(id, [game]);
  }
  return [...buckets.entries()]
    .map(([id, list]) => ({
      id,
      label: platformLabel(id),
      games: sortGames(list),
    }))
    .sort((a, b) => b.games.length - a.games.length || a.label.localeCompare(b.label, "zh-CN"));
}

export function sortGames<T extends HandheldGameLike>(games: T[]): T[] {
  return [...games].sort((a, b) => {
    const fav = Number(b.favorite ?? false) - Number(a.favorite ?? false);
    if (fav !== 0) return fav;
    return (a.name ?? "").localeCompare(b.name ?? "", "zh-CN");
  });
}

/* ============================================================================
   全内容整合：番剧 / 漫画 / 小说 + 「继续」聚合
   —— 与游戏系统共用同一套轮播 + 网格 + 详情三段式。
   全部为纯函数（结构化入参，不依赖 store），便于单测。
   ============================================================================ */

export type HandheldMediaKind = "anime" | "comic" | "novel";
export type HandheldItemKind = "game" | HandheldMediaKind;

/** 媒体/聚合网格的统一条目。封面：游戏由渲染层 fileSrc 解析；媒体为远程 URL。 */
export interface HandheldUnifiedItem {
  kind: HandheldItemKind;
  /** 回查原始条目的键：游戏 id / 番剧历史 key（收藏为 name）/ 漫画 id / 小说历史 key */
  key: string;
  title: string;
  cover: string;
  /** 副标题：最近集数 / 章节 / 收藏类型等 */
  subtitle: string;
  /** ms 时间戳，排序依据 */
  time: number;
  /** 是否来自历史（影响 A 键行为：续播 vs 打开详情） */
  fromHistory: boolean;
}

/** 番剧收藏分型展示名（与 collectionStore 的 collectType 对应）。 */
export const COLLECT_TYPE_LABELS: Record<number, string> = {
  1: "在看",
  2: "想看",
  3: "搁置",
  4: "看过",
  5: "抛弃",
};

export interface AnimeHistoryLike {
  key?: string;
  name: string;
  image?: string | null;
  lastEpisodeName?: string | null;
  ruleName?: string | null;
  sourceUrl?: string | null;
  /** ISO 字符串 */
  updatedAt?: string | null;
}

export interface AnimeCollectLike {
  name: string;
  image?: string | null;
  collectType?: number;
  updatedAt?: string | null;
}

function isoToMs(iso: string | null | undefined): number {
  if (!iso) return 0;
  const ms = Date.parse(iso);
  return Number.isNaN(ms) ? 0 : ms;
}

/**
 * 番剧系统条目：观看历史 + 收藏合并去重（同名以历史为准），时间降序。
 */
export function mergeAnimeItems(
  history: AnimeHistoryLike[],
  collects: AnimeCollectLike[],
): HandheldUnifiedItem[] {
  const items: HandheldUnifiedItem[] = [];
  const seen = new Set<string>();
  for (const h of history) {
    seen.add(h.name);
    items.push({
      kind: "anime",
      key: h.key || h.name,
      title: h.name,
      cover: h.image ?? "",
      subtitle: h.lastEpisodeName ?? "",
      time: isoToMs(h.updatedAt),
      fromHistory: true,
    });
  }
  for (const c of collects) {
    if (seen.has(c.name)) continue;
    seen.add(c.name);
    items.push({
      kind: "anime",
      key: c.name,
      title: c.name,
      cover: c.image ?? "",
      subtitle: COLLECT_TYPE_LABELS[c.collectType ?? 0] ?? "已收藏",
      time: isoToMs(c.updatedAt),
      fromHistory: false,
    });
  }
  return items.sort((a, b) => b.time - a.time);
}

export interface ComicHistoryLike {
  id: string;
  title: string;
  thumb_url?: string | null;
  last_title?: string | null;
  /** ms 时间戳 */
  ts?: number | null;
}

export interface ComicFavoriteLike {
  id: string;
  title: string;
  thumb_url?: string | null;
  author?: string | null;
}

/**
 * 漫画系统条目：阅读历史（按时间）+ 书架（登录后可用），历史优先展示。
 */
export function mergeComicItems(
  history: ComicHistoryLike[],
  favorites: ComicFavoriteLike[],
): HandheldUnifiedItem[] {
  const items: HandheldUnifiedItem[] = [];
  const seen = new Set<string>();
  for (const r of history) {
    seen.add(r.id);
    items.push({
      kind: "comic",
      key: r.id,
      title: r.title,
      cover: r.thumb_url ?? "",
      subtitle: r.last_title ?? "",
      time: r.ts ?? 0,
      fromHistory: true,
    });
  }
  for (const f of favorites) {
    if (seen.has(f.id)) continue;
    seen.add(f.id);
    items.push({
      kind: "comic",
      key: f.id,
      title: f.title,
      cover: f.thumb_url ?? "",
      subtitle: f.author || "书架",
      time: 0,
      fromHistory: false,
    });
  }
  return items.sort(
    (a, b) => b.time - a.time || a.title.localeCompare(b.title, "zh-CN"),
  );
}

export interface NovelHistoryLike {
  key: string;
  title: string;
  coverUrl?: string | null;
  chapterTitle?: string | null;
  /** 0~1 阅读进度 */
  progress?: number | null;
  /** ms 时间戳 */
  updatedAt?: number | null;
}

/** 小说系统条目：阅读历史 + 章节进度百分比。 */
export function buildNovelItems(entries: NovelHistoryLike[]): HandheldUnifiedItem[] {
  return entries
    .map((e) => {
      const pct = Math.round((e.progress ?? 0) * 100);
      const subtitle = e.chapterTitle
        ? pct > 0
          ? `${e.chapterTitle} · ${pct}%`
          : e.chapterTitle
        : pct > 0
          ? `已读 ${pct}%`
          : "";
      return {
        kind: "novel" as const,
        key: e.key,
        title: e.title,
        cover: e.coverUrl ?? "",
        subtitle,
        time: e.updatedAt ?? 0,
        fromHistory: true,
      };
    })
    .sort((a, b) => b.time - a.time);
}

/** 「继续」聚合：跨类别最近使用，时间降序取前 limit 条。 */
export function sortRecentItems(
  items: HandheldUnifiedItem[],
  limit = 16,
): HandheldUnifiedItem[] {
  return [...items].sort((a, b) => b.time - a.time).slice(0, Math.max(limit, 0));
}

/* ---- 系统轮播分区 ---- */

export type HandheldSectionKind = "recent" | "media" | "games";

export interface HandheldSectionMeta {
  /** recent / anime / comic / novel / 平台 id */
  id: string;
  label: string;
  kind: HandheldSectionKind;
  count: number;
}

/** Android 首页的第一层频道，顺序固定以避免加载/空态造成焦点漂移。 */
export type HandheldChannelId = "recent" | "games" | "anime" | "comic" | "novel";

export interface HandheldChannelMeta {
  id: HandheldChannelId;
  label: string;
  kind: HandheldSectionKind;
  count: number;
}

export interface HandheldGameSystemOption {
  /** all 表示全部游戏平台，其他值为规范化平台 id。 */
  id: string;
  label: string;
  count: number;
}

export interface HandheldMemoryV3 {
  version: 3;
  channel: HandheldChannelId;
  gameSystemId: string;
  focusByView: Record<string, number>;
  keyByView: Record<string, string>;
}

const CHANNEL_IDS: readonly HandheldChannelId[] = ["recent", "games", "anime", "comic", "novel"];

export function isHandheldChannelId(value: unknown): value is HandheldChannelId {
  return typeof value === "string" && CHANNEL_IDS.includes(value as HandheldChannelId);
}

/** 构造稳定的第一层频道；空频道保留，由页面展示空态而不是改变索引。 */
export function buildHandheldChannels(opts: {
  recentCount: number;
  animeCount: number;
  comicCount: number;
  novelCount: number;
  gameCount: number;
}): HandheldChannelMeta[] {
  return [
    { id: "recent", label: "继续", kind: "recent", count: Math.max(0, opts.recentCount) },
    { id: "games", label: "游戏", kind: "games", count: Math.max(0, opts.gameCount) },
    { id: "anime", label: "番剧", kind: "media", count: Math.max(0, opts.animeCount) },
    { id: "comic", label: "漫画", kind: "media", count: Math.max(0, opts.comicCount) },
    { id: "novel", label: "小说", kind: "media", count: Math.max(0, opts.novelCount) },
  ];
}

/** 游戏频道第二层平台：全部游戏 + 有内容的平台，空平台不进入循环。 */
export function buildHandheldGameSystems(
  systems: { id: string; label: string; count: number }[],
): HandheldGameSystemOption[] {
  const total = systems.reduce((sum, system) => sum + Math.max(0, system.count), 0);
  return [
    { id: "all", label: "全部游戏", count: total },
    ...systems
      .filter((system) => system.count > 0)
      .map((system) => ({ id: system.id, label: system.label, count: system.count })),
  ];
}

export function wrappedIndex(index: number, delta: number, length: number): number {
  if (length <= 0) return 0;
  return (index + delta + length * 2) % length;
}

export function handheldViewKey(channel: HandheldChannelId, gameSystemId = "all"): string {
  return channel === "games" ? `games:${gameSystemId}` : channel;
}

/** 将旧版 section/key 记忆迁移到双层频道模型；无效内容安全回退到继续频道。 */
export function migrateHandheldMemory(
  raw: unknown,
  gameSystemIds: string[],
): HandheldMemoryV3 {
  const value = raw && typeof raw === "object" ? raw as Record<string, unknown> : {};
  const knownSystems = new Set(["all", ...gameSystemIds]);
  let channel: HandheldChannelId = "recent";
  let gameSystemId = "all";
  let viewKey = "recent";

  if (value.version === 3 && isHandheldChannelId(value.channel)) {
    channel = value.channel;
    gameSystemId = typeof value.gameSystemId === "string" && knownSystems.has(value.gameSystemId)
      ? value.gameSystemId
      : "all";
    viewKey = handheldViewKey(channel, gameSystemId);
  } else {
    const legacySection = typeof value.section === "string" ? value.section : "";
    if (legacySection === "games") {
      channel = "games";
    } else if (isHandheldChannelId(legacySection)) {
      channel = legacySection;
    } else if (knownSystems.has(legacySection)) {
      channel = "games";
      gameSystemId = legacySection;
    }
    viewKey = handheldViewKey(channel, gameSystemId);
  }

  const focusByView: Record<string, number> = {};
  const keyByView: Record<string, string> = {};
  if (value.version === 3) {
    const storedFocus = value.focusByView;
    if (storedFocus && typeof storedFocus === "object") {
      for (const [key, index] of Object.entries(storedFocus as Record<string, unknown>)) {
        if (typeof index === "number" && Number.isFinite(index)) focusByView[key] = Math.max(0, Math.floor(index));
      }
    }
    const storedKeys = value.keyByView;
    if (storedKeys && typeof storedKeys === "object") {
      for (const [key, itemKey] of Object.entries(storedKeys as Record<string, unknown>)) {
        if (typeof itemKey === "string" && itemKey) keyByView[key] = itemKey;
      }
    }
  } else if (typeof value.key === "string" && value.key) {
    keyByView[viewKey] = value.key;
  }

  return { version: 3, channel, gameSystemId, focusByView, keyByView };
}

/**
 * 轮播分区序列：继续（非空才显示）→ 游戏 → 番剧 → 漫画 → 小说 → 游戏平台（按数量降序，来自调用方）。
 */
export function buildSections(opts: {
  recentCount: number;
  animeCount: number;
  comicCount: number;
  novelCount: number;
  gameSystems: { id: string; label: string; count: number }[];
}): HandheldSectionMeta[] {
  const sections: HandheldSectionMeta[] = [];
  if (opts.recentCount > 0) {
    sections.push({ id: "recent", label: "继续", kind: "recent", count: opts.recentCount });
  }
  const gameCount = opts.gameSystems.reduce((total, system) => total + Math.max(0, system.count), 0);
  if (gameCount > 0) {
    sections.push({ id: "games", label: "游戏", kind: "games", count: gameCount });
  }
  sections.push({ id: "anime", label: "番剧", kind: "media", count: opts.animeCount });
  sections.push({ id: "comic", label: "漫画", kind: "media", count: opts.comicCount });
  sections.push({ id: "novel", label: "小说", kind: "media", count: opts.novelCount });
  for (const s of opts.gameSystems) {
    sections.push({ id: s.id, label: s.label, kind: "games", count: s.count });
  }
  return sections;
}

/**
 * 掌机全功能菜单：把桌面端的内容、工具、导入入口收进一个可手柄操作的抽屉。
 * 这里复用主导航定义，避免掌机模式与普通模式出现两套会漂移的入口文案。
 */
export interface HandheldQuickNavGroup {
  id: "content" | "tools" | "system";
  label: string;
  items: NavItem[];
}

const HANDHELD_HOME_ITEM: NavItem = {
  id: "handheld-home",
  label: "首页",
  ariaLabel: "返回统一掌机首页",
  icon: "home",
  view: "home",
  group: "library",
  surface: "content",
};

const HANDHELD_GAME_LIBRARY_ITEM: NavItem = {
  id: "handheld-game-library",
  label: "游戏",
  ariaLabel: "打开游戏库",
  icon: "gamepad",
  view: "game-library",
  group: "library",
  surface: "content",
};

const HANDHELD_IMPORT_ITEM: NavItem = {
  id: "handheld-import",
  label: "掌机导入",
  ariaLabel: "导入掌机游戏",
  icon: "database",
  view: "handheld-import",
  group: "import",
  surface: "utility",
};

export function buildHandheldQuickNavGroups(): HandheldQuickNavGroup[] {
  return [
    {
      id: "content",
      label: "内容",
      items: [
        HANDHELD_HOME_ITEM,
        HANDHELD_GAME_LIBRARY_ITEM,
        ...DOCK_ITEMS.filter((item) => item.surface === "content" && item.view !== "home"),
      ],
    },
    {
      id: "tools",
      label: "工具",
      items: TOOL_ITEMS.filter((item) => item.group === "library" || item.group === "tools"),
    },
    {
      id: "system",
      label: "导入与系统",
      items: [
        HANDHELD_IMPORT_ITEM,
        ...TOOL_ITEMS.filter((item) => item.group === "import" || item.group === "system"),
        ...DOCK_ITEMS.filter((item) => item.group === "system" && !item.view.startsWith("__")),
      ],
    },
  ];
}
