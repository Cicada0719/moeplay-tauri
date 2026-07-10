import { untrack } from "svelte";
import { getRouteLevel, isPrimaryContentView, DOCK_ITEMS, TOOL_ITEMS } from "../nav";
import { gameStore } from "./games.svelte";
import { uiStore, type ViewChange } from "./ui.svelte";

export interface RouteParams {
  gameId?: string;
}

export interface AppRoute {
  view: string;
  params: RouteParams;
}

export interface RouteEntity {
  kind: string;
  id: string;
}

export interface RouteSnapshot {
  view: string;
  entity: RouteEntity | null;
  focusKey: string | null;
  scrollOffset: number;
}

export interface NavigateOptions {
  entity?: RouteEntity | null;
  focusKey?: string | null;
  scrollOffset?: number;
  replace?: boolean;
  focus?: "start" | "restore" | "none";
}

export interface OverlayEntry {
  id: string;
  kind: "dialog" | "drawer" | "search" | "virtual-keyboard" | "overlay";
  returnFocusKey: string | null;
}

export type BackNavigationResult = "overlay" | "detail" | "subview" | "none";

const INTERNAL_VIEWS = new Set(["__tools", "__bigpicture"]);
export const KNOWN_VIEWS: string[] = [
  "home",
  "game-detail",
  ...DOCK_ITEMS.filter((i) => !INTERNAL_VIEWS.has(i.view)).map((i) => i.view),
  ...TOOL_ITEMS.filter((i) => !INTERNAL_VIEWS.has(i.view)).map((i) => i.view),
];
const knownViewSet = new Set(KNOWN_VIEWS);
const MAX_HISTORY = 50;

let _currentRoute = $state<RouteSnapshot>(createRoute("home"));
let _history = $state<RouteSnapshot[]>([]);
let _overlayStack = $state<OverlayEntry[]>([]);
const overlayClosers = new Map<string, () => void>();
let _applyingHash = false;
let _initialized = false;
let releaseViewSubscription: (() => void) | null = null;
let releaseWindowListeners: (() => void) | null = null;
let focusRequestId = 0;

export const routerStore = {
  get current() { return _currentRoute; },
  get history() { return _history; },
  get overlayStack() { return _overlayStack; },
  get topOverlay() { return _overlayStack.at(-1) ?? null; },
  get canGoBack() {
    return _overlayStack.length > 0 || getRouteLevel(_currentRoute.view) !== "primary";
  },
};

function cloneRoute(route: RouteSnapshot): RouteSnapshot {
  return {
    view: route.view,
    entity: route.entity ? { ...route.entity } : null,
    focusKey: route.focusKey,
    scrollOffset: route.scrollOffset,
  };
}

function inferEntity(view: string): RouteEntity | null {
  if (view === "game-detail" && gameStore.selectedId) {
    return { kind: "game", id: gameStore.selectedId };
  }
  return null;
}

function createRoute(
  view: string,
  entity: RouteEntity | null = inferEntity(view),
  focusKey: string | null = null,
  scrollOffset = 0,
): RouteSnapshot {
  return { view, entity, focusKey, scrollOffset };
}

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

