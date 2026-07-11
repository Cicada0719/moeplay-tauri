import { writable, type Readable } from "svelte/store";
import type {
  ConceptTemplate,
  ContentMode,
  ContentModule,
  ContentStageState,
  MotionQuality,
} from "../contracts";

const MODULES: ContentModule[] = ["games", "anime", "comics"];
const MODES: ContentMode[] = ["visual", "index", "scene"];
const TEMPLATES: ConceptTemplate[] = ["cinematic", "editorial", "kinetic"];
const QUALITIES: MotionQuality[] = ["full", "balanced", "reduced"];

export const DEFAULT_CONTENT_STAGE_STATE: ContentStageState = {
  template: "cinematic",
  module: "games",
  modeByModule: { games: "visual", anime: "visual", comics: "visual" },
  selectedIdByModule: { games: "", anime: "", comics: "" },
  focusKeyByModule: { games: "", anime: "", comics: "" },
  scrollPositionByModuleMode: {},
  quality: "full",
  muted: true,
  detailId: null,
  reviewOpen: false,
};

export interface ContentStageStore extends Readable<ContentStageState> {
  getSnapshot(): ContentStageState;
  setTemplate(template: ConceptTemplate): void;
  setModule(module: ContentModule): void;
  setMode(mode: ContentMode, module?: ContentModule): void;
  cycleMode(direction: -1 | 1, module?: ContentModule): void;
  select(id: string, module?: ContentModule): void;
  setFocus(key: string, module?: ContentModule): void;
  setScroll(position: number, module?: ContentModule, mode?: ContentMode): void;
  setQuality(quality: MotionQuality): void;
  setMuted(muted: boolean): void;
  toggleMuted(): void;
  openDetail(id: string): void;
  closeDetail(): void;
  setReviewOpen(open: boolean): void;
  patch(patch: Partial<ContentStageState>): void;
  reset(): void;
}

export interface CreateContentStageStoreOptions {
  initial?: Partial<ContentStageState>;
  storageKey?: string | false;
}

