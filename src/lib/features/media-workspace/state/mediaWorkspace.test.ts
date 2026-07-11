import { describe, expect, it } from "vitest";
import { createMediaWorkspaceState } from "./mediaWorkspace.svelte";

describe("createMediaWorkspaceState", () => {
  it("remembers mode, selection, focus and scroll independently per module", () => {
    const state = createMediaWorkspaceState();
    state.setMode("scene");
    state.selectItem("game-1");
    state.focusItem("game-2");
    state.rememberScroll({ x: 42, y: 360 });

    state.setModule("anime");
    expect(state.activeMode).toBe("visual");
    expect(state.activeMemory.selectedItemId).toBeNull();
    state.setMode("index");
    state.selectItem("anime-1");

    state.setModule("games");
    expect(state.activeMemory).toEqual({
      mode: "scene",
      selectedItemId: "game-1",
      focusedItemId: "game-2",
      scroll: { x: 42, y: 360 },
    });
  });

  it("supports explicit module updates without changing the active module", () => {
    const state = createMediaWorkspaceState();
    state.setMode("index", "comics");
    state.selectItem("comic-1", "comics");

    expect(state.activeModule).toBe("games");
    expect(state.memoryFor("comics")).toMatchObject({ mode: "index", selectedItemId: "comic-1" });
  });

  it("creates detached snapshots and restores validated state", () => {
    const state = createMediaWorkspaceState({ activeModule: "anime", surface: "immersive" });
    state.setMode("scene");
    state.rememberScroll({ y: 99 });
    const snapshot = state.snapshot();

    snapshot.modules.anime.scroll.y = 1000;
    expect(state.activeMemory.scroll.y).toBe(99);

    state.reset();
    state.restore({ ...snapshot, surface: "management" });
    expect(state.activeModule).toBe("anime");
    expect(state.surface).toBe("management");
    expect(state.activeMemory).toMatchObject({ mode: "scene", scroll: { x: 0, y: 1000 } });
  });

  it("clamps invalid scroll values and resets a single module", () => {
    const state = createMediaWorkspaceState();
    state.rememberScroll({ x: -4, y: Number.NaN });
    expect(state.activeMemory.scroll).toEqual({ x: 0, y: 0 });

    state.setMode("scene");
    state.resetModule();
    expect(state.activeMemory).toEqual({
      mode: "visual",
      selectedItemId: null,
      focusedItemId: null,
      scroll: { x: 0, y: 0 },
    });
  });
});
