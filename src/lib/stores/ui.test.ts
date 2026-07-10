import { afterEach, describe, expect, it, vi } from "vitest";
import { uiStore, type ViewChange } from "./ui.svelte";

describe("ui store shell state", () => {
  afterEach(() => {
    uiStore.setCurrentViewFromRouter("home");
    uiStore.closeDrawer();
    uiStore.consumeFocusSearchSignal();
  });

  it("emits direct and router view changes with their source", () => {
    const changes: ViewChange[] = [];
    const release = uiStore.subscribeViewChanges(change => changes.push(change));

    uiStore.currentView = "anime";
    uiStore.setCurrentViewFromRouter("comic");
    release();

    expect(changes).toEqual([
      { previous: "home", current: "anime", source: "direct" },
      { previous: "anime", current: "comic", source: "router" },
    ]);
  });

  it("keeps one canonical drawer state", () => {
    uiStore.openDrawer("tools");
    expect(uiStore.drawerOpen).toBe(true);
    expect(uiStore.drawerView).toBe("tools");
    uiStore.closeDrawer();
    expect(uiStore.drawerOpen).toBe(false);
    expect(uiStore.drawerView).toBeNull();
  });

  it("records the current page search scope without navigating", () => {
    uiStore.setCurrentViewFromRouter("anime");
    const before = uiStore.focusSearchSignal;
    uiStore.requestFocusSearch();
    expect(uiStore.currentView).toBe("anime");
    expect(uiStore.focusSearchScope).toBe("anime");
    expect(uiStore.focusSearchSignal).toBe(before + 1);
  });

  it("does not notify listeners for an unchanged view", () => {
    const listener = vi.fn();
    const release = uiStore.subscribeViewChanges(listener);
    uiStore.setCurrentViewFromRouter(uiStore.currentView);
    release();
    expect(listener).not.toHaveBeenCalled();
  });
});
