export type NavGroup = "library" | "tools" | "import" | "system";
export type NavSurface = "content" | "utility" | "mode";
export type RouteLevel = "primary" | "detail" | "subview";

export interface NavItem {
  id: string;
  label: string;
  ariaLabel: string;
  icon: string;
  view: string;
  group: NavGroup;
  surface: NavSurface;
  shortcut?: string;
}

/** Five primary content roots. Escape never silently jumps between these roots. */
export const PRIMARY_CONTENT_VIEWS = ["home", "records", "anime", "comic", "novel"] as const;
const primaryContentViewSet = new Set<string>(PRIMARY_CONTENT_VIEWS);
const detailViewSet = new Set(["game-detail"]);

/** Primary dock entries in visual order: content, utility, then mode controls. */
export const DOCK_ITEMS: NavItem[] = [
  { id: "home",     label: "游戏库", ariaLabel: "打开游戏库",       icon: "home",  view: "home",         group: "library", surface: "content", shortcut: "1" },
  { id: "records",  label: "记录",   ariaLabel: "打开游玩记录",     icon: "chart", view: "records",      group: "library", surface: "content", shortcut: "2" },
  { id: "anime",    label: "番剧",   ariaLabel: "打开番剧",         icon: "film",  view: "anime",        group: "library", surface: "content", shortcut: "3" },
  { id: "comic",    label: "漫画",   ariaLabel: "打开漫画",         icon: "book",  view: "comic",        group: "library", surface: "content", shortcut: "4" },
  { id: "novel",    label: "小说",   ariaLabel: "打开小说阅读",     icon: "collection", view: "novel", group: "library", surface: "content" },
  { id: "tools",    label: "工具",   ariaLabel: "打开工具抽屉",     icon: "grid",  view: "__tools",      group: "tools",   surface: "utility", shortcut: "5" },
  { id: "settings", label: "设置",   ariaLabel: "打开设置",         icon: "gear",  view: "settings",     group: "system",  surface: "utility" },
  { id: "bigpic",   label: "大屏",   ariaLabel: "进入大屏模式",     icon: "tv",    view: "__bigpicture", group: "system",  surface: "mode" },
];

/** Tool drawer items. They are subviews and return to the source content root. */
export const TOOL_ITEMS: NavItem[] = [
  { id: "continue",    label: "继续",   ariaLabel: "打开继续游玩", icon: "play",     view: "continue",     group: "library", surface: "utility" },
  { id: "discovery",   label: "发现",   ariaLabel: "打开资源发现", icon: "compass",  view: "discovery",    group: "tools",   surface: "utility" },
  { id: "scraper",     label: "刮削",   ariaLabel: "打开刮削中心", icon: "star",     view: "scraper",      group: "tools",   surface: "utility" },
  { id: "tasks",       label: "任务",   ariaLabel: "打开任务中心", icon: "list",     view: "tasks",        group: "tools",   surface: "utility" },
  { id: "sources",     label: "来源",   ariaLabel: "打开来源中心", icon: "compass",  view: "sources",      group: "tools",   surface: "utility" },
  { id: "downloads",   label: "下载",   ariaLabel: "打开下载任务", icon: "download", view: "downloads",    group: "tools",   surface: "utility" },
  { id: "backup",      label: "存档",   ariaLabel: "打开存档管理", icon: "save",     view: "backup",       group: "tools",   surface: "utility" },
  { id: "stats",       label: "统计",   ariaLabel: "打开统计",     icon: "chart",    view: "stats",        group: "tools",   surface: "utility" },
  { id: "import",      label: "导入",   ariaLabel: "打开平台导入", icon: "database", view: "steam-import", group: "import",  surface: "utility" },
  { id: "emulator",    label: "模拟器", ariaLabel: "打开模拟器导入", icon: "gamepad", view: "emulator",     group: "import",  surface: "utility" },
  { id: "diagnostics", label: "诊断",   ariaLabel: "打开诊断",     icon: "toolbox",  view: "diagnostics",  group: "system",  surface: "utility" },
];

/** All URL-navigable items. Internal drawer/mode commands are deliberately excluded. */
export const NAV_ITEMS: NavItem[] = [
  ...DOCK_ITEMS.filter(i => !i.view.startsWith("__")),
  ...TOOL_ITEMS,
];

export const GROUP_LABELS: Record<NavGroup, string | null> = {
  library: null,
  tools: "工具",
  import: "导入",
  system: "系统",
};

export const NAV_GROUP_ORDER: NavGroup[] = ["library", "tools", "import", "system"];

export const BIG_PICTURE_ITEM = DOCK_ITEMS.find(item => item.id === "bigpic")!;

export function isPrimaryContentView(view: string): boolean {
  return primaryContentViewSet.has(view);
}

export function getRouteLevel(view: string): RouteLevel {
  if (isPrimaryContentView(view)) return "primary";
  if (detailViewSet.has(view)) return "detail";
  return "subview";
}

export function getNavItemByView(view: string): NavItem | undefined {
  return [...DOCK_ITEMS, ...TOOL_ITEMS].find(item => item.view === view);
}

export function getViewLabel(view: string): string {
  if (view === "game-detail") return "游戏详情";
  return getNavItemByView(view)?.label ?? "萌游";
}
