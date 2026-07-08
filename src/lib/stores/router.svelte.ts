import { DOCK_ITEMS, TOOL_ITEMS } from "../nav";
import { gameStore } from "./games.svelte";
import { uiStore } from "./ui.svelte";

export interface RouteParams {
  gameId?: string;
}

export interface AppRoute {
  view: string;
  params: RouteParams;
}

const INTERNAL_VIEWS = new Set(["__tools", "__bigpicture"]);

export const KNOWN_VIEWS: string[] = [
  "home",
  "game-detail",
  ...DOCK_ITEMS.filter((i) => !INTERNAL_VIEWS.has(i.view)).map((i) => i.view),
  ...TOOL_ITEMS.filter((i) => !INTERNAL_VIEWS.has(i.view)).map((i) => i.view),
];

const knownViewSet = new Set(KNOWN_VIEWS);

export function isKnownView(view: string): boolean {
  return knownViewSet.has(view);
}

export function parseHash(hash: string): AppRoute {
  const raw = hash.replace(/^#/, "").trim();
  if (!raw) return { view: "home", params: {} };

  const [viewPart, search] = raw.split("?");
  const decodedView = decodeURIComponent(viewPart);
  const view = decodedView === "loop" ? "records" : decodedView;
  const params: RouteParams = {};

  if (search) {
    const qs = new URLSearchParams(search);
    const gameId = qs.get("id");
    if (gameId) params.gameId = gameId;
  }

  if (!isKnownView(view)) return { view: "home", params: {} };
  return { view, params };
}

export function buildHash(view: string, params: RouteParams = {}): string {
  if (!isKnownView(view) || INTERNAL_VIEWS.has(view)) return "#home";
  if (view === "game-detail" && params.gameId) {
    return `#game-detail?id=${encodeURIComponent(params.gameId)}`;
  }
  return `#${encodeURIComponent(view)}`;
}

let _applying = false;
let _initialized = false;

export function applyHash() {
  if (typeof window === "undefined") return;
  _applying = true;
  try {
    const { view, params } = parseHash(window.location.hash);
    uiStore.currentView = view;
    if (view === "game-detail" && params.gameId) {
      gameStore.selectGame(params.gameId);
    }
  } finally {
    Promise.resolve().then(() => {
      _applying = false;
    });
  }
}

function syncHash() {
  if (_applying || !_initialized || typeof window === "undefined") return;
  const view = uiStore.currentView;
  const params: RouteParams = view === "game-detail" ? { gameId: gameStore.selectedId ?? undefined } : {};
  const next = buildHash(view, params);
  if (window.location.hash !== next) {
    window.location.hash = next;
  }
}

function onHashChange() {
  applyHash();
}

export function initRouter() {
  if (typeof window === "undefined") return;

  applyHash();
  _initialized = true;
  syncHash();

  window.addEventListener("hashchange", onHashChange);
  window.addEventListener("popstate", onHashChange);

  $effect(() => {
    uiStore.currentView;
    gameStore.selectedId;
    syncHash();
  });
}
