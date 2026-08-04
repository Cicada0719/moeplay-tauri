import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";

import AnimePlayer from "../../src/lib/components/anime/AnimePlayer.svelte";
import { setMockInvokeHandler, clearMockInvokeHandler } from "../../src/lib/api/core";
import { clearPlayerError, controlsVisible, openMenuCount } from "../../src/lib/stores/player";
import { settingsStore } from "../../src/lib/stores/settings.svelte";

/**
 * spec §6.2 泄漏回归：反复进入/退出播放页 10 次，document 上残留的全局 keydown
 * 监听器数量为 0（通过 addEventListener/removeEventListener spy 净计数验证）。
 *
 * 组件级测试：挂载真实 AnimePlayer（onMount 注册 document keydown / fullscreenchange、
 * window pointermove、idleTimer window keydown、focusTrap document keydown），
 * 每次 unmount 后 onDestroy 必须全部注销，10 轮后 keydown 净增监听为 0。
 * 真实 Tauri 进出播放页的窗口切换不在 Vitest 环境验证，此处覆盖 DOM 监听生命周期。
 */
describe("AnimePlayer 反复进出播放页 keydown 泄漏回归（spec §6.2）", () => {
  beforeEach(() => {
    clearPlayerError();
    controlsVisible.set(true);
    openMenuCount.set(0);
    settingsStore.settings.startup_mode = "windowed";
    setMockInvokeHandler(() => null);
  });

  afterEach(() => {
    clearMockInvokeHandler();
  });

  it("反复进入/退出播放页 10 次后 document 无残留 keydown 监听器", () => {
    const addSpy = vi.spyOn(document, "addEventListener");
    const removeSpy = vi.spyOn(document, "removeEventListener");
    const windowAddSpy = vi.spyOn(window, "addEventListener");
    const windowRemoveSpy = vi.spyOn(window, "removeEventListener");

    const keydownAdds = () => addSpy.mock.calls.filter(([type]) => type === "keydown").length;
    const keydownRemoves = () => removeSpy.mock.calls.filter(([type]) => type === "keydown").length;

    const baselineAdds = keydownAdds();
    const baselineRemoves = keydownRemoves();

    for (let i = 0; i < 10; i++) {
      const { unmount } = render(AnimePlayer);
      unmount();
    }

    // 净增 add 数必须等于净增 remove 数 → 残留 = 0
    expect(keydownAdds() - baselineAdds).toBe(keydownRemoves() - baselineRemoves);
    expect(keydownAdds() - baselineAdds).toBeGreaterThan(0); // 确保确实注册过（测试有效性）

    // fullscreenchange（document）监听同样应在 10 轮后全部注销
    const fullscreenAdds = addSpy.mock.calls.filter(([type]) => type === "fullscreenchange").length;
    const fullscreenRemoves = removeSpy.mock.calls.filter(([type]) => type === "fullscreenchange").length;
    expect(fullscreenAdds - fullscreenRemoves).toBe(0);

    // pointermove（window，顶部导航沉浸隐藏）监听同样应在 10 轮后全部注销
    const pointerAdds = windowAddSpy.mock.calls.filter(([type]) => type === "pointermove").length;
    const pointerRemoves = windowRemoveSpy.mock.calls.filter(([type]) => type === "pointermove").length;
    expect(pointerAdds - pointerRemoves).toBe(0);

    addSpy.mockRestore();
    removeSpy.mockRestore();
    windowAddSpy.mockRestore();
    windowRemoveSpy.mockRestore();
  });
});
