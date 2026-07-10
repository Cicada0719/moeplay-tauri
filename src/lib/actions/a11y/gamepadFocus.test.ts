import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createGamepadFocusRuntime,
  type GamepadClock,
  type GamepadLike,
  type GamepadNavigatorLike,
} from "./gamepadFocus";

class ManualClock implements GamepadClock {
  current = 0;
  private nextId = 0;
  private callbacks = new Map<number, (timestamp: number) => void>();

  now(): number {
    return this.current;
  }

  requestFrame(callback: (timestamp: number) => void): number {
    const id = ++this.nextId;
    this.callbacks.set(id, callback);
    return id;
  }

  cancelFrame(handle: number): void {
    this.callbacks.delete(handle);
  }

  frameAt(timestamp: number): void {
    this.current = timestamp;
    const callbacks = [...this.callbacks.values()];
    this.callbacks.clear();
    for (const callback of callbacks) callback(timestamp);
  }

  pendingFrames(): number {
    return this.callbacks.size;
  }
}

function createPad(): GamepadLike {
  return {
    connected: true,
    buttons: Array.from({ length: 16 }, () => ({ pressed: false, value: 0 })),
    axes: [0, 0],
  };
}

function setButton(pad: GamepadLike, index: number, pressed: boolean) {
  const button = pad.buttons[index] as { pressed: boolean; value?: number };
  button.pressed = pressed;
  button.value = pressed ? 1 : 0;
}

function setAxes(pad: GamepadLike, x: number, y: number) {
  (pad.axes as number[])[0] = x;
  (pad.axes as number[])[1] = y;
}

function createHarness() {
  const clock = new ManualClock();
  const pad = createPad();
  const pads: Array<GamepadLike | null> = [pad];
  const navigator: GamepadNavigatorLike = { getGamepads: () => pads };
  const connectionEvents = new EventTarget();
  const keyboardEvents = new EventTarget();
  const runtime = createGamepadFocusRuntime({
    navigator,
    clock,
    connectionEvents,
    keyboardEvents,
    hasFocus: () => true,
  });
  cleanups.push(() => runtime.destroy());
  return { clock, pad, pads, runtime, connectionEvents, keyboardEvents };
}

const cleanups: Array<() => void> = [];

afterEach(() => {
  while (cleanups.length) cleanups.pop()?.();
  document.body.replaceChildren();
});

describe("GamepadFocusRuntime scope routing", () => {
  it("uses priority, stack order, active zones and an exclusive overlay", () => {
    const { runtime, pad } = createHarness();
    const base = vi.fn();
    const wheel = vi.fn();
    const media = vi.fn();
    const overlay = vi.fn();

    runtime.registerScope({ left: base }, { id: "base", priority: 1 });
    runtime.registerScope({ left: wheel }, { id: "wheel", priority: 5, zone: "wheel" });
    runtime.registerScope({ left: media }, { id: "media", priority: 5, zone: "media" });

    runtime.setActiveZone("wheel");
    runtime.poll(0); // scope transition neutral barrier
    setButton(pad, 14, true);
    runtime.poll(10);
    expect(wheel).toHaveBeenCalledOnce();
    expect(base).not.toHaveBeenCalled();
    setButton(pad, 14, false);
    runtime.poll(20);

    runtime.setActiveZone("media");
    runtime.poll(30);
    setButton(pad, 14, true);
    runtime.poll(40);
    expect(media).toHaveBeenCalledOnce();
    setButton(pad, 14, false);
    runtime.poll(50);

    const overlayScope = runtime.registerScope(
      { left: overlay },
      { id: "overlay", overlay: true, priority: -100, zone: "other" },
    );
    runtime.poll(60);
    setButton(pad, 14, true);
    runtime.poll(70);
    expect(overlay).toHaveBeenCalledOnce();
    expect(media).toHaveBeenCalledOnce();

    setButton(pad, 14, false);
    runtime.poll(80);
    overlayScope.destroy();
    runtime.poll(90);
    setButton(pad, 14, true);
    runtime.poll(100);
    expect(media).toHaveBeenCalledTimes(2);
  });

  it("routes Start to a global handler when the active content scope does not handle it", () => {
    const { runtime, pad } = createHarness();
    const start = vi.fn();
    runtime.registerScope({ start }, { id: "global", priority: -100 });
    runtime.registerScope({ left: vi.fn() }, { id: "wheel", priority: 10, zone: "wheel" });
    runtime.setActiveZone("wheel");
    runtime.poll(0);

    setButton(pad, 9, true);
    runtime.poll(10);
    expect(start).toHaveBeenCalledOnce();
    runtime.poll(20);
    expect(start).toHaveBeenCalledOnce();

    setButton(pad, 9, false);
    runtime.poll(30);
    const overlay = runtime.registerScope({}, { id: "overlay-start-guard", overlay: true, priority: 100 });
    runtime.poll(40);
    setButton(pad, 9, true);
    runtime.poll(50);
    expect(start).toHaveBeenCalledOnce();
    overlay.destroy();
  });

  it("uses priority first, stack order as a tie-breaker, and resume brings a scope forward", () => {
    const { runtime, pad } = createHarness();
    const first = vi.fn();
    const second = vi.fn();
    const firstScope = runtime.registerScope({ launch: first }, { id: "first", priority: 10 });
    const secondScope = runtime.registerScope({ launch: second }, { id: "second", priority: 0 });

    runtime.poll(0);
    setButton(pad, 0, true);
    runtime.poll(1);
    expect(first).toHaveBeenCalledOnce();
    expect(second).not.toHaveBeenCalled();
    setButton(pad, 0, false);
    runtime.poll(2);

    secondScope.setPriority(10);
    runtime.poll(3);
    setButton(pad, 0, true);
    runtime.poll(4);
    expect(second).toHaveBeenCalledOnce();
    setButton(pad, 0, false);
    runtime.poll(5);

    firstScope.pause();
    firstScope.resume();
    runtime.poll(6);
    setButton(pad, 0, true);
    runtime.poll(7);
    expect(first).toHaveBeenCalledTimes(2);
    expect(second).toHaveBeenCalledOnce();
    secondScope.destroy();
  });
});

