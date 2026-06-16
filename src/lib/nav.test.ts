import { describe, expect, it } from "vitest";
import {
  NAV_ITEMS,
  DOCK_ITEMS,
  TOOL_ITEMS,
  NAV_GROUP_ORDER,
  GROUP_LABELS,
  BIG_PICTURE_ITEM,
} from "./nav";

describe("navigation single source of truth", () => {
  it("has unique ids and unique views across all items", () => {
    const ids = NAV_ITEMS.map((i) => i.id);
    const views = NAV_ITEMS.map((i) => i.view);
    expect(new Set(ids).size).toBe(ids.length);
    expect(new Set(views).size).toBe(views.length);
  });

  it("every item has a non-empty label and icon", () => {
    for (const i of [...DOCK_ITEMS, ...TOOL_ITEMS]) {
      expect(i.label.length).toBeGreaterThan(0);
      expect(i.icon.length).toBeGreaterThan(0);
    }
  });

  it("dock has primary nav items including home and big picture", () => {
    expect(DOCK_ITEMS.some((d) => d.view === "home")).toBe(true);
    expect(DOCK_ITEMS.some((d) => d.id === "bigpic")).toBe(true);
    expect(DOCK_ITEMS.some((d) => d.id === "tools")).toBe(true);
    expect(DOCK_ITEMS.length).toBeLessThanOrEqual(7);
  });

  it("tool drawer items don't overlap with dock primary views", () => {
    const dockViews = new Set(DOCK_ITEMS.map(d => d.view));
    for (const t of TOOL_ITEMS) {
      expect(dockViews.has(t.view)).toBe(false);
    }
  });

  it("NAV_ITEMS covers dock + tools (excluding special views)", () => {
    const navViews = new Set(NAV_ITEMS.map(i => i.view));
    for (const t of TOOL_ITEMS) {
      expect(navViews.has(t.view)).toBe(true);
    }
  });

  it("group order covers all groups in use and every group has a label entry", () => {
    const used = new Set(NAV_ITEMS.map((i) => i.group));
    for (const g of used) expect(NAV_GROUP_ORDER).toContain(g);
    for (const g of NAV_GROUP_ORDER) expect(GROUP_LABELS).toHaveProperty(g);
  });
});
