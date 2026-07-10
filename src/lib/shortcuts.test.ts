import { describe, expect, it, vi } from "vitest";
import {
  buildDockShortcuts,
  buildShortcutCatalog,
  buildShortcutParameter,
  resolveDockIndexByKey,
  toShortcutTrigger,
  type ShortcutActions,
} from "./shortcuts";
import { DOCK_ITEMS } from "./nav";

function actions(): ShortcutActions & Record<string, ReturnType<typeof vi.fn>> {
  return {
    navigate: vi.fn(),
    toggleTools: vi.fn(),
    focusSearch: vi.fn(),
    toggleHelp: vi.fn(),
    goBack: vi.fn(),
  };
}

function detail(trigger: { callback: (detail: any) => void }, event: KeyboardEvent) {
  trigger.callback({ node: document.body, trigger, originalEvent: event });
}

describe("shortcuts", () => {
  it("parses Digit/Numpad 1-5 and ignores other keys", () => {
    expect(resolveDockIndexByKey("Digit1")).toBe(0);
    expect(resolveDockIndexByKey("Numpad5")).toBe(4);
    expect(resolveDockIndexByKey("Digit6")).toBeNull();
    expect(resolveDockIndexByKey("Escape")).toBeNull();
  });

  it("generates dock shortcuts from nav metadata", () => {
    const shortcuts = buildDockShortcuts();
    const configured = DOCK_ITEMS.filter(item => item.shortcut);
    expect(shortcuts).toHaveLength(configured.length);
    expect(shortcuts.map(item => item.keys)).toEqual(configured.map(item => item.shortcut));
    expect(shortcuts.map(item => item.description)).toEqual(configured.map(item => item.ariaLabel));
  });

  it("documents current-page search and layered back semantics", () => {
    const catalog = buildShortcutCatalog();
    const slash = catalog.find(item => item.id === "focus-search-slash")!;
    const modK = catalog.find(item => item.id === "focus-search-modk")!;
    const back = catalog.find(item => item.id === "back")!;
    expect(slash.description).toContain("当前页面");
    expect(modK.description).toContain("当前页面");
    expect(back.description).toContain("按层级");
  });

  it("routes numeric Dock commands and the tools command", () => {
    const handlers = actions();
    const param = buildShortcutParameter(handlers);
    const first = param.trigger.find(trigger => trigger.key === "1")!;
    detail(first, new KeyboardEvent("keydown", { key: "1" }));
    expect(handlers.navigate).toHaveBeenCalledWith(DOCK_ITEMS[0].view);

    const tools = param.trigger.find(trigger => trigger.key === DOCK_ITEMS.find(item => item.id === "tools")!.shortcut)!;
    detail(tools, new KeyboardEvent("keydown", { key: tools.key }));
    expect(handlers.toggleTools).toHaveBeenCalledOnce();
  });

  it("focuses current search for slash and Mod+K without registering Escape twice", () => {
    const handlers = actions();
    const param = buildShortcutParameter(handlers);
    const slash = param.trigger.find(trigger => trigger.key === "/")!;
    const modK = param.trigger.find(trigger => trigger.code === "KeyK")!;
    detail(slash, new KeyboardEvent("keydown", { key: "/" }));
    detail(modK, new KeyboardEvent("keydown", { key: "k", ctrlKey: true }));
    expect(handlers.focusSearch).toHaveBeenCalledTimes(2);
    expect(param.trigger.some(trigger => trigger.key === "Escape")).toBe(false);
  });

  it("does not steal slash while typing but allows the explicit Mod+K command", () => {
    const handlers = actions();
    const catalog = buildShortcutCatalog();
    const input = document.createElement("input");
    document.body.append(input);

    const slash = toShortcutTrigger(catalog.find(item => item.id === "focus-search-slash")!, handlers).trigger;
    const modK = toShortcutTrigger(catalog.find(item => item.id === "focus-search-modk")!, handlers).trigger;
    detail(slash, new KeyboardEvent("keydown", { key: "/" }));
    slash.callback({ node: document.body, trigger: slash, originalEvent: { target: input } as unknown as KeyboardEvent });
    modK.callback({ node: document.body, trigger: modK, originalEvent: { target: input } as unknown as KeyboardEvent });

    expect(handlers.focusSearch).toHaveBeenCalledTimes(2);
  });
});
