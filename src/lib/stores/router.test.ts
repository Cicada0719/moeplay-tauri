import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";
import RouterHarness from "./RouterHarness.test.svelte";

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
  applyHash,
  buildHash,
  captureRouteContext,
  closeOverlay,
  focusCurrentRouteSearch,
  handleBackNavigation,
  isKnownView,
  KNOWN_VIEWS,
  navigateTo,
  openOverlay,
  parseHash,
  resetRouterState,
  restoreRouteContext,
  routerStore,
} from "./router.svelte";
import { gameStore } from "./games.svelte";
import { uiStore } from "./ui.svelte";

describe("router", () => {
  let hash = "";

  function fakeWindow() {
    return {
      location: {
        get hash() { return hash; },
        set hash(value: string) { hash = value; },
      },
      requestAnimationFrame: (callback: FrameRequestCallback) => {
        callback(0);
        return 1;
      },
      setTimeout,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    } as unknown as Window & typeof globalThis;
  }

  beforeEach(() => {
    hash = "";
    vi.stubGlobal("window", fakeWindow());
    document.body.innerHTML = "";
    gameStore.selectGame("");
    resetRouterState();
  });

  afterEach(() => {
    resetRouterState();
    document.body.innerHTML = "";
    vi.unstubAllGlobals();
  });

  it("exposes known navigable views and rejects internal commands", () => {
    expect(KNOWN_VIEWS).toContain("home");
    expect(KNOWN_VIEWS).toContain("game-detail");
    expect(KNOWN_VIEWS).toContain("settings");
    expect(isKnownView("__tools")).toBe(false);
    expect(isKnownView("__bigpicture")).toBe(false);
    expect(isKnownView("narnia")).toBe(false);
  });

  it("parses and builds stable hashes including game entities", () => {
    expect(parseHash("#settings")).toEqual<AppRoute>({ view: "settings", params: {} });
    expect(parseHash("#loop")).toEqual<AppRoute>({ view: "records", params: {} });
    expect(parseHash("#game-detail?id=abc-123")).toEqual<AppRoute>({
      view: "game-detail",
      params: { gameId: "abc-123" },
    });
    expect(parseHash("#unknown-view")).toEqual<AppRoute>({ view: "home", params: {} });
    expect(buildHash("game-detail", { gameId: "abc-123" })).toBe("#game-detail?id=abc-123");
    expect(buildHash("__tools")).toBe("#home");
  });

  it("applies a hash and its entity to UI and game stores", () => {
    hash = "#game-detail?id=g-42";
    applyHash();
    expect(uiStore.currentView).toBe("game-detail");
    expect(gameStore.selectedId).toBe("g-42");
    expect(routerStore.current.entity).toEqual({ kind: "game", id: "g-42" });
  });

  it("captures focusKey and scrollOffset before navigation", () => {
    document.body.innerHTML = `
      <div data-route-view="home" data-route-scroll>
        <button data-focus-key="game:g-1">Game</button>
      </div>`;
    const root = document.querySelector<HTMLElement>("[data-route-scroll]")!;
    root.scrollTop = 240;
    document.querySelector<HTMLElement>("[data-focus-key]")!.focus();

    navigateTo("settings", { focus: "none" });

    expect(routerStore.history.at(-1)).toMatchObject({
      view: "home",
      focusKey: "data:game:g-1",
      scrollOffset: 240,
    });
  });

  it("returns detail to its exact source route and restores entity context", () => {
    navigateTo("records", { focus: "none" });
    gameStore.selectGame("g-7");
    navigateTo("game-detail", { entity: { kind: "game", id: "g-7" }, focus: "none" });

    expect(handleBackNavigation()).toBe("detail");
    expect(routerStore.current.view).toBe("records");
    expect(uiStore.currentView).toBe("records");
  });

  it("returns a utility subview to the nearest primary content root", () => {
    navigateTo("anime", { focus: "none" });
    navigateTo("settings", { focus: "none" });
    navigateTo("diagnostics", { focus: "none" });

    expect(handleBackNavigation()).toBe("subview");
    expect(routerStore.current.view).toBe("anime");
  });

  it("does not make Escape jump between primary content roots", () => {
    navigateTo("records", { focus: "none" });
    expect(handleBackNavigation()).toBe("none");
    expect(routerStore.current.view).toBe("records");
  });


  it("adapts legacy direct view writes and treats a detail-to-home write as source back", async () => {
    render(RouterHarness);
    await Promise.resolve();

    uiStore.currentView = "records";
    gameStore.selectGame("g-direct");
    uiStore.currentView = "game-detail";
    expect(routerStore.current).toMatchObject({
      view: "game-detail",
      entity: { kind: "game", id: "g-direct" },
    });

    uiStore.currentView = "home";
    expect(routerStore.current.view).toBe("records");
    expect(uiStore.currentView).toBe("records");
  });

  it("closes the top overlay before changing the route", () => {
    navigateTo("settings", { focus: "none" });
    const closeOuter = vi.fn();
    const closeInner = vi.fn();
    openOverlay({ id: "drawer", kind: "drawer" }, closeOuter);
    openOverlay({ id: "dialog", kind: "dialog" }, closeInner);

    expect(handleBackNavigation()).toBe("overlay");
    expect(closeInner).toHaveBeenCalledOnce();
    expect(closeOuter).not.toHaveBeenCalled();
    expect(routerStore.current.view).toBe("settings");
    expect(routerStore.overlayStack.map(item => item.id)).toEqual(["drawer"]);
    expect(closeOverlay("drawer")).toBe(true);
  });

  it("focuses search only inside the current route scope", () => {
    document.body.innerHTML = `
      <div data-route-view="anime"><input type="search" aria-label="番剧搜索"></div>
      <div data-route-view="comic"><input type="search" aria-label="漫画搜索"></div>`;
    navigateTo("comic", { replace: true, focus: "none" });

    expect(focusCurrentRouteSearch()).toBe(true);
    expect(document.activeElement).toHaveAttribute("aria-label", "漫画搜索");
  });

  it("restores a saved route focus target and scroll offset", () => {
    document.body.innerHTML = `
      <div data-route-view="home" data-route-scroll>
        <button data-focus-key="game:g-9">Game</button>
      </div>`;
    const route = {
      view: "home",
      entity: null,
      focusKey: "data:game:g-9",
      scrollOffset: 360,
    };

    expect(restoreRouteContext(route)).toBe(true);
    expect(document.querySelector<HTMLElement>("[data-route-scroll]")!.scrollTop).toBe(360);
    expect(document.activeElement).toHaveAttribute("data-focus-key", "game:g-9");
    expect(captureRouteContext(route)).toMatchObject(route);
  });
});