describe("GamepadFocusRuntime input", () => {
  it("reads all D-pad/stick directions with 320ms/100ms directional repeat", () => {
    const { runtime, pad } = createHarness();
    const up = vi.fn();
    const down = vi.fn();
    const left = vi.fn();
    const right = vi.fn();
    runtime.registerScope({ up, down, left, right });

    runtime.poll(0);
    setButton(pad, 14, true);
    runtime.poll(10);
    expect(left).toHaveBeenCalledTimes(1);
    runtime.poll(329);
    expect(left).toHaveBeenCalledTimes(1);
    runtime.poll(330);
    expect(left).toHaveBeenCalledTimes(2);
    runtime.poll(430);
    expect(left).toHaveBeenCalledTimes(3);

    setButton(pad, 14, false);
    setButton(pad, 12, true);
    runtime.poll(440);
    expect(up).toHaveBeenCalledOnce();
    setButton(pad, 12, false);
    setAxes(pad, 0.8, 0);
    runtime.poll(450);
    expect(right).toHaveBeenCalledOnce();
    setAxes(pad, 0.2, 0.2); // below release threshold
    runtime.poll(460);

    setAxes(pad, 0, 0.8);
    runtime.poll(470);
    expect(down).toHaveBeenCalledTimes(1);
    runtime.poll(789);
    expect(down).toHaveBeenCalledTimes(1);
    runtime.poll(790);
    expect(down).toHaveBeenCalledTimes(2);
  });

  it("keeps action buttons edge-triggered while directions repeat", () => {
    const { runtime, pad } = createHarness();
    const launch = vi.fn();
    runtime.registerScope({ launch });
    runtime.poll(0);

    setButton(pad, 0, true);
    runtime.poll(1);
    runtime.poll(500);
    expect(launch).toHaveBeenCalledOnce();
    setButton(pad, 0, false);
    runtime.poll(501);
    setButton(pad, 0, true);
    runtime.poll(502);
    expect(launch).toHaveBeenCalledTimes(2);
  });

  it("supports per-scope and global pause/resume without replaying held input", () => {
    const { runtime, pad } = createHarness();
    const left = vi.fn();
    const scope = runtime.registerScope({ left });
    runtime.poll(0);

    scope.pause();
    setButton(pad, 14, true);
    runtime.poll(1);
    expect(left).not.toHaveBeenCalled();
    scope.resume();
    runtime.poll(2);
    expect(left).not.toHaveBeenCalled();
    setButton(pad, 14, false);
    runtime.poll(3);
    setButton(pad, 14, true);
    runtime.poll(4);
    expect(left).toHaveBeenCalledOnce();

    runtime.pause();
    runtime.poll(5);
    expect(runtime.isPaused()).toBe(true);
    runtime.resume();
    runtime.poll(6);
    expect(left).toHaveBeenCalledOnce();
    setButton(pad, 14, false);
    runtime.poll(7);
    setButton(pad, 14, true);
    runtime.poll(8);
    expect(left).toHaveBeenCalledTimes(2);
  });

  it("lets keyboard take over and requires a held gamepad control to return neutral", () => {
    const { runtime, pad, keyboardEvents } = createHarness();
    const left = vi.fn();
    const right = vi.fn();
    const modes: string[] = [];
    runtime.subscribeInputMode((mode) => modes.push(mode));
    runtime.registerScope({ left, right });
    runtime.poll(0);

    setButton(pad, 14, true);
    runtime.poll(1);
    expect(runtime.getInputMode()).toBe("gamepad");
    expect(left).toHaveBeenCalledOnce();

    keyboardEvents.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    expect(runtime.getInputMode()).toBe("keyboard");
    runtime.poll(2);
    expect(left).toHaveBeenCalledOnce();

    setButton(pad, 14, false);
    runtime.poll(3);
    setButton(pad, 15, true);
    runtime.poll(4);
    expect(right).toHaveBeenCalledOnce();
    expect(runtime.getInputMode()).toBe("gamepad");
    expect(modes).toEqual(["keyboard", "gamepad", "keyboard", "gamepad"]);
  });

  it("does not blur the active element when the controller disconnects", () => {
    const { runtime, pads, connectionEvents, clock } = createHarness();
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();
    runtime.registerScope({ left: vi.fn() });
    expect(clock.pendingFrames()).toBe(1);

    pads.splice(0);
    connectionEvents.dispatchEvent(new Event("gamepaddisconnected"));
    expect(document.activeElement).toBe(trigger);
    expect(clock.pendingFrames()).toBe(0);
  });

  it("starts lazily on connection and uses the injected navigator and clock", () => {
    const { runtime, pad, pads, connectionEvents, clock } = createHarness();
    const left = vi.fn();
    pads.splice(0);
    runtime.registerScope({ left });
    expect(clock.pendingFrames()).toBe(0);

    pads.push(pad);
    connectionEvents.dispatchEvent(new Event("gamepadconnected"));
    expect(clock.pendingFrames()).toBe(1);
    clock.frameAt(0);
    setButton(pad, 14, true);
    clock.frameAt(10);
    expect(left).toHaveBeenCalledOnce();
  });
});
