/**
 * UI v2 motion preference. Keep this store framework-native so components can
 * react to changes without a writable Svelte store adapter.
 */
export type MotionPreference = "system" | "reduce" | "full";

type MotionMediaQuery = Pick<MediaQueryList, "matches" | "addEventListener" | "removeEventListener">;

let preference = $state<MotionPreference>("system");
let systemPrefersReduced = $state(false);
let mediaQuery: MotionMediaQuery | undefined;
let mediaQueryListener: ((event: MediaQueryListEvent) => void) | undefined;
let subscriberCount = 0;

function updateSystemPreference(matches: boolean) {
  systemPrefersReduced = matches;
}

function updateDocumentAttribute() {
  if (typeof document === "undefined") return;

  if (motionStore.reduced) {
    document.documentElement.dataset.motion = "reduce";
  } else {
    delete document.documentElement.dataset.motion;
  }
}

function setSystemPreference(matches: boolean) {
  updateSystemPreference(matches);
  updateDocumentAttribute();
}

function releaseSubscription() {
  subscriberCount = Math.max(0, subscriberCount - 1);
  if (subscriberCount !== 0 || !mediaQuery || !mediaQueryListener) return;

  mediaQuery.removeEventListener("change", mediaQueryListener);
  mediaQuery = undefined;
  mediaQueryListener = undefined;
}

/**
 * A small, shared reactive source of truth for motion decisions.
 *
 * Call `initialize()` from a long-lived layout (or a component's lifecycle)
 * to follow operating-system changes. The returned cleanup is reference-counted
 * so short-lived consumers do not remove another consumer's listener.
 */
export const motionStore = {
  get preference() {
    return preference;
  },
  set preference(value: MotionPreference) {
    preference = value;
    updateDocumentAttribute();
  },
  get systemPrefersReduced() {
    return systemPrefersReduced;
  },
  get reduced() {
    return preference === "reduce" || (preference === "system" && systemPrefersReduced);
  },
  get enabled() {
    return !this.reduced;
  },
  setPreference(value: MotionPreference) {
    this.preference = value;
  },
  initialize() {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return () => undefined;
    }

    subscriberCount += 1;

    if (!mediaQuery) {
      const query = window.matchMedia("(prefers-reduced-motion: reduce)");
      mediaQuery = query;
      mediaQueryListener = (event: MediaQueryListEvent) => setSystemPreference(event.matches);
      updateSystemPreference(query.matches);
      updateDocumentAttribute();
      query.addEventListener("change", mediaQueryListener);
    }

    return releaseSubscription;
  },
};