import { get } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import {
  clearPlayerError,
  controlsIdleClass,
  controlsVisible,
  openMenuCount,
  playerError,
  reportPlayerError,
  retryCount,
  retryPlayback,
  setRetryHandler,
  shouldPauseIdleTimer,
  showSourceSuggest,
  type PlayerError,
} from "./player";

function makeError(overrides: Partial<PlayerError> = {}): PlayerError {
  return { kind: "HTTP_FORBIDDEN", message: "该源拒绝访问", occurredAt: 1, ...overrides };
}

beforeEach(() => {
  clearPlayerError();
  controlsVisible.set(true);
  openMenuCount.set(0);
  setRetryHandler(null);
});

describe("player store", () => {
  it("reportPlayerError 记录错误并重置 retryCount / showSourceSuggest", () => {
    retryCount.set(3);
    showSourceSuggest.set(true);
    reportPlayerError(makeError());

    expect(get(playerError)).toEqual(expect.objectContaining({ kind: "HTTP_FORBIDDEN" }));
    expect(get(retryCount)).toBe(0);
    expect(get(showSourceSuggest)).toBe(false);
  });

  it("retryPlayback 连续 2 次失败后 showSourceSuggest === true", async () => {
    setRetryHandler(() => Promise.resolve(false));

    await retryPlayback();
    expect(get(retryCount)).toBe(1);
    expect(get(showSourceSuggest)).toBe(false);

    await retryPlayback();
    expect(get(retryCount)).toBe(2);
    expect(get(showSourceSuggest)).toBe(true);
  });

  it("retryPlayback 成功后清除全部错误状态", async () => {
    reportPlayerError(makeError());
    setRetryHandler(() => Promise.resolve(true));

    await retryPlayback();
    expect(get(playerError)).toBeNull();
    expect(get(retryCount)).toBe(0);
    expect(get(showSourceSuggest)).toBe(false);
  });

  it("clearPlayerError 清空全部错误状态", () => {
    reportPlayerError(makeError());
    clearPlayerError();
    expect(get(playerError)).toBeNull();
    expect(get(retryCount)).toBe(0);
    expect(get(showSourceSuggest)).toBe(false);
  });

  it("openMenuCount 增减正确，>0 时 shouldPause 语义为暂停", () => {
    openMenuCount.update((n) => n + 1);
    expect(get(openMenuCount)).toBe(1);
    expect(shouldPauseIdleTimer({ openMenuCount: get(openMenuCount), isFullscreen: true, hasPlayerError: false })).toBe(true);

    openMenuCount.update((n) => n + 1);
    openMenuCount.update((n) => Math.max(0, n - 1));
    openMenuCount.update((n) => Math.max(0, n - 1));
    expect(get(openMenuCount)).toBe(0);
    expect(shouldPauseIdleTimer({ openMenuCount: get(openMenuCount), isFullscreen: true, hasPlayerError: false })).toBe(false);
  });

  it("shouldPauseIdleTimer 在非全屏 / 有错误时暂停", () => {
    expect(shouldPauseIdleTimer({ openMenuCount: 0, isFullscreen: false, hasPlayerError: false })).toBe(true);
    expect(shouldPauseIdleTimer({ openMenuCount: 0, isFullscreen: true, hasPlayerError: true })).toBe(true);
    expect(shouldPauseIdleTimer({ openMenuCount: 0, isFullscreen: true, hasPlayerError: false })).toBe(false);
  });

  it("controlsIdleClass：controlsVisible 变化时同步切换容器 idle class，destroy 后停止", () => {
    const node = document.createElement("div");
    document.body.append(node);
    const action = controlsIdleClass(node);

    expect(node.classList.contains("idle")).toBe(false);
    controlsVisible.set(false);
    expect(node.classList.contains("idle")).toBe(true);
    controlsVisible.set(true);
    expect(node.classList.contains("idle")).toBe(false);

    action.destroy();
    controlsVisible.set(false);
    expect(node.classList.contains("idle")).toBe(false);

    node.remove();
  });
});
