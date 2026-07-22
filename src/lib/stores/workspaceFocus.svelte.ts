export type WorkspaceFocusView = "home" | "records" | "anime" | "comic" | "novel";

const STORAGE_KEY = "moeplay.workspace-focus.v1";
const SUPPORTED_VIEWS = new Set<WorkspaceFocusView>(["home", "records", "anime", "comic", "novel"]);

function scopeForView(view: string): WorkspaceFocusView | null {
  if (view === "game-detail") return "home";
  return SUPPORTED_VIEWS.has(view as WorkspaceFocusView) ? view as WorkspaceFocusView : null;
}

let activeScope = $state<WorkspaceFocusView | null>(null);

export const workspaceFocusStore = {
  supports(view: string) {
    return scopeForView(view) != null;
  },
  scopeFor(view: string) {
    return scopeForView(view);
  },
  isEnabled(view: string) {
    const scope = scopeForView(view);
    return scope != null && activeScope === scope;
  },
  set(view: string, enabled: boolean) {
    const scope = scopeForView(view);
    if (!scope) return false;
    if (enabled) {
      activeScope = scope;
    } else if (activeScope === scope) {
      activeScope = null;
    }
    return activeScope === scope;
  },
  toggle(view: string) {
    return this.set(view, !this.isEnabled(view));
  },
  reconcile(view: string) {
    const scope = scopeForView(view);
    if (activeScope != null && activeScope !== scope) activeScope = null;
    return activeScope;
  },
  reset() {
    activeScope = null;
    if (typeof localStorage === "undefined") return;
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // Removing the legacy preference is best-effort only.
    }
  },
};
