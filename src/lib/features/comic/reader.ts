import type { ComicProviderDescriptor, ComicReaderDecision, ComicResolvedTarget } from "./types";

const SAFE_HTTP_PROTOCOLS = new Set(["http:", "https:"]);
const FORBIDDEN_HEADER_NAMES = new Set(["cookie", "host", "origin", "referer", "content-length"]);

export type ComicReadingDirection = "vertical" | "left-to-right" | "right-to-left";
export type ComicReaderCommand =
  | "close"
  | "previous_page"
  | "next_page"
  | "previous_chapter"
  | "next_chapter"
  | "cycle_direction"
  | "zoom_in"
  | "zoom_out"
  | "reset_zoom"
  | "toggle_toolbar"
  | "first_page"
  | "last_page";

export const READER_ZOOM_MIN = 60;
export const READER_ZOOM_MAX = 200;
export const READER_ZOOM_STEP = 10;

function parsedUrl(value: string): URL | undefined {
  try { return new URL(value); } catch { return undefined; }
}

function normalizedHost(host: string): string {
  return host.trim().toLowerCase().replace(/^\[|\]$/g, "");
}

function normalizePath(path: string): string {
  const decoded = decodeURIComponent(path)
    .replace(/^file:\/\//i, "")
    .replace(/^\/([a-zA-Z]:\/)/, "$1")
    .replace(/\\/g, "/");
  const prefix = decoded.match(/^[a-zA-Z]:/)?.[0] ?? (decoded.startsWith("/") ? "/" : "");
  const segments = decoded.replace(/^[a-zA-Z]:/, "").split("/");
  const normalized: string[] = [];
  for (const segment of segments) {
    if (!segment || segment === ".") continue;
    if (segment === "..") normalized.pop();
    else normalized.push(segment);
  }
  const separator = prefix && prefix !== "/" ? "/" : "";
  return `${prefix}${separator}${normalized.join("/")}`.replace(/\/+$/, "").toLowerCase();
}

export function isPathInsideRoot(path: string, root?: string): boolean {
  if (!root) return false;
  const candidate = normalizePath(path);
  const boundary = normalizePath(root);
  return candidate === boundary || candidate.startsWith(`${boundary}/`);
}

export function localPathFromFileUrl(value: string): string | undefined {
  const url = parsedUrl(value);
  if (!url || url.protocol !== "file:") return undefined;
  const pathname = decodeURIComponent(url.pathname);
  return pathname.replace(/^\/([a-zA-Z]:\/)/, "$1");
}

export function isSafeRemoteUrl(value: string, provider: ComicProviderDescriptor, allowedHosts = provider.manifest.allowedHosts): boolean {
  const url = parsedUrl(value);
  if (!url || !SAFE_HTTP_PROTOCOLS.has(url.protocol) || url.username || url.password) return false;
  const host = normalizedHost(url.hostname);
  const allowed = new Set(allowedHosts.map(normalizedHost));
  if (provider.origin) {
    const origin = parsedUrl(provider.origin);
    if (origin && normalizedHost(origin.hostname) === host && origin.protocol === url.protocol && origin.port === url.port) return true;
  }
  return allowed.has(host);
}

export function sanitizeRequestHeaders(headers: [string, string][]): [string, string][] {
  return headers
    .slice(0, 32)
    .filter(([name, value]) => {
      const normalized = name.trim().toLowerCase();
      return /^[a-z0-9-]+$/.test(normalized)
        && !FORBIDDEN_HEADER_NAMES.has(normalized)
        && !/[\r\n]/.test(value);
    });
}

function safePage(value: string, provider: ComicProviderDescriptor): boolean {
  if (provider.kind === "local") {
    const path = localPathFromFileUrl(value);
    return Boolean(path && isPathInsideRoot(path, provider.localRoot));
  }
  return isSafeRemoteUrl(value, provider);
}

export function decideComicTarget(target: ComicResolvedTarget, provider: ComicProviderDescriptor): ComicReaderDecision {
  if (target.mode === "image_pages") {
    if (target.pages.length === 0) return { kind: "blocked", reason: "漫画源没有返回可阅读页面" };
    if (!target.pages.every((page) => safePage(page, provider))) {
      return { kind: "blocked", reason: "页面地址未通过漫画源安全边界校验" };
    }
    return { kind: "images", pages: [...target.pages], headers: sanitizeRequestHeaders(target.headers) };
  }

  if (target.mode === "native_file") {
    if (provider.kind !== "local" || !isPathInsideRoot(target.path, provider.localRoot)) {
      return { kind: "blocked", reason: "本地文件不在已配置的漫画根目录内" };
    }
    return { kind: "local_file", path: target.path };
  }

  if (target.mode === "webview") {
    const allowedHosts = target.allowedHosts.filter((host) => provider.manifest.allowedHosts.includes(host));
    return isSafeRemoteUrl(target.url, provider, allowedHosts)
      ? { kind: "external", url: target.url, reason: "该章节需要网页阅读器，将在系统浏览器中安全打开" }
      : { kind: "blocked", reason: "网页阅读地址未通过允许域名校验" };
  }

  if (target.mode === "external") {
    return isSafeRemoteUrl(target.url, provider)
      ? { kind: "external", url: target.url, reason: target.reason }
      : { kind: "blocked", reason: "外部阅读地址未通过漫画源安全边界校验" };
  }

  if (target.mode === "unsupported") return { kind: "blocked", reason: target.reason };
  return { kind: "blocked", reason: "该漫画源返回了不适用于漫画阅读的媒体目标" };
}

export function clampReaderPage(index: number, pageCount: number): number {
  if (!Number.isFinite(index) || pageCount <= 0) return 0;
  return Math.min(pageCount - 1, Math.max(0, Math.trunc(index)));
}

export function moveReaderPage(index: number, delta: number, pageCount: number): number {
  return clampReaderPage(index + delta, pageCount);
}

export function readerSwipePageDelta(
  deltaX: number,
  deltaY: number,
  direction: ComicReadingDirection,
  threshold = 48,
): -1 | 0 | 1 {
  if (direction === "vertical" || Math.abs(deltaX) < threshold || Math.abs(deltaX) <= Math.abs(deltaY) * 1.2) return 0;
  const swipedRight = deltaX > 0;
  if (direction === "right-to-left") return swipedRight ? 1 : -1;
  return swipedRight ? -1 : 1;
}

export function normalizeReaderZoom(value: number): number {
  if (!Number.isFinite(value)) return 100;
  const stepped = Math.round(value / READER_ZOOM_STEP) * READER_ZOOM_STEP;
  return Math.min(READER_ZOOM_MAX, Math.max(READER_ZOOM_MIN, stepped));
}

export function nextReadingDirection(direction: ComicReadingDirection): ComicReadingDirection {
  if (direction === "vertical") return "left-to-right";
  return direction === "left-to-right" ? "right-to-left" : "vertical";
}

export function readerDirectionLabel(direction: ComicReadingDirection): string {
  if (direction === "left-to-right") return "从左到右";
  if (direction === "right-to-left") return "从右到左";
  return "纵向滚动";
}

export function getReaderKeyboardCommand(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">,
  direction: ComicReadingDirection,
): ComicReaderCommand | undefined {
  if (event.ctrlKey || event.metaKey || event.altKey) return undefined;
  const key = event.key.toLowerCase();
  if (key === "escape") return "close";
  if (key === "t") return "toggle_toolbar";
  if (key === "d") return "cycle_direction";
  if (key === "+" || key === "=") return "zoom_in";
  if (key === "-" || key === "_") return "zoom_out";
  if (key === "0") return "reset_zoom";
  if (key === "[") return "previous_chapter";
  if (key === "]") return "next_chapter";
  if (key === "home") return "first_page";
  if (key === "end") return "last_page";
  if (key === "pageup" || key === "arrowup") return "previous_page";
  if (key === "pagedown" || key === "arrowdown" || key === " ") return "next_page";

  if (key === "arrowleft") {
    if (event.shiftKey) return direction === "right-to-left" ? "next_chapter" : "previous_chapter";
    return direction === "right-to-left" ? "next_page" : "previous_page";
  }
  if (key === "arrowright") {
    if (event.shiftKey) return direction === "right-to-left" ? "previous_chapter" : "next_chapter";
    return direction === "right-to-left" ? "previous_page" : "next_page";
  }
  return undefined;
}
