import { beforeEach, describe, expect, it } from "vitest";
import { workspaceFocusStore } from "./workspaceFocus.svelte";

beforeEach(() => {
  localStorage.clear();
  workspaceFocusStore.reset();
});

describe("workspace focus layout preferences", () => {
  it("supports every primary content category and shares game detail with home", () => {
    for (const view of ["home", "records", "anime", "comic", "novel", "game-detail"]) {
      expect(workspaceFocusStore.supports(view)).toBe(true);
    }
    expect(workspaceFocusStore.scopeFor("game-detail")).toBe("home");
    expect(workspaceFocusStore.supports("settings")).toBe(false);
  });

  it("persists independent per-category modes", () => {
    expect(workspaceFocusStore.toggle("comic")).toBe(true);
    expect(workspaceFocusStore.set("anime", true)).toBe(true);
    expect(workspaceFocusStore.isEnabled("comic")).toBe(true);
    expect(workspaceFocusStore.isEnabled("anime")).toBe(true);
    expect(workspaceFocusStore.isEnabled("home")).toBe(false);
    expect(JSON.parse(localStorage.getItem("moeplay.workspace-focus.v1") ?? "{}")).toEqual({ comic: true, anime: true });
  });

  it("uses the same game layout preference in library and archive views", () => {
    workspaceFocusStore.set("game-detail", true);
    expect(workspaceFocusStore.isEnabled("home")).toBe(true);
    expect(workspaceFocusStore.isEnabled("game-detail")).toBe(true);
    workspaceFocusStore.set("home", false);
    expect(workspaceFocusStore.isEnabled("game-detail")).toBe(false);
  });
});
