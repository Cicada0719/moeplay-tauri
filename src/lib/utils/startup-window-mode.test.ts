import { describe, expect, it } from "vitest";
import type { LogicalSize, PhysicalSize, Size } from "@tauri-apps/api/dpi";
import { applyStartupWindowMode, WINDOWED_STARTUP_SIZE } from "./startup-window-mode";

interface FakeWindowOptions {
  fullscreen?: boolean;
  maximized?: boolean;
  /** 模拟 Windows 早期 setFullscreen 静默失败：前 N 次调用不生效也不抛错 */
  silentSetFullscreenFailures?: number;
}

function createFakeWindow(opts: FakeWindowOptions = {}) {
  const calls: string[] = [];
  let fullscreen = opts.fullscreen ?? true;
  let maximized = opts.maximized ?? false;
  let silentFailuresLeft = opts.silentSetFullscreenFailures ?? 0;
  return {
    calls,
    isFullscreen: async () => {
      calls.push("isFullscreen");
      return fullscreen;
    },
    setFullscreen: async (value: boolean) => {
      calls.push(`setFullscreen(${value})`);
      if (silentFailuresLeft > 0) {
        silentFailuresLeft--;
        return;
      }
      fullscreen = value;
    },
    isMaximized: async () => {
      calls.push("isMaximized");
      return maximized;
    },
    maximize: async () => {
      calls.push("maximize");
      maximized = true;
    },
    unmaximize: async () => {
      calls.push("unmaximize");
      maximized = false;
    },
    setSize: async (size: LogicalSize | PhysicalSize | Size) => {
      const { width, height } = size as LogicalSize;
      calls.push(`setSize(${width}x${height})`);
    },
    center: async () => {
      calls.push("center");
    },
  };
}

const noWait = () => Promise.resolve();
const count = (calls: string[], name: string) => calls.filter((c) => c === name).length;

describe("applyStartupWindowMode", () => {
  it("windowed 成功路径：退出全屏 → 解除最大化 → 设定尺寸 → 居中，只尝试 1 次", async () => {
    const win = createFakeWindow({ fullscreen: true, maximized: true });
    const ok = await applyStartupWindowMode("windowed", win, { sleep: noWait });
    expect(ok).toBe(true);
    expect(win.calls).toEqual([
      "setFullscreen(false)",
      "isMaximized",
      "unmaximize",
      `setSize(${WINDOWED_STARTUP_SIZE.width}x${WINDOWED_STARTUP_SIZE.height})`,
      "center",
      "isFullscreen",
    ]);
  });

  it("windowed 非最大化窗口不调用 unmaximize", async () => {
    const win = createFakeWindow({ fullscreen: true, maximized: false });
    const ok = await applyStartupWindowMode("windowed", win, { sleep: noWait });
    expect(ok).toBe(true);
    expect(win.calls).not.toContain("unmaximize");
    expect(win.calls).toEqual([
      "setFullscreen(false)",
      "isMaximized",
      `setSize(${WINDOWED_STARTUP_SIZE.width}x${WINDOWED_STARTUP_SIZE.height})`,
      "center",
      "isFullscreen",
    ]);
  });

  it("windowed 重试路径：前 2 次 setFullscreen 静默失败，第 3 次生效", async () => {
    const win = createFakeWindow({ fullscreen: true, maximized: false, silentSetFullscreenFailures: 2 });
    const ok = await applyStartupWindowMode("windowed", win, { sleep: noWait });
    expect(ok).toBe(true);
    expect(count(win.calls, "setFullscreen(false)")).toBe(3);
    expect(count(win.calls, "center")).toBe(3);
  });

  it("放弃路径：永远达不到目标时恰好尝试 maxAttempts 次，返回 false 且不抛出", async () => {
    const win = createFakeWindow({ fullscreen: true, silentSetFullscreenFailures: 99 });
    const ok = await applyStartupWindowMode("windowed", win, { sleep: noWait });
    expect(ok).toBe(false);
    expect(count(win.calls, "setFullscreen(false)")).toBe(4);
  });

  it("fullscreen 路径：未全屏时调用一次 setFullscreen(true)", async () => {
    const win = createFakeWindow({ fullscreen: false });
    const ok = await applyStartupWindowMode("fullscreen", win, { sleep: noWait });
    expect(ok).toBe(true);
    expect(win.calls).toEqual(["isFullscreen", "setFullscreen(true)", "isFullscreen"]);
  });

  it("fullscreen 路径：已是全屏则不重复调用 setFullscreen", async () => {
    const win = createFakeWindow({ fullscreen: true });
    const ok = await applyStartupWindowMode("fullscreen", win, { sleep: noWait });
    expect(ok).toBe(true);
    expect(count(win.calls, "setFullscreen(true)")).toBe(0);
  });

  it("历史值兜底（如 dashboard）：退出全屏后最大化", async () => {
    const win = createFakeWindow({ fullscreen: true });
    const ok = await applyStartupWindowMode("dashboard", win, { sleep: noWait });
    expect(ok).toBe(true);
    expect(win.calls).toEqual(["setFullscreen(false)", "maximize", "isFullscreen"]);
  });
});
