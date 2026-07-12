import { describe, expect, it, vi } from "vitest";
import type { MediaPresentationItem, PresentationAsset } from "../model";
import { normalizeMediaIdentity } from "./mediaIdentity";
import { composeGameScene } from "./sceneComposition";
import { composeGameVisual } from "./visualComposition";

function asset(id: string, role: PresentationAsset["role"] = "screenshot", aspect: PresentationAsset["aspect"] = "landscape"): PresentationAsset {
  return { id, src: `https://media.test/${id}.webp`, role, alt: id, aspect };
}

function item(id: string, options: Partial<MediaPresentationItem> = {}): MediaPresentationItem {
  const cover = asset(`${id}-cover`, "cover", "portrait");
  const hero = asset(`${id}-hero`, "hero");
  const screenshots = [asset(`${id}-shot-1`), asset(`${id}-shot-2`)];
  return {
    id,
    module: "games",
    title: id,
    cover,
    hero,
    screenshots,
    media: [hero, ...screenshots, cover],
    mediaQuality: "a",
    favorite: false,
    installed: true,
    metadata: { tags: [] },
    actions: [
      { id: "select", label: "选择", emphasis: "quiet", enabled: true, run: vi.fn() },
      { id: "open", label: "详情", emphasis: "secondary", enabled: true, run: vi.fn() },
      { id: "launch", label: "启动", emphasis: "primary", enabled: true, run: vi.fn() },
    ],
    ...options,
  };
}

describe("media composition", () => {
  it("normalizes remote query ordering and local slashes", () => {
    expect(normalizeMediaIdentity("https://x.test/a.webp?b=2&a=1#x")).toBe("https://x.test/a.webp?a=1&b=2");
    expect(normalizeMediaIdentity("C:\\Games\\Cover.webp")).toBe("c:/games/cover.webp");
  });

  it("always emits five semantic visual slots with matching ownership", () => {
    const current = item("current");
    const recent = item("recent", { metadata: { tags: [], lastPlayed: "2026-07-10T00:00:00Z", totalSeconds: 400 } });
    const favorite = item("favorite", { favorite: true });
    const result = composeGameVisual([current, recent, favorite], "current");
    expect(result.slots.map((slot) => slot.role)).toEqual(["lead", "scene-a", "scene-b", "continue", "featured"]);
    expect(result.slots).toHaveLength(5);
    expect(result.slots[3].ownerItemId).toBe("recent");
    expect(result.slots[3].action).toEqual({ type: "select-item", itemId: "recent" });
    expect(result.slots[4].ownerItemId).toBe("favorite");
    expect(result.backgroundAsset?.id).toBe(result.slots[0].asset?.id);
    expect(result.chromaAsset?.id).toBe(result.slots[0].asset?.id);
  });

  it("deduplicates repeated selected media without shifting slot semantics", () => {
    const repeated = asset("same", "hero");
    const current = item("current", { hero: repeated, cover: { ...repeated, role: "cover" }, screenshots: [], media: [repeated] });
    const result = composeGameVisual([current], "current");
    expect(result.slots).toHaveLength(5);
    expect(result.slots[0].asset?.id).toBe("same");
    expect(result.slots[1].asset).toBeNull();
    expect(result.slots[3].action.type).toBe("none");
  });

  it("gives the selected title multiple scene frames before adjacent titles", () => {
    const result = composeGameScene([item("current"), item("other"), item("third")], "current", 8);
    expect(result.entries.slice(0, 4).every((entry) => entry.ownerItemId === "current")).toBe(true);
    expect(new Set(result.entries.map((entry) => entry.asset?.src).filter(Boolean)).size).toBe(result.entries.filter((entry) => entry.asset).length);
    expect(result.entries[0].role).toBe("lead");
  });
});