function scrollKey(module: ContentModule, mode: ContentMode): string {
  return `${module}:${mode}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function sanitizeState(value: unknown, fallback: ContentStageState): ContentStageState {
  if (!isRecord(value)) return fallback;
  const modeInput = isRecord(value.modeByModule) ? value.modeByModule : {};
  const selectedInput = isRecord(value.selectedIdByModule) ? value.selectedIdByModule : {};
  const focusInput = isRecord(value.focusKeyByModule) ? value.focusKeyByModule : {};
  const scrollInput = isRecord(value.scrollPositionByModuleMode) ? value.scrollPositionByModuleMode : {};

  const modes = Object.fromEntries(MODULES.map((module) => {
    const mode = modeInput[module];
    return [module, MODES.includes(mode as ContentMode) ? mode : fallback.modeByModule[module]];
  })) as Record<ContentModule, ContentMode>;

  const stringsFor = (input: Record<string, unknown>, source: Record<ContentModule, string>) =>
    Object.fromEntries(MODULES.map((module) => [module, typeof input[module] === "string" ? input[module] : source[module]])) as Record<ContentModule, string>;

  const positions: Record<string, number> = {};
  for (const [key, position] of Object.entries(scrollInput)) {
    if (typeof position === "number" && Number.isFinite(position) && position >= 0) positions[key] = position;
  }

  return {
    template: TEMPLATES.includes(value.template as ConceptTemplate) ? value.template as ConceptTemplate : fallback.template,
    module: MODULES.includes(value.module as ContentModule) ? value.module as ContentModule : fallback.module,
    modeByModule: modes,
    selectedIdByModule: stringsFor(selectedInput, fallback.selectedIdByModule),
    focusKeyByModule: stringsFor(focusInput, fallback.focusKeyByModule),
    scrollPositionByModuleMode: positions,
    quality: QUALITIES.includes(value.quality as MotionQuality) ? value.quality as MotionQuality : fallback.quality,
    muted: typeof value.muted === "boolean" ? value.muted : fallback.muted,
    detailId: typeof value.detailId === "string" ? value.detailId : null,
    reviewOpen: typeof value.reviewOpen === "boolean" ? value.reviewOpen : fallback.reviewOpen,
  };
}

function mergeInitial(initial?: Partial<ContentStageState>): ContentStageState {
  return sanitizeState({
    ...DEFAULT_CONTENT_STAGE_STATE,
    ...initial,
    modeByModule: { ...DEFAULT_CONTENT_STAGE_STATE.modeByModule, ...initial?.modeByModule },
    selectedIdByModule: { ...DEFAULT_CONTENT_STAGE_STATE.selectedIdByModule, ...initial?.selectedIdByModule },
    focusKeyByModule: { ...DEFAULT_CONTENT_STAGE_STATE.focusKeyByModule, ...initial?.focusKeyByModule },
    scrollPositionByModuleMode: { ...initial?.scrollPositionByModuleMode },
  }, DEFAULT_CONTENT_STAGE_STATE);
}

export function createContentStageStore(options: CreateContentStageStoreOptions = {}): ContentStageStore {
  const storageKey = options.storageKey === undefined ? "moeplay-concept-stage" : options.storageKey;
  let current = mergeInitial(options.initial);

  if (storageKey && typeof localStorage !== "undefined") {
    try {
      const stored = localStorage.getItem(storageKey);
      if (stored) current = sanitizeState(JSON.parse(stored), current);
    } catch {
      // Corrupt or unavailable storage must never block the concept shell.
    }
  }

  const store = writable(current);
  const commit = (next: ContentStageState) => {
    current = next;
    store.set(next);
    if (storageKey && typeof localStorage !== "undefined") {
      try { localStorage.setItem(storageKey, JSON.stringify(next)); } catch { /* best effort */ }
    }
  };
  const update = (recipe: (state: ContentStageState) => ContentStageState) => commit(recipe(current));
  const activeModule = (module?: ContentModule) => module ?? current.module;

  return {
    subscribe: store.subscribe,
    getSnapshot: () => current,
    setTemplate: (template) => update((state) => ({ ...state, template, detailId: null })),
    setModule: (module) => update((state) => ({ ...state, module, detailId: null })),
    setMode: (mode, module) => update((state) => {
      const target = activeModule(module);
      return { ...state, modeByModule: { ...state.modeByModule, [target]: mode }, detailId: null };
    }),
    cycleMode: (direction, module) => update((state) => {
      const target = activeModule(module);
      const index = MODES.indexOf(state.modeByModule[target]);
      const mode = MODES[(index + direction + MODES.length) % MODES.length];
      return { ...state, modeByModule: { ...state.modeByModule, [target]: mode }, detailId: null };
    }),
    select: (id, module) => update((state) => {
      const target = activeModule(module);
      return { ...state, selectedIdByModule: { ...state.selectedIdByModule, [target]: id } };
    }),
    setFocus: (key, module) => update((state) => {
      const target = activeModule(module);
      return { ...state, focusKeyByModule: { ...state.focusKeyByModule, [target]: key } };
    }),
    setScroll: (position, module, mode) => update((state) => {
      const targetModule = activeModule(module);
      const targetMode = mode ?? state.modeByModule[targetModule];
      return {
        ...state,
        scrollPositionByModuleMode: {
          ...state.scrollPositionByModuleMode,
          [scrollKey(targetModule, targetMode)]: Math.max(0, Number.isFinite(position) ? position : 0),
        },
      };
    }),
    setQuality: (quality) => update((state) => ({ ...state, quality })),
    setMuted: (muted) => update((state) => ({ ...state, muted })),
    toggleMuted: () => update((state) => ({ ...state, muted: !state.muted })),
    openDetail: (id) => update((state) => ({ ...state, detailId: id })),
    closeDetail: () => update((state) => ({ ...state, detailId: null })),
    setReviewOpen: (reviewOpen) => update((state) => ({ ...state, reviewOpen })),
    patch: (patch) => commit(sanitizeState({ ...current, ...patch }, current)),
    reset: () => commit(mergeInitial(options.initial)),
  };
}

export const contentStage = createContentStageStore();
export const contentStageScrollKey = scrollKey;
