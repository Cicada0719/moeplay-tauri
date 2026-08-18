// 掌机小屏适配层：为 5.5-8 寸 Windows 掌机（720p-1080p 横屏）放大操作目标与字号。
// 结果写到 document.documentElement.dataset.handheld，CSS 经 [data-handheld="true"] 覆盖。
// 不改模式激活方式（START / dock 大屏 / startup_mode），纯显示适配。

export type HandheldMode = "auto" | "on" | "off";

const HANDHELD_STORAGE_KEY = "moeplay-handheld-mode-v1";
const HANDHELD_HINTS_KEY = "moeplay-handheld-hints-v1";
const HANDHELD_KEYBOARD_KEY = "moeplay-handheld-keyboard-v1";
const HANDHELD_PREFS_EVENT = "moeplay-handheld-prefs-changed";

/** 掌机模式联动偏好：手柄提示条常显（默认开） */
export function readHandheldHintsPreference(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(HANDHELD_HINTS_KEY) !== "off";
}

export function writeHandheldHintsPreference(on: boolean): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(HANDHELD_HINTS_KEY, on ? "on" : "off");
  notifyHandheldPrefsChanged();
}

/** 掌机模式联动偏好：输入框聚焦自动弹出屏幕键盘（默认开） */
export function readHandheldKeyboardPreference(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(HANDHELD_KEYBOARD_KEY) !== "off";
}

export function writeHandheldKeyboardPreference(on: boolean): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(HANDHELD_KEYBOARD_KEY, on ? "on" : "off");
  notifyHandheldPrefsChanged();
}

/** 偏好变更广播：App/键盘覆盖层等跨组件同步读取 */
export function notifyHandheldPrefsChanged(): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent(HANDHELD_PREFS_EVENT));
}

/** 订阅掌机联动偏好变更事件 */
export function onHandheldPrefsChanged(listener: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  const handler = () => listener();
  window.addEventListener(HANDHELD_PREFS_EVENT, handler);
  return () => window.removeEventListener(HANDHELD_PREFS_EVENT, handler);
}

export function readHandheldPreference(): HandheldMode {
  if (typeof localStorage === "undefined") return "auto";
  const raw = localStorage.getItem(HANDHELD_STORAGE_KEY);
  return raw === "on" || raw === "off" ? raw : "auto";
}

/**
 * 掌机判定（auto 模式）：横屏 && 宽 ≤ 1920，高度上限按指针类型分档——
 * 触屏设备（Steam Deck / ROG Ally 等掌机）放宽到 1080p；非触屏只认 ≤800p 矮屏，
 * 避免 1080p 桌面显示器误判。
 */
export function resolveHandheld(
  mode: HandheldMode,
  viewport: { width: number; height: number },
  coarsePointer: boolean,
): boolean {
  if (mode === "on") return true;
  if (mode === "off") return false;
  const landscape = viewport.width > viewport.height;
  if (!landscape || viewport.width > 1920) return false;
  return coarsePointer ? viewport.height <= 1080 : viewport.height <= 800;
}

let currentMode: HandheldMode = "auto";
const activeListeners = new Set<(on: boolean) => void>();
let lastApplied: boolean | null = null;

function applyHandheld(): void {
  if (typeof document === "undefined" || typeof window === "undefined") return;
  const coarse = window.matchMedia("(pointer: coarse)").matches;
  const on = resolveHandheld(
    currentMode,
    { width: window.innerWidth, height: window.innerHeight },
    coarse,
  );
  if (on) document.documentElement.dataset.handheld = "true";
  else delete document.documentElement.dataset.handheld;
  if (on !== lastApplied) {
    lastApplied = on;
    for (const listener of activeListeners) listener(on);
  }
}

/** 查询掌机模式当前是否生效（供非响应式代码读取） */
export function isHandheldActive(): boolean {
  return lastApplied === true;
}

/** 写入偏好并立即重新应用 */
export function writeHandheldPreference(mode: HandheldMode): void {
  currentMode = mode;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(HANDHELD_STORAGE_KEY, mode);
  }
  applyHandheld();
}

/** 安装 resize 监听并做初始应用；onChange 在开关切换时收到最新生效状态 */
export function installHandheldWatcher(onChange?: (on: boolean) => void): () => void {
  currentMode = readHandheldPreference();
  if (onChange) activeListeners.add(onChange);
  applyHandheld();
  const onResize = () => applyHandheld();
  window.addEventListener("resize", onResize);
  return () => {
    window.removeEventListener("resize", onResize);
    if (onChange) activeListeners.delete(onChange);
  };
}
