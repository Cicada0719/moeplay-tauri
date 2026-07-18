import { describe, expect, it } from "vitest";
import {
  clampReaderPage,
  decideComicTarget,
  getReaderKeyboardCommand,
  isPathInsideRoot,
  moveReaderPage,
  nextReadingDirection,
  normalizeReaderZoom,
  readerDirectionLabel,
  readerSwipePageDelta,
  sanitizeRequestHeaders,
} from "./reader";
import type { ComicProviderDescriptor } from "./types";

const komga: ComicProviderDescriptor = {
  id: "komga:https://example.com",
  kind: "komga",
  name: "Komga",
  baseUrl: "https://example.com/app",
  origin: "https://example.com",
  authMode: "bearer",
  secretConfigured: true,
  manifest: {
    id: "komga:https://example.com",
    name: "Komga",
    resourceKinds: ["comic"],
    capabilities: ["search", "detail", "chapters", "resolve"],
    trust: "self_hosted",
    version: "batch2",
    enabled: true,
    requiresAuth: true,
    allowedHosts: ["example.com"],
  },
};

const local: ComicProviderDescriptor = {
  id: "local",
  kind: "local",
  name: "Local",
  localRoot: "C:\\Comics",
  authMode: "none",
  secretConfigured: false,
  manifest: {
    id: "local",
    name: "Local",
    resourceKinds: ["comic"],
    capabilities: ["search", "detail", "chapters", "resolve"],
    trust: "user_configured",
    version: "batch2",
    enabled: true,
    requiresAuth: false,
    allowedHosts: [],
  },
};

const key = (value: string, overrides: Partial<Pick<KeyboardEvent, "ctrlKey" | "metaKey" | "altKey" | "shiftKey">> = {}) => ({
  key: value,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...overrides,
});

describe("comic reader target safety", () => {
  it("allows same-origin image pages while rejecting cross-origin pages", () => {
    expect(decideComicTarget({
      mode: "image_pages",
      pages: ["https://example.com/api/books/1/pages/1"],
      headers: [["Authorization", "Bearer runtime-only"]],
    }, komga)).toEqual({
      kind: "images",
      pages: ["https://example.com/api/books/1/pages/1"],
      headers: [["Authorization", "Bearer runtime-only"]],
    });

    expect(decideComicTarget({
      mode: "image_pages",
      pages: ["https://evil.example/page.jpg"],
      headers: [],
    }, komga)).toEqual({ kind: "blocked", reason: "页面地址未通过漫画源安全边界校验" });
  });

  it("keeps local files inside the configured root and collapses traversal", () => {
    expect(isPathInsideRoot("C:\\Comics\\Series\\01.jpg", "C:\\Comics")).toBe(true);
    expect(isPathInsideRoot("C:\\Comics\\..\\Secrets\\token.txt", "C:\\Comics")).toBe(false);
    expect(decideComicTarget({
      mode: "image_pages",
      pages: ["file:///C:/Comics/Series/01.jpg"],
      headers: [],
    }, local).kind).toBe("images");
    expect(decideComicTarget({
      mode: "native_file",
      path: "C:\\Outside\\chapter.cbz",
    }, local).kind).toBe("blocked");
  });

  it("opens webview targets only through the provider allowlist", () => {
    expect(decideComicTarget({
      mode: "webview",
      url: "https://example.com/reader/1",
      allowedHosts: ["example.com"],
    }, komga).kind).toBe("external");
    expect(decideComicTarget({
      mode: "webview",
      url: "https://example.com.evil.test/reader/1",
      allowedHosts: ["example.com.evil.test"],
    }, komga).kind).toBe("blocked");
  });

  it("drops forbidden or newline-injected fetch headers", () => {
    expect(sanitizeRequestHeaders([
      ["Authorization", "Bearer ok"],
      ["Cookie", "session=secret"],
      ["X-Test", "safe\r\nInjected: bad"],
    ])).toEqual([["Authorization", "Bearer ok"]]);
  });
});

describe("comic reader interaction contract", () => {
  it("clamps page navigation and zoom", () => {
    expect(clampReaderPage(-3, 10)).toBe(0);
    expect(clampReaderPage(99, 10)).toBe(9);
    expect(moveReaderPage(4, -2, 10)).toBe(2);
    expect(moveReaderPage(9, 1, 10)).toBe(9);
    expect(normalizeReaderZoom(54)).toBe(60);
    expect(normalizeReaderZoom(137)).toBe(140);
    expect(normalizeReaderZoom(999)).toBe(200);
  });

  it("cycles reading directions with stable labels", () => {
    expect(nextReadingDirection("vertical")).toBe("left-to-right");
    expect(nextReadingDirection("left-to-right")).toBe("right-to-left");
    expect(nextReadingDirection("right-to-left")).toBe("vertical");
    expect(readerDirectionLabel("vertical")).toBe("纵向滚动");
    expect(readerDirectionLabel("right-to-left")).toBe("从右到左");
  });

  it("maps horizontal touch swipes to the selected reading direction", () => {
    expect(readerSwipePageDelta(-80, 8, "left-to-right")).toBe(1);
    expect(readerSwipePageDelta(80, 8, "left-to-right")).toBe(-1);
    expect(readerSwipePageDelta(80, 8, "right-to-left")).toBe(1);
    expect(readerSwipePageDelta(-80, 8, "right-to-left")).toBe(-1);
    expect(readerSwipePageDelta(30, 2, "left-to-right")).toBe(0);
    expect(readerSwipePageDelta(90, 100, "right-to-left")).toBe(0);
    expect(readerSwipePageDelta(90, 0, "vertical")).toBe(0);
  });

  it("maps keyboard input without stealing modified browser shortcuts", () => {
    expect(getReaderKeyboardCommand(key("Escape"), "vertical")).toBe("close");
    expect(getReaderKeyboardCommand(key("PageDown"), "vertical")).toBe("next_page");
    expect(getReaderKeyboardCommand(key("["), "vertical")).toBe("previous_chapter");
    expect(getReaderKeyboardCommand(key("]"), "vertical")).toBe("next_chapter");
    expect(getReaderKeyboardCommand(key("d"), "vertical")).toBe("cycle_direction");
    expect(getReaderKeyboardCommand(key("+"), "vertical")).toBe("zoom_in");
    expect(getReaderKeyboardCommand(key("t"), "vertical")).toBe("toggle_toolbar");
    expect(getReaderKeyboardCommand(key("ArrowLeft"), "right-to-left")).toBe("next_page");
    expect(getReaderKeyboardCommand(key("ArrowRight", { shiftKey: true }), "right-to-left")).toBe("previous_chapter");
    expect(getReaderKeyboardCommand(key("ArrowRight", { ctrlKey: true }), "vertical")).toBeUndefined();
  });
});
