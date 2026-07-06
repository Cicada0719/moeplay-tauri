import { DOCK_ITEMS } from "./nav";
import type { ShortcutEventDetail } from "@svelte-put/shortcut";

export interface ShortcutTrigger {
  key: string;
  modifier?: "ctrl" | "meta" | "alt" | "shift" | ("ctrl" | "meta" | "alt" | "shift")[] | false;
  code?: string;
}

export interface ShortcutDefinition {
  id: string;
  keys: string;
  trigger: ShortcutTrigger;
  description: string;
  scope: "global" | "home";
}

export interface ShortcutActions {
  navigate(view: string): void;
  toggleTools(): void;
  focusSearch(): void;
  toggleHelp(): void;
  goHome(): void;
}

/** Parse Digit1..Digit5 / Numpad1..Numpad5 to 0-based dock index. */
export function resolveDockIndexByKey(key: string): number | null {
  const digitMatch = /^Digit([1-5])$/.exec(key);
  if (digitMatch) return parseInt(digitMatch[1], 10) - 1;
  const numpadMatch = /^Numpad([1-5])$/.exec(key);
  if (numpadMatch) return parseInt(numpadMatch[1], 10) - 1;
  return null;
}

/** Build the numeric dock shortcuts (1-5) based on current DOCK_ITEMS order. */
export function buildDockShortcuts(): ShortcutDefinition[] {
  const shortcuts: ShortcutDefinition[] = [];
  const dockable = DOCK_ITEMS.slice(0, 5);
  for (let i = 0; i < dockable.length; i++) {
    const item = dockable[i];
    const num = i + 1;
    shortcuts.push({
      id: `dock-${item.id}`,
      keys: String(num),
      trigger: { key: String(num) },
      description: `切换到「${item.label}」`,
      scope: "global",
    });
  }
  return shortcuts;
}

/** Build the complete shortcut catalog for help rendering and registration. */
export function buildShortcutCatalog(): ShortcutDefinition[] {
  return [
    ...buildDockShortcuts(),
    {
      id: "help",
      keys: "?",
      trigger: { key: "?" },
      description: "打开/关闭快捷键帮助",
      scope: "global",
    },
    {
      id: "focus-search-slash",
      keys: "/",
      trigger: { key: "/" },
      description: "聚焦游戏库搜索框",
      scope: "home",
    },
    {
      id: "focus-search-modk",
      keys: "Ctrl / Cmd + K",
      trigger: { key: "k", modifier: ["ctrl", "meta"] },
      description: "聚焦游戏库搜索框",
      scope: "home",
    },
    {
      id: "home",
      keys: "Esc",
      trigger: { key: "Escape", modifier: false },
      description: "返回首页",
      scope: "global",
    },
  ];
}

/** 判断当前焦点是否在可编辑输入元素上，避免快捷键把输入内容当导航键处理。 */
function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  return target.isContentEditable;
}

/** Convert a shortcut definition into a @svelte-put/shortcut trigger config with callback. */
export function toShortcutTrigger(
  def: ShortcutDefinition,
  actions: ShortcutActions
): { trigger: ShortcutTrigger & { callback: (detail: ShortcutEventDetail) => void } } {
  const callback = (detail: ShortcutEventDetail) => {
    if (isTypingTarget(detail.originalEvent.target)) return;
    switch (def.id) {
      case "help":
        actions.toggleHelp();
        return;
      case "home":
        actions.goHome();
        return;
      case "focus-search-slash":
      case "focus-search-modk":
        actions.focusSearch();
        return;
      default:
        if (def.id.startsWith("dock-")) {
          const idx = parseInt(def.keys, 10) - 1;
          const item = DOCK_ITEMS[idx];
          if (item) {
            if (item.view === "__tools") actions.toggleTools();
            else actions.navigate(item.view);
          }
        }
    }
  };
  return { trigger: { ...def.trigger, callback } };
}

/** Build the shortcut parameter ready for use:shortcut action. */
export function buildShortcutParameter(actions: ShortcutActions) {
  const triggers = buildShortcutCatalog()
    .filter((def) => def.id !== "home") // Escape handled by existing keydown listener
    .map((def) => toShortcutTrigger(def, actions).trigger);
  return { trigger: triggers };
}
