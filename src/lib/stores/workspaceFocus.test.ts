import { beforeEach, describe, expect, it } from "vitest";
import { workspaceFocusStore } from "./workspaceFocus.svelte";

beforeEach(() => {
  localStorage.clear();
  workspaceFocusStore.reset();
});

describe("workspace focus mode", () => {
  it("supports every primary content category and shares game detail with home", () => {
    for (const view of ["home", "records", "anime", "comic", "novel", "game-detail"]) {
      expect(workspaceFocusStore.supports(view)).toBe(true);
    }
    expect(workspaceFocusStore.scopeFor("game-detail")).toBe("home");
    expect(workspaceFocusStore.supports("settings")).toBe(false);
  });

  it("keeps only one transient focus scope active", () => {
    expect(workspaceFocusStore.toggle("comic")).toBe(true);
    expect(workspaceFocusStore.set("anime", true)).toBe(true);
    expect(workspaceFocusStore.isEnabled("comic")).toBe(false);
    expect(workspaceFocusStore.isEnabled("anime")).toBe(true);
    expect(localStorage.getItem("moeplay.workspace-focus.v1")).toBeNull();
  });

  it("exits focus mode when navigation leaves the active scope", () => {
    workspaceFocusStore.set("comic", true);
    expect(workspaceFocusStore.reconcile("comic")).toBe("comic");
    expect(workspaceFocusStore.reconcile("records")).toBeNull();
    expect(workspaceFocusStore.isEnabled("comic")).toBe(false);
  });

  it("keeps the shared home scope between library and game detail only", () => {
    workspaceFocusStore.set("game-detail", true);
    expect(workspaceFocusStore.reconcile("home")).toBe("home");
    expect(workspaceFocusStore.isEnabled("game-detail")).toBe(true);
    workspaceFocusStore.set("home", false);
    expect(workspaceFocusStore.isEnabled("game-detail")).toBe(false);
  });

  it("clears the legacy persisted preference on reset", () => {
    localStorage.setItem("moeplay.workspace-focus.v1", JSON.stringify({ home: true }));
    workspaceFocusStore.reset();
    expect(localStorage.getItem("moeplay.workspace-focus.v1")).toBeNull();
  });
});
