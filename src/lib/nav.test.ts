import { describe, expect, it } from "vitest";
import {
  BIG_PICTURE_ITEM,
  DOCK_ITEMS,
  getRouteLevel,
  getViewLabel,
  GROUP_LABELS,
  isPrimaryContentView,
  NAV_GROUP_ORDER,
  NAV_ITEMS,
  PRIMARY_CONTENT_VIEWS,
  TOOL_ITEMS,
} from "./nav";

describe("navigation single source of truth", () => {
  it("has unique ids, views and shortcut assignments", () => {
    const all = [...DOCK_ITEMS, ...TOOL_ITEMS];
    const ids = all.map(item => item.id);
    const navigableViews = NAV_ITEMS.map(item => item.view);
    const shortcuts = DOCK_ITEMS.flatMap(item => item.shortcut ? [item.shortcut] : []);
    expect(new Set(ids).size).toBe(ids.length);
    expect(new Set(navigableViews).size).toBe(navigableViews.length);
    expect(new Set(shortcuts).size).toBe(shortcuts.length);
  });

  it("keeps the five primary content roots in the required order", () => {
    expect(PRIMARY_CONTENT_VIEWS).toEqual(["home", "records", "anime", "comic", "novel"]);
    expect(DOCK_ITEMS.filter(item => item.surface === "content").map(item => item.view))
      .toEqual(PRIMARY_CONTENT_VIEWS);
  });

  it("gives every control a visible label, accessible label and icon", () => {
    for (const item of [...DOCK_ITEMS, ...TOOL_ITEMS]) {
      expect(item.label.trim()).not.toBe("");
      expect(item.ariaLabel.trim()).not.toBe("");
      expect(item.icon.trim()).not.toBe("");
    }
    expect(BIG_PICTURE_ITEM.ariaLabel).toBe("进入大屏模式");
  });

  it("classifies primary, detail and subview return levels", () => {
    expect(isPrimaryContentView("anime")).toBe(true);
    expect(isPrimaryContentView("novel")).toBe(true);
    expect(getRouteLevel("home")).toBe("primary");
    expect(getRouteLevel("game-detail")).toBe("detail");
    expect(getRouteLevel("settings")).toBe("subview");
    expect(getViewLabel("game-detail")).toBe("游戏详情");
  });

  it("keeps internal mode commands out of URL navigation", () => {
    const navViews = new Set(NAV_ITEMS.map(item => item.view));
    expect(navViews.has("__tools")).toBe(false);
    expect(navViews.has("__bigpicture")).toBe(false);
    for (const tool of TOOL_ITEMS) expect(navViews.has(tool.view)).toBe(true);
  });

  it("covers every group with stable ordering and labels", () => {
    const used = new Set(NAV_ITEMS.map(item => item.group));
    for (const group of used) expect(NAV_GROUP_ORDER).toContain(group);
    for (const group of NAV_GROUP_ORDER) expect(GROUP_LABELS).toHaveProperty(group);
  });
});
