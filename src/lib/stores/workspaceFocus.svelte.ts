export type WorkspaceFocusView = "home" | "records" | "anime" | "comic" | "novel";

const STORAGE_KEY = "moeplay.workspace-focus.v1";
const SUPPORTED_VIEWS = new Set<WorkspaceFocusView>(["home", "records", "anime", "comic", "novel"]);

function scopeForView(view: string): WorkspaceFocusView | null {
  if (view === "game-detail") return "home";
  return SUPPORTED_VIEWS.has(view as WorkspaceFocusView) ? view as WorkspaceFocusView : null;
}

function readStoredModes(): Partial<Record<WorkspaceFocusView, boolean>> {
  if (typeof localStorage === "undefined") return {};
  try {
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    if (!parsed || typeof parsed !== "object") return {};
    return Object.fromEntries(
      Object.entries(parsed).filter(([key, value]) => SUPPORTED_VIEWS.has(key as WorkspaceFocusView) && typeof value === "boolean"),
    ) as Partial<Record<WorkspaceFocusView, boolean>>;
  } catch {
    return {};
  }
}

let modes = $state<Partial<Record<WorkspaceFocusView, boolean>>>(readStoredModes());

function persist() {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(modes));
  } catch {
    // Layout preferences are optional; unavailable storage must not break navigation.
  }
}

export const workspaceFocusStore = {
  supports(view: string) {
    return scopeForView(view) != null;
  },
  scopeFor(view: string) {
    return scopeForView(view);
  },
  isEnabled(view: string) {
    const scope = scopeForView(view);
    return scope ? Boolean(modes[scope]) : false;
  },
  set(view: string, enabled: boolean) {
    const scope = scopeForView(view);
    if (!scope) return false;
    modes = { ...modes, [scope]: Boolean(enabled) };
    persist();
    return Boolean(modes[scope]);
  },
  toggle(view: string) {
    return this.set(view, !this.isEnabled(view));
  },
  reset() {
    modes = {};
    persist();
  },
};
