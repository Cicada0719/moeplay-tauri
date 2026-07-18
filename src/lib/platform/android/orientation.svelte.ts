import { addPluginListener, invoke, type PluginListener } from "@tauri-apps/api/core";
import { platformStore } from "../runtime.svelte";

import { effectiveOrientation, enterVideoFullscreen, exitVideoFullscreen, type OrientationMode } from "./orientation-policy";

export type { OrientationMode } from "./orientation-policy";

const MODE_KEY = "moeplay.mobile.orientation";
const VIDEO_KEY = "moeplay.mobile.video-auto-landscape";

function readMode(): OrientationMode {
  if (typeof localStorage === "undefined") return "auto";
  const value = localStorage.getItem(MODE_KEY);
  return value === "portrait" || value === "landscape" ? value : "auto";
}

function readVideoPreference(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(VIDEO_KEY) !== "false";
}

let _mode = $state<OrientationMode>(readMode());
let _videoAutoLandscape = $state(readVideoPreference());
let _temporaryMode = $state<OrientationMode | null>(null);
let _initialized = false;
let _fullscreenActive = false;
let _nativeListener: PluginListener | null = null;

async function applyNative(mode: OrientationMode) {
  if (!platformStore.isAndroid || !platformStore.capabilities.orientationControl) return;
  try {
    await invoke("plugin:orientation|set_orientation", { request: { mode } });
  } catch (error) {
    console.warn("[orientation] native orientation command unavailable", error);
  }
}

function persist() {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(MODE_KEY, _mode);
  localStorage.setItem(VIDEO_KEY, String(_videoAutoLandscape));
}

function syncDocumentState() {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.orientationPreference = _mode;
  document.documentElement.dataset.orientationEffective = effectiveOrientation({ preferred: _mode, temporary: _temporaryMode, videoAutoLandscape: _videoAutoLandscape });
}

async function handleFullscreenChange() {
  if (typeof document === "undefined") return;
  const active = Boolean(document.fullscreenElement);
  if (active === _fullscreenActive) return;
  _fullscreenActive = active;
  if (active && _videoAutoLandscape) await orientationStore.enterVideoFullscreen();
  else if (!active) await orientationStore.exitVideoFullscreen();
}

export const orientationStore = {
  get mode() { return _mode; },
  get effectiveMode() { return _temporaryMode ?? _mode; },
  get videoAutoLandscape() { return _videoAutoLandscape; },

  async initialize() {
    if (_initialized) return;
    _initialized = true;
    syncDocumentState();
    if (typeof document !== "undefined") {
      document.addEventListener("fullscreenchange", handleFullscreenChange);
      document.addEventListener("visibilitychange", () => {
        if (document.visibilityState === "visible") void applyNative(_temporaryMode ?? _mode);
      });
    }
    if (platformStore.isAndroid && !_nativeListener) {
      _nativeListener = await addPluginListener<{ mode: OrientationMode; orientation: "portrait" | "landscape" }>(
        "orientation",
        "orientation-change",
        (payload) => {
          if (typeof document !== "undefined") document.documentElement.dataset.deviceOrientation = payload.orientation;
        },
      ).catch(() => null);
    }
    await applyNative(_mode);
  },

  async setMode(mode: OrientationMode) {
    // 能力守卫：无屏幕方向控制能力的端（如 Windows）误调时静默 no-op，不抛错。
    if (!platformStore.capabilities.orientationControl) return;
    _mode = mode;
    persist();
    syncDocumentState();
    if (!_temporaryMode) await applyNative(mode);
  },

  setVideoAutoLandscape(enabled: boolean) {
    // 能力守卫：无屏幕方向控制能力的端（如 Windows）误调时静默 no-op，不抛错。
    if (!platformStore.capabilities.orientationControl) return;
    _videoAutoLandscape = enabled;
    persist();
  },

  async enterVideoFullscreen() {
    const next = enterVideoFullscreen({ preferred: _mode, temporary: _temporaryMode, videoAutoLandscape: _videoAutoLandscape });
    if (next.temporary === _temporaryMode) return;
    _temporaryMode = next.temporary;
    syncDocumentState();
    await applyNative(effectiveOrientation(next));
  },

  async exitVideoFullscreen() {
    const next = exitVideoFullscreen({ preferred: _mode, temporary: _temporaryMode, videoAutoLandscape: _videoAutoLandscape });
    if (next.temporary === _temporaryMode) return;
    _temporaryMode = next.temporary;
    syncDocumentState();
    await applyNative(effectiveOrientation(next));
  },
};