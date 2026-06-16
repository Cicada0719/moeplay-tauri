export type NavGroup = "library" | "tools" | "import" | "system";

export interface NavItem {
  id: string;
  label: string;
  icon: string;
  view: string;
  group: NavGroup;
}

/** Primary dock items — the 6 always-visible navigation entries. */
export const DOCK_ITEMS: NavItem[] = [
  { id: "home",     label: "游戏库", icon: "home",  view: "home",          group: "library" },
  { id: "anime",    label: "番剧",   icon: "film",  view: "anime",         group: "library" },
  { id: "comic",    label: "漫画",   icon: "book",  view: "comic",         group: "library" },
  { id: "tools",    label: "工具",   icon: "grid",  view: "__tools",       group: "tools" },
  { id: "settings", label: "设置",   icon: "gear",  view: "settings",      group: "system" },
  { id: "bigpic",   label: "大屏",   icon: "tv",    view: "__bigpicture",  group: "system" },
];

/** Tool drawer items — shown when the "工具" dock button is tapped. */
export const TOOL_ITEMS: NavItem[] = [
  { id: "discovery",   label: "发现",   icon: "compass",  view: "discovery",    group: "tools" },
  { id: "scraper",     label: "刮削",   icon: "star",     view: "scraper",      group: "tools" },
  { id: "downloads",   label: "下载",   icon: "download", view: "downloads",    group: "tools" },
  { id: "backup",      label: "存档",   icon: "save",     view: "backup",       group: "tools" },
  { id: "stats",       label: "统计",   icon: "chart",    view: "stats",        group: "tools" },
  { id: "import",      label: "导入",   icon: "database", view: "steam-import", group: "import" },
  { id: "emulator",    label: "模拟器", icon: "gamepad",  view: "emulator",     group: "import" },
  { id: "diagnostics", label: "诊断",   icon: "toolbox",  view: "diagnostics",  group: "system" },
];

/** All navigable items (for sidebar rendering, search, etc). */
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

export const BIG_PICTURE_ITEM = {
  id: "bigpic",
  label: "大屏",
  icon: "tv",
  view: "__bigpicture",
} as const;
