import { describe, expect, it, vi } from "vitest";
import {
  buildDockShortcuts,
  buildShortcutCatalog,
  buildShortcutParameter,
  resolveDockIndexByKey,
} from "./shortcuts";
import { DOCK_ITEMS } from "./nav";

describe("shortcuts", () => {
  describe("resolveDockIndexByKey", () => {
    it("parses Digit keys 1-5", () => {
      expect(resolveDockIndexByKey("Digit1")).toBe(0);
      expect(resolveDockIndexByKey("Digit5")).toBe(4);
    });

    it("parses Numpad keys 1-5", () => {
      expect(resolveDockIndexByKey("Numpad1")).toBe(0);
      expect(resolveDockIndexByKey("Numpad5")).toBe(4);
    });

    it("ignores other keys", () => {
      expect(resolveDockIndexByKey("Digit6")).toBeNull();
      expect(resolveDockIndexByKey("KeyA")).toBeNull();
      expect(resolveDockIndexByKey("Escape")).toBeNull();
    });
  });

  describe("buildDockShortcuts", () => {
    it("generates 5 shortcuts matching first 5 dock items", () => {
      const shortcuts = buildDockShortcuts();
      expect(shortcuts).toHaveLength(5);
      for (let i = 0; i < 5; i++) {
        expect(shortcuts[i].keys).toBe(String(i + 1));
        expect(shortcuts[i].trigger.key).toBe(String(i + 1));
        expect(shortcuts[i].description).toContain(DOCK_ITEMS[i].label);
      }
    });
  });

  describe("buildShortcutCatalog", () => {
    it("includes dock, help, search, and escape shortcuts", () => {
      const catalog = buildShortcutCatalog();
      const ids = catalog.map((s) => s.id);
      expect(ids).toContain("help");
      expect(ids).toContain("focus-search-slash");
      expect(ids).toContain("focus-search-modk");
      expect(ids).toContain("home");
      expect(catalog.filter((s) => s.id.startsWith("dock-"))).toHaveLength(5);
    });

    it("has non-empty keys and descriptions", () => {
      for (const s of buildShortcutCatalog()) {
        expect(s.keys.length).toBeGreaterThan(0);
        expect(s.description.length).toBeGreaterThan(0);
      }
    });
  });

  describe("buildShortcutParameter", () => {
    it("calls navigate when dock shortcut fires", () => {
      const actions = {
        navigate: vi.fn(),
        toggleTools: vi.fn(),
        focusSearch: vi.fn(),
        toggleHelp: vi.fn(),
        goHome: vi.fn(),
      };
      const param = buildShortcutParameter(actions);
      const dock1 = param.trigger.find((t) => t.key === "1");
      expect(dock1).toBeDefined();
      dock1!.callback({ node: document.body, trigger: dock1!, originalEvent: new KeyboardEvent("keydown", { key: "1" }) });
      expect(actions.navigate).toHaveBeenCalledWith(DOCK_ITEMS[0].view);
    });

    it("calls toggleTools for tools dock shortcut", () => {
      const actions = {
        navigate: vi.fn(),
        toggleTools: vi.fn(),
        focusSearch: vi.fn(),
        toggleHelp: vi.fn(),
        goHome: vi.fn(),
      };
      const param = buildShortcutParameter(actions);
      const toolsIdx = DOCK_ITEMS.findIndex((i) => i.view === "__tools") + 1;
      const toolsTrigger = param.trigger.find((t) => t.key === String(toolsIdx));
      expect(toolsTrigger).toBeDefined();
      toolsTrigger!.callback({ node: document.body, trigger: toolsTrigger!, originalEvent: new KeyboardEvent("keydown", { key: String(toolsIdx) }) });
      expect(actions.toggleTools).toHaveBeenCalled();
    });

    it("calls focusSearch and toggleHelp", () => {
      const actions = {
        navigate: vi.fn(),
        toggleTools: vi.fn(),
        focusSearch: vi.fn(),
        toggleHelp: vi.fn(),
        goHome: vi.fn(),
      };
      const param = buildShortcutParameter(actions);
      const slashTrigger = param.trigger.find((t) => t.key === "/");
      const helpTrigger = param.trigger.find((t) => t.key === "?");
      slashTrigger?.callback({ node: document.body, trigger: slashTrigger!, originalEvent: new KeyboardEvent("keydown", { key: "/" }) });
      helpTrigger?.callback({ node: document.body, trigger: helpTrigger!, originalEvent: new KeyboardEvent("keydown", { key: "?" }) });
      expect(actions.focusSearch).toHaveBeenCalled();
      expect(actions.toggleHelp).toHaveBeenCalled();
    });

    it("does not register Escape as a shortcut trigger", () => {
      const actions = {
        navigate: vi.fn(),
        toggleTools: vi.fn(),
        focusSearch: vi.fn(),
        toggleHelp: vi.fn(),
        goHome: vi.fn(),
      };
      const param = buildShortcutParameter(actions);
      expect(param.trigger.some((t) => t.key === "Escape")).toBe(false);
    });
  });
});
