import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";

vi.mock("./ui.svelte", () => {
  const store = { currentView: "home" };
  return { uiStore: store };
});

vi.mock("./games.svelte", () => {
  const store = {
    selectedId: "",
    selectGame(this: { selectedId: string }, id: string) {
      this.selectedId = id;
    },
  };
  return { gameStore: store };
});

import {
  type AppRoute,
  buildHash,
  isKnownView,
  KNOWN_VIEWS,
  parseHash,
  applyHash,
} from "./router.svelte";
import { uiStore } from "./ui.svelte";
import { gameStore } from "./games.svelte";

describe("router", () => {
  let hash = "";
  const listeners: Record<string, EventListener[]> = {};

  function fakeWindow() {
    return {
      location: {
        get hash() {
          return hash;
        },
        set hash(value: string) {
          hash = value;
        },
      },
      addEventListener: (event: string, listener: EventListener) => {
        listeners[event] = listeners[event] || [];
        listeners[event].push(listener);
      },
      removeEventListener: (event: string, listener: EventListener) => {
        if (listeners[event]) {
          listeners[event] = listeners[event].filter((l) => l !== listener);
        }
      },
    } as unknown as Window & typeof globalThis;
  }

  beforeEach(() => {
    hash = "";
    listeners.hashchange = [];
    listeners.popstate = [];
    vi.stubGlobal("window", fakeWindow());
    uiStore.currentView = "home";
    gameStore.selectGame("");
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("exposes known views including home and game-detail", () => {
    expect(KNOWN_VIEWS).toContain("home");
    expect(KNOWN_VIEWS).toContain("game-detail");
    expect(KNOWN_VIEWS).toContain("settings");
    expect(KNOWN_VIEWS).toContain("stats");
  });

  it("does not treat internal views as known", () => {
    expect(isKnownView("__tools")).toBe(false);
    expect(isKnownView("__bigpicture")).toBe(false);
  });

  it("rejects unknown views", () => {
    expect(isKnownView("narnia")).toBe(false);
  });

  it("parses plain view hashes", () => {
    expect(parseHash("#settings")).toEqual<AppRoute>({ view: "settings", params: {} });
    expect(parseHash("#stats")).toEqual<AppRoute>({ view: "stats", params: {} });
  });

  it("parses game detail hash with id", () => {
    expect(parseHash("#game-detail?id=abc-123")).toEqual<AppRoute>({
      view: "game-detail",
      params: { gameId: "abc-123" },
    });
  });

  it("falls back to home for empty or illegal hashes", () => {
    expect(parseHash("")).toEqual<AppRoute>({ view: "home", params: {} });
    expect(parseHash("#")).toEqual<AppRoute>({ view: "home", params: {} });
    expect(parseHash("#unknown-view")).toEqual<AppRoute>({ view: "home", params: {} });
  });

  it("builds hashes for known views", () => {
    expect(buildHash("home")).toBe("#home");
    expect(buildHash("settings")).toBe("#settings");
    expect(buildHash("game-detail", { gameId: "abc-123" })).toBe("#game-detail?id=abc-123");
  });

  it("builds home hash for unknown or internal views", () => {
    expect(buildHash("narnia")).toBe("#home");
    expect(buildHash("__tools")).toBe("#home");
  });

  it("applies hash to ui store", () => {
    hash = "#settings";
    applyHash();
    expect(uiStore.currentView).toBe("settings");
  });

  it("applies game detail hash to ui and game stores", () => {
    hash = "#game-detail?id=g-42";
    applyHash();
    expect(uiStore.currentView).toBe("game-detail");
    expect(gameStore.selectedId).toBe("g-42");
  });

  it("falls back to home when applying unknown hash", () => {
    hash = "#mordor";
    applyHash();
    expect(uiStore.currentView).toBe("home");
  });
});