function cssEscape(value: string): string {
  if (typeof CSS !== "undefined" && typeof CSS.escape === "function") return CSS.escape(value);
  return value.replace(/["\\]/g, "\\$&");
}

function routeRoot(view = _currentRoute.view): HTMLElement | null {
  if (typeof document === "undefined") return null;
  const candidates = Array.from(
    document.querySelectorAll<HTMLElement>(`[data-route-view="${cssEscape(view)}"]`),
  );
  // Keyed route transitions briefly keep the outgoing view mounted. Prefer the
  // newest visible route so focus/search never targets the fading page.
  return candidates.reverse().find((candidate) => candidate.isConnected && candidate.getClientRects().length > 0)
    ?? candidates[0]
    ?? null;
}

function routeScrollContainer(root: HTMLElement | null): HTMLElement | null {
  if (!root) return null;
  return root.querySelector<HTMLElement>("[data-route-scroll]") ?? root;
}

function focusKeyFor(element: HTMLElement | null, root: HTMLElement | null): string | null {
  if (!element || !root || !root.contains(element)) return null;
  const keyed = element.closest<HTMLElement>("[data-focus-key]");
  if (keyed?.dataset.focusKey) return `data:${keyed.dataset.focusKey}`;
  if (element.id) return `id:${element.id}`;
  return null;
}

function elementForFocusKey(root: HTMLElement | null, key: string | null): HTMLElement | null {
  if (!root || !key) return null;
  if (key.startsWith("data:")) {
    const value = key.slice(5);
    return root.querySelector<HTMLElement>(`[data-focus-key="${cssEscape(value)}"]`);
  }
  if (key.startsWith("id:")) {
    const candidate = document.getElementById(key.slice(3));
    return candidate instanceof HTMLElement && root.contains(candidate) ? candidate : null;
  }
  return root.querySelector<HTMLElement>(`[data-focus-key="${cssEscape(key)}"]`);
}

export function captureRouteContext(route: RouteSnapshot = _currentRoute): RouteSnapshot {
  if (typeof document === "undefined") return cloneRoute(route);
  const root = routeRoot(route.view);
  const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  return {
    ...cloneRoute(route),
    focusKey: focusKeyFor(active, root) ?? route.focusKey,
    scrollOffset: routeScrollContainer(root)?.scrollTop ?? route.scrollOffset,
  };
}

function isFocusableSearch(element: Element | null): element is HTMLInputElement | HTMLTextAreaElement {
  if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) return false;
  return !element.disabled && element.getAttribute("aria-hidden") !== "true";
}

export function focusCurrentRouteSearch(view = _currentRoute.view): boolean {
  focusRequestId++;
  const root = routeRoot(view);
  if (!root) return false;
  const scoped = root.querySelector<HTMLElement>(`[data-search-scope="${cssEscape(view)}"]`)
    ?? root.querySelector<HTMLElement>("[data-route-search]");
  const candidates: (Element | null)[] = [
    scoped instanceof HTMLInputElement || scoped instanceof HTMLTextAreaElement ? scoped : scoped?.querySelector("input, textarea") ?? null,
    root.querySelector("input[type='search']"),
    root.querySelector("input[placeholder*='搜索']"),
    root.querySelector("textarea[placeholder*='搜索']"),
  ];
  const target = candidates.find(isFocusableSearch);
  if (!target) return false;
  target.focus({ preventScroll: true });
  if (typeof target.select === "function") target.select();
  return document.activeElement === target;
}

export function focusRouteStart(view = _currentRoute.view): boolean {
  const root = routeRoot(view);
  if (!root) return false;
  const target = root.querySelector<HTMLElement>("[data-route-focus], [data-page-title], h1") ?? root;
  if (!target.hasAttribute("tabindex") && !/^(A|BUTTON|INPUT|SELECT|TEXTAREA)$/.test(target.tagName)) {
    target.setAttribute("tabindex", "-1");
  }
  target.focus({ preventScroll: true });
  return document.activeElement === target;
}

export function restoreRouteContext(route: RouteSnapshot): boolean {
  const root = routeRoot(route.view);
  if (!root) return false;
  const scroll = routeScrollContainer(root);
  if (scroll) scroll.scrollTop = route.scrollOffset;
  const target = elementForFocusKey(root, route.focusKey);
  if (target) {
    target.focus({ preventScroll: true });
    return document.activeElement === target;
  }
  return focusRouteStart(route.view);
}

function afterRouteRender(callback: () => void) {
  if (typeof window === "undefined") return;
  const frame = typeof window.requestAnimationFrame === "function"
    ? window.requestAnimationFrame.bind(window)
    : (fn: FrameRequestCallback) => window.setTimeout(() => fn(Date.now()), 0);
  frame(() => frame(() => callback()));
}

function queueRouteFocus(route: RouteSnapshot, mode: NavigateOptions["focus"] = "start") {
  const requestId = ++focusRequestId;
  if (mode === "none") return;
  afterRouteRender(() => {
    if (requestId !== focusRequestId || _currentRoute.view !== route.view) return;
    if (mode === "restore") {
      restoreRouteContext(route);
      return;
    }
    const root = routeRoot(route.view);
    const active = typeof document !== "undefined" && document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    if (root && active && root.contains(active) && active !== root) return;
    focusRouteStart(route.view);
  });
}

function applyEntity(route: RouteSnapshot) {
  if (route.entity?.kind === "game") gameStore.selectGame(route.entity.id);
}

function setUiView(view: string) {
  // Route application must also work in lightweight consumers/tests that expose
  // only the public `currentView` property; setting it is the canonical UI contract.
  uiStore.currentView = view;
}

function pushHistory(route: RouteSnapshot) {
  _history = [..._history, cloneRoute(route)].slice(-MAX_HISTORY);
}

function syncHash() {
  if (_applyingHash || !_initialized || typeof window === "undefined") return;
  const params: RouteParams = _currentRoute.entity?.kind === "game"
    ? { gameId: _currentRoute.entity.id }
    : {};
  const next = buildHash(_currentRoute.view, params);
  if (window.location.hash !== next) window.location.hash = next;
}

function installRoute(route: RouteSnapshot, focus: NavigateOptions["focus"]) {
  _currentRoute = cloneRoute(route);
  applyEntity(_currentRoute);
  setUiView(_currentRoute.view);
  syncHash();
  queueRouteFocus(_currentRoute, focus);
}

export function navigateTo(view: string, options: NavigateOptions = {}): RouteSnapshot {
  const safeView = isKnownView(view) && !INTERNAL_VIEWS.has(view) ? view : "home";
  const next = createRoute(
    safeView,
    options.entity === undefined ? inferEntity(safeView) : options.entity,
    options.focusKey ?? null,
    options.scrollOffset ?? 0,
  );

  const current = captureRouteContext(_currentRoute);
  const sameEntity = current.entity?.kind === next.entity?.kind && current.entity?.id === next.entity?.id;
  if (current.view === next.view && sameEntity) {
    _currentRoute = { ...next, focusKey: current.focusKey, scrollOffset: current.scrollOffset };
    queueRouteFocus(_currentRoute, options.focus ?? "start");
    return cloneRoute(_currentRoute);
  }

  if (!options.replace) pushHistory(current);
  installRoute(next, options.focus ?? "start");
  return cloneRoute(_currentRoute);
}

function restorePreviousRoute(route: RouteSnapshot, result: BackNavigationResult): BackNavigationResult {
  installRoute(route, "restore");
  return result;
}

function nearestPrimaryHistoryIndex(): number {
  for (let index = _history.length - 1; index >= 0; index--) {
    if (isPrimaryContentView(_history[index].view)) return index;
  }
  return -1;
}

export function openOverlay(
  entry: Omit<OverlayEntry, "returnFocusKey"> & { returnFocusKey?: string | null },
  close?: () => void,
) {
  const normalized: OverlayEntry = { ...entry, returnFocusKey: entry.returnFocusKey ?? null };
  const stack = untrack(() => _overlayStack);
  const current = stack.at(-1);
  const alreadyTop = current?.id === normalized.id
    && current.kind === normalized.kind
    && current.returnFocusKey === normalized.returnFocusKey;
  if (!alreadyTop) {
    _overlayStack = [...stack.filter(item => item.id !== entry.id), normalized];
  }
  if (close) overlayClosers.set(entry.id, close);
}

export function closeOverlay(id: string): boolean {
  const stack = untrack(() => _overlayStack);
  const existed = stack.some(entry => entry.id === id);
  if (existed) _overlayStack = stack.filter(entry => entry.id !== id);
  overlayClosers.delete(id);
  return existed;
}

export function closeTopOverlay(): boolean {
  const top = _overlayStack.at(-1);
  if (!top) return false;
  const close = overlayClosers.get(top.id);
  _overlayStack = _overlayStack.slice(0, -1);
  overlayClosers.delete(top.id);
  close?.();
  return true;
}

export function handleBackNavigation(): BackNavigationResult {
  if (closeTopOverlay()) return "overlay";

  const level = getRouteLevel(_currentRoute.view);
  if (level === "primary") return "none";

  if (level === "detail" && _history.length > 0) {
    const previous = _history.at(-1)!;
    _history = _history.slice(0, -1);
    return restorePreviousRoute(previous, "detail");
  }

  const primaryIndex = nearestPrimaryHistoryIndex();
  if (primaryIndex >= 0) {
    const primary = _history[primaryIndex];
    _history = _history.slice(0, primaryIndex);
    return restorePreviousRoute(primary, "subview");
  }

  if (_currentRoute.view !== "home") {
    return restorePreviousRoute(createRoute("home"), "subview");
  }
  return "none";
}

function handleExternalViewChange(change: ViewChange) {
  if (change.source === "router" || _applyingHash) return;
  const previous = captureRouteContext(
    _currentRoute.view === change.previous ? _currentRoute : createRoute(change.previous),
  );

  if (getRouteLevel(previous.view) === "detail" && isPrimaryContentView(change.current) && _history.length > 0) {
    const parent = _history.at(-1)!;
    _history = _history.slice(0, -1);
    installRoute(parent, "restore");
    return;
  }

  pushHistory(previous);
  _currentRoute = createRoute(change.current);
  syncHash();
  queueRouteFocus(_currentRoute, "start");
}

export function applyHash() {
  if (typeof window === "undefined") return;
  _applyingHash = true;
  try {
    const { view, params } = parseHash(window.location.hash);
    const entity = view === "game-detail" && params.gameId
      ? { kind: "game", id: params.gameId }
      : null;
    _currentRoute = createRoute(view, entity);
    applyEntity(_currentRoute);
    setUiView(view);
    queueRouteFocus(_currentRoute, "start");
  } finally {
    _applyingHash = false;
  }
}

function onHashChange() {
  const parsed = parseHash(window.location.hash);
  const currentId = _currentRoute.entity?.kind === "game" ? _currentRoute.entity.id : undefined;
  if (parsed.view === _currentRoute.view && parsed.params.gameId === currentId) return;
  applyHash();
}

export function initRouter() {
  if (typeof window === "undefined") return () => {};
  if (_initialized) return () => {};

  applyHash();
  _initialized = true;
  releaseViewSubscription = uiStore.subscribeViewChanges(handleExternalViewChange);
  syncHash();

  window.addEventListener("hashchange", onHashChange);
  window.addEventListener("popstate", onHashChange);
  releaseWindowListeners = () => {
    window.removeEventListener("hashchange", onHashChange);
    window.removeEventListener("popstate", onHashChange);
  };

  $effect(() => {
    const selectedId = gameStore.selectedId;
    if (_currentRoute.view !== "game-detail" || !selectedId) return;
    if (_currentRoute.entity?.kind === "game" && _currentRoute.entity.id === selectedId) return;
    _currentRoute = { ..._currentRoute, entity: { kind: "game", id: selectedId } };
    syncHash();
  });

  return destroyRouter;
}

export function destroyRouter() {
  releaseViewSubscription?.();
  releaseViewSubscription = null;
  releaseWindowListeners?.();
  releaseWindowListeners = null;
  _initialized = false;
}

/** Test-only reset; kept explicit so history and overlay state never leak between cases. */
export function resetRouterState() {
  destroyRouter();
  _currentRoute = createRoute("home", null);
  _history = [];
  _overlayStack = [];
  overlayClosers.clear();
  _applyingHash = false;
  focusRequestId++;
  setUiView("home");
}
