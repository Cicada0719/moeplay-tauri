// 掌机小屏适配层：为 5.5-8 寸 Windows 掌机（720p-1080p 横屏）放大操作目标与字号。
// 结果写到 document.documentElement.dataset.handheld，CSS 经 [data-handheld="true"] 覆盖。
// 不改模式激活方式（START / dock 大屏 / startup_mode），纯显示适配。

export type HandheldMode = "auto" | "on" | "off";

const HANDHELD_STORAGE_KEY = "moeplay-handheld-mode-v1";

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
}

/** 写入偏好并立即重新应用 */
export function writeHandheldPreference(mode: HandheldMode): void {
  currentMode = mode;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(HANDHELD_STORAGE_KEY, mode);
  }
  applyHandheld();
}

/** 安装 resize 监听并做初始应用；返回卸载函数 */
export function installHandheldWatcher(): () => void {
  currentMode = readHandheldPreference();
  applyHandheld();
  const onResize = () => applyHandheld();
  window.addEventListener("resize", onResize);
  return () => window.removeEventListener("resize", onResize);
}
