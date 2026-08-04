import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { idleTimer, type IdleTimerOptions } from "../idleTimer";

function mount(node: HTMLElement, options: IdleTimerOptions) {
  return idleTimer(node, options);
}

function makeOptions(overrides: Partial<IdleTimerOptions> = {}): IdleTimerOptions {
  return { onIdle: vi.fn(), onActive: vi.fn(), ...overrides };
}

describe("idleTimer", () => {
  let node: HTMLElement;

  beforeEach(() => {
    vi.useFakeTimers();
    node = document.createElement("div");
    document.body.append(node);
  });

  afterEach(() => {
    vi.useRealTimers();
    node.remove();
  });

  it("挂载后 3s 无事件触发 onIdle 恰好一次（fake timers）", () => {
    const opts = makeOptions({ timeout: 3000 });
    const action = mount(node, opts);
    vi.advanceTimersByTime(2999);
    expect(opts.onIdle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(opts.onIdle).toHaveBeenCalledTimes(1);
    // 空闲态下不再自行触发
    vi.advanceTimersByTime(5000);
    expect(opts.onIdle).toHaveBeenCalledTimes(1);
    action.destroy();
  });

  it("2.9s 时触发 mousemove，第 3s 不触发 onIdle，第 5.9s 触发", () => {
    const opts = makeOptions({ timeout: 3000 });
    const action = mount(node, opts);
    vi.advanceTimersByTime(2900);
    node.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
    vi.advanceTimersByTime(100); // 累计 3000ms
    expect(opts.onIdle).not.toHaveBeenCalled();
    vi.advanceTimersByTime(2900); // 累计 5900ms
    expect(opts.onIdle).toHaveBeenCalledTimes(1);
    action.destroy();
  });

  it("idle 态下 mousemove / keydown 先调 onActive 再重置计时；连续事件不重复调 onActive", () => {
    const opts = makeOptions({ timeout: 3000 });
    const action = mount(node, opts);
    vi.advanceTimersByTime(3000);
    expect(opts.onIdle).toHaveBeenCalledTimes(1);

    node.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
    expect(opts.onActive).toHaveBeenCalledTimes(1);

    window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true }));
    expect(opts.onActive).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(3000);
    expect(opts.onIdle).toHaveBeenCalledTimes(2);
    action.destroy();
  });

  it("shouldPause() 返回 true 时到点不触发 onIdle，轮询至返回 false 后正常隐藏", () => {
    let paused = true;
    const opts = makeOptions({ timeout: 3000, shouldPause: () => paused });
    const action = mount(node, opts);

    vi.advanceTimersByTime(3000);
    expect(opts.onIdle).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1000); // 轮询中
    expect(opts.onIdle).not.toHaveBeenCalled();

    paused = false;
    vi.advanceTimersByTime(500); // 下一轮 poll 检测到解除 → 重新计时
    vi.advanceTimersByTime(3000); // 重新计时到点
    expect(opts.onIdle).toHaveBeenCalledTimes(1);
    action.destroy();
  });

  it("destroy() 后定时器与 window keydown 监听全部移除，事件不再触发（无泄漏）", () => {
    const opts = makeOptions({ timeout: 3000 });
    const addSpy = vi.spyOn(window, "addEventListener");
    const removeSpy = vi.spyOn(window, "removeEventListener");
    const action = mount(node, opts);

    const keydownCalls = addSpy.mock.calls.filter(([type]) => type === "keydown");
    const keydownHandler = keydownCalls.at(-1)?.[1] as EventListener | undefined;

    action.destroy();
    if (keydownHandler) {
      expect(removeSpy).toHaveBeenCalledWith("keydown", keydownHandler);
    }
    expect(removeSpy).toHaveBeenCalledWith("keydown", expect.any(Function));

    vi.advanceTimersByTime(5000);
    node.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
    window.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true }));
    expect(opts.onIdle).not.toHaveBeenCalled();
    expect(opts.onActive).not.toHaveBeenCalled();

    addSpy.mockRestore();
    removeSpy.mockRestore();
  });
});
