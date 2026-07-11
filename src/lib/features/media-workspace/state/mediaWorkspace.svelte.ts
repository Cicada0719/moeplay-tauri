import {
  CONTENT_MODULES,
  isAppSurface,
  isContentMode,
  isContentModule,
  type AppSurface,
  type ContentMode,
  type ContentModule,
  type MediaWorkspaceSnapshot,
  type ModuleWorkspaceMemory,
} from "../model/types";

const DEFAULT_MODE: ContentMode = "visual";

function createMemory(mode: ContentMode = DEFAULT_MODE): ModuleWorkspaceMemory {
  return {
    mode,
    selectedItemId: null,
    focusedItemId: null,
    scroll: { x: 0, y: 0 },
  };
}

function cloneMemory(memory: ModuleWorkspaceMemory): ModuleWorkspaceMemory {
  return {
    mode: memory.mode,
    selectedItemId: memory.selectedItemId,
    focusedItemId: memory.focusedItemId,
    scroll: { ...memory.scroll },
  };
}

function finiteCoordinate(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? Math.max(0, value) : 0;
}

function normalizedMemory(value: Partial<ModuleWorkspaceMemory> | undefined): ModuleWorkspaceMemory {
  return {
    mode: isContentMode(value?.mode) ? value.mode : DEFAULT_MODE,
    selectedItemId: typeof value?.selectedItemId === "string" ? value.selectedItemId : null,
    focusedItemId: typeof value?.focusedItemId === "string" ? value.focusedItemId : null,
    scroll: {
      x: finiteCoordinate(value?.scroll?.x),
      y: finiteCoordinate(value?.scroll?.y),
    },
  };
}

export interface MediaWorkspaceInitialState {
  activeModule?: ContentModule;
  surface?: AppSurface;
  modules?: Partial<Record<ContentModule, Partial<ModuleWorkspaceMemory>>>;
}

export interface MediaWorkspaceState {
  activeModule: ContentModule;
  surface: AppSurface;
  readonly activeMode: ContentMode;
  readonly activeMemory: ModuleWorkspaceMemory;
  memoryFor: (module: ContentModule) => ModuleWorkspaceMemory;
  setModule: (module: ContentModule) => void;
  setSurface: (surface: AppSurface) => void;
  setMode: (mode: ContentMode, module?: ContentModule) => void;
  selectItem: (itemId: string | null, module?: ContentModule) => void;
  focusItem: (itemId: string | null, module?: ContentModule) => void;
  rememberScroll: (scroll: Partial<{ x: number; y: number }>, module?: ContentModule) => void;
  resetModule: (module?: ContentModule) => void;
  reset: () => void;
  snapshot: () => MediaWorkspaceSnapshot;
  restore: (snapshot: Partial<MediaWorkspaceSnapshot>) => void;
}

/**
 * Creates an isolated rune-backed state source. Nothing is persisted here;
 * callers can serialize `snapshot()` using their existing settings/session layer.
 */
export function createMediaWorkspaceState(initial: MediaWorkspaceInitialState = {}): MediaWorkspaceState {
  let activeModule = $state<ContentModule>(isContentModule(initial.activeModule) ? initial.activeModule : "games");
  let surface = $state<AppSurface>(isAppSurface(initial.surface) ? initial.surface : "browse");
  let modules = $state<Record<ContentModule, ModuleWorkspaceMemory>>({
    games: normalizedMemory(initial.modules?.games),
    anime: normalizedMemory(initial.modules?.anime),
    comics: normalizedMemory(initial.modules?.comics),
  });

  function target(module?: ContentModule): ContentModule {
    return module ?? activeModule;
  }

  const state: MediaWorkspaceState = {
    get activeModule() {
      return activeModule;
    },
    set activeModule(value: ContentModule) {
      if (isContentModule(value)) activeModule = value;
    },
    get surface() {
      return surface;
    },
    set surface(value: AppSurface) {
      if (isAppSurface(value)) surface = value;
    },
    get activeMode() {
      return modules[activeModule].mode;
    },
    get activeMemory() {
      return modules[activeModule];
    },
    memoryFor(module: ContentModule) {
      return modules[module];
    },
    setModule(module: ContentModule) {
      if (isContentModule(module)) activeModule = module;
    },
    setSurface(nextSurface: AppSurface) {
      if (isAppSurface(nextSurface)) surface = nextSurface;
    },
    setMode(mode: ContentMode, module?: ContentModule) {
      if (!isContentMode(mode)) return;
      modules[target(module)].mode = mode;
    },
    selectItem(itemId: string | null, module?: ContentModule) {
      modules[target(module)].selectedItemId = itemId;
    },
    focusItem(itemId: string | null, module?: ContentModule) {
      modules[target(module)].focusedItemId = itemId;
    },
    rememberScroll(scroll: Partial<{ x: number; y: number }>, module?: ContentModule) {
      const memory = modules[target(module)];
      if (typeof scroll.x === "number" && Number.isFinite(scroll.x)) memory.scroll.x = Math.max(0, scroll.x);
      if (typeof scroll.y === "number" && Number.isFinite(scroll.y)) memory.scroll.y = Math.max(0, scroll.y);
    },
    resetModule(module?: ContentModule) {
      modules[target(module)] = createMemory();
    },
    reset() {
      activeModule = "games";
      surface = "browse";
      for (const module of CONTENT_MODULES) modules[module] = createMemory();
    },
    snapshot() {
      return {
        activeModule,
        surface,
        modules: {
          games: cloneMemory(modules.games),
          anime: cloneMemory(modules.anime),
          comics: cloneMemory(modules.comics),
        },
      };
    },
    restore(snapshot: Partial<MediaWorkspaceSnapshot>) {
      if (isContentModule(snapshot.activeModule)) activeModule = snapshot.activeModule;
      if (isAppSurface(snapshot.surface)) surface = snapshot.surface;
      if (snapshot.modules) {
        for (const module of CONTENT_MODULES) {
          if (snapshot.modules[module]) modules[module] = normalizedMemory(snapshot.modules[module]);
        }
      }
    },
  };

  return state;
}

export const mediaWorkspaceState = createMediaWorkspaceState();
