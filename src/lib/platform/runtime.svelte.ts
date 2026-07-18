import { invokeCmd } from "../api/core";
import { desktopCapabilities } from "./windows/capabilities";

export type RuntimePlatform = "windows" | "android";

export interface PlatformCapabilities {
  platform: RuntimePlatform;
  orientationControl: boolean;
  steamIntegration: boolean;
  gameLaunch: boolean;
  localGameScan: boolean;
  emulatorImport: boolean;
  desktopWindowControl: boolean;
  tray: boolean;
  autostart: boolean;
  desktopUpdater: boolean;
  externalPlayer: boolean;
  fileSystemWatch: boolean;
}

const androidCapabilities: PlatformCapabilities = {
  platform: "android",
  orientationControl: true,
  steamIntegration: false,
  gameLaunch: false,
  localGameScan: false,
  emulatorImport: false,
  desktopWindowControl: false,
  tray: false,
  autostart: false,
  desktopUpdater: false,
  externalPlayer: false,
  fileSystemWatch: false,
};

export const MOBILE_ALLOWED_VIEWS = new Set([
  "home",
  "game-detail",
  "anime",
  "comic",
  "novel",
  "continue",
  "discovery",
  "downloads",
  "sources",
  "tasks",
  "settings",
]);

export function isViewSupportedOnPlatform(view: string, capabilities: PlatformCapabilities): boolean {
  if (capabilities.platform !== "android") return true;
  return MOBILE_ALLOWED_VIEWS.has(view);
}

function browserPlatform(): RuntimePlatform {
  if (typeof window === "undefined") return "windows";
  const forced = new URLSearchParams(window.location.search).get("platform");
  if (forced === "android") return "android";
  if (forced === "windows") return "windows";
  return /Android/i.test(window.navigator.userAgent) ? "android" : "windows";
}

let _capabilities = $state<PlatformCapabilities>(
  browserPlatform() === "android" ? androidCapabilities : desktopCapabilities,
);
let _loaded = $state(false);

function applyDocumentPlatform(capabilities: PlatformCapabilities) {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.platform = capabilities.platform;
  document.documentElement.classList.toggle("is-mobile-platform", capabilities.platform === "android");
}

function isPlatformCapabilities(value: unknown): value is PlatformCapabilities {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<PlatformCapabilities>;
  if (candidate.platform !== "windows" && candidate.platform !== "android") return false;
  const booleanKeys: (keyof Omit<PlatformCapabilities, "platform">)[] = [
    "orientationControl", "steamIntegration", "gameLaunch", "localGameScan", "emulatorImport",
    "desktopWindowControl", "tray", "autostart", "desktopUpdater", "externalPlayer", "fileSystemWatch",
  ];
  return booleanKeys.every((key) => typeof candidate[key] === "boolean");
}

export const platformStore = {
  get capabilities() { return _capabilities; },
  get platform() { return _capabilities.platform; },
  get isAndroid() { return _capabilities.platform === "android"; },
  get loaded() { return _loaded; },

  async initialize() {
    applyDocumentPlatform(_capabilities);
    if (_loaded) return _capabilities;
    try {
      const received = await invokeCmd<unknown>("get_platform_capabilities");
      if (isPlatformCapabilities(received)) _capabilities = received;
    } catch {
      // Browser previews and visual tests do not run a Tauri backend. The
      // user-agent/query-string inference above keeps those surfaces useful.
    }
    _loaded = true;
    applyDocumentPlatform(_capabilities);
    return _capabilities;
  },

  supports<K extends keyof Omit<PlatformCapabilities, "platform">>(capability: K) {
    return Boolean(_capabilities[capability]);
  },
};

export function defaultCapabilities(platform: RuntimePlatform): PlatformCapabilities {
  return platform === "android" ? { ...androidCapabilities } : { ...desktopCapabilities };
}
