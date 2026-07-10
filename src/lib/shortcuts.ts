import { DOCK_ITEMS } from "./nav";
import type { ShortcutEventDetail } from "@svelte-put/shortcut";

export interface ShortcutTrigger {
  key?: string;
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
  goBack(): void;
}

/** Parse Digit1..Digit5 / Numpad1..Numpad5 to a zero-based dock index. */
export function resolveDockIndexByKey(key: string): number | null {
  const digitMatch = /^Digit([1-5])$/.exec(key);
  if (digitMatch) return parseInt(digitMatch[1], 10) - 1;
  const numpadMatch = /^Numpad([1-5])$/.exec(key);
  if (numpadMatch) return parseInt(numpadMatch[1], 10) - 1;
  return null;
}

/** Build numeric shortcuts from nav metadata so help, Dock badges and handlers cannot drift. */
export function buildDockShortcuts(): ShortcutDefinition[] {
  return DOCK_ITEMS
    .filter((item) => item.shortcut)
    .map((item) => ({
      id: `dock-${item.id}`,
      keys: item.shortcut!,
      trigger: { key: item.shortcut! },
      description: item.ariaLabel,
      scope: "global" as const,
    }));
}

/** Build the complete shortcut catalog for help rendering and registration. */
export function buildShortcutCatalog(): ShortcutDefinition[] {
  return [
    ...buildDockShortcuts(),
    {
      id: "help",
      keys: "?",
      trigger: { key: "?" },
      description: "打开或关闭快捷键帮助",
      scope: "global",
    },
    {
      id: "focus-search-slash",
      keys: "/",
      trigger: { key: "/" },
      description: "聚焦当前页面搜索框",
      scope: "home",
    },
    {
      id: "focus-search-modk",
      keys: "Ctrl / Cmd + K",
      trigger: { code: "KeyK", modifier: ["ctrl", "meta"] },
      description: "聚焦当前页面搜索框",
      scope: "home",
    },
    {
      id: "back",
      keys: "Esc",
      trigger: { key: "Escape", modifier: false },
      description: "按层级关闭或返回",
      scope: "global",
    },
  ];
}

/** Avoid treating text input as navigation while preserving the explicit Mod+K command. */
function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  return target.isContentEditable;
}

/** Convert a definition into a @svelte-put/shortcut trigger config. */
export function toShortcutTrigger(
  def: ShortcutDefinition,
  actions: ShortcutActions,
): { trigger: ShortcutTrigger & { callback: (detail: ShortcutEventDetail) => void } } {
  const callback = (detail: ShortcutEventDetail) => {
    const typing = isTypingTarget(detail.originalEvent.target);
    if (typing && def.id !== "focus-search-modk") return;

    switch (def.id) {
      case "help":
        actions.toggleHelp();
        return;
      case "back":
        actions.goBack();
        return;
      case "focus-search-slash":
      case "focus-search-modk":
        actions.focusSearch();
        return;
      default:
        if (def.id.startsWith("dock-")) {
          const item = DOCK_ITEMS.find(candidate => candidate.shortcut === def.keys);
          if (!item) return;
          if (item.view === "__tools") actions.toggleTools();
          else actions.navigate(item.view);
        }
    }
  };
  return { trigger: { ...def.trigger, callback } };
}

/** Escape stays on the layered App/router handler so modal capture has first priority. */
export function buildShortcutParameter(actions: ShortcutActions) {
  const triggers = buildShortcutCatalog()
    .filter((def) => def.id !== "back")
    .map((def) => toShortcutTrigger(def, actions).trigger);
  return { trigger: triggers };
}
