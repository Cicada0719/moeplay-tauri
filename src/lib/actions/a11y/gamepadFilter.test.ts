import { beforeEach, describe, expect, it, vi } from "vitest";
import { createGamepadFocusRuntime, type GamepadClock, type GamepadLike } from "./gamepadFocus";

class Clock implements GamepadClock {
  now() { return 0; }
  requestFrame() { return 1; }
  cancelFrame() {}
}

describe("gamepad view/filter button", () => {
  it("dispatches button 8 as the filter action on an edge", () => {
    const pad: GamepadLike = {
      connected: true,
      buttons: Array.from({ length: 16 }, () => ({ pressed: false, value: 0 })),
      axes: [0, 0],
    };
    const runtime = createGamepadFocusRuntime({
      navigator: { getGamepads: () => [pad] },
      clock: new Clock(),
      hasFocus: () => true,
    });
    const filter = vi.fn();
    runtime.registerScope({ filter });
    runtime.poll(0);
    (pad.buttons[8] as { pressed: boolean; value?: number }).pressed = true;
    runtime.poll(1);
    runtime.poll(2);
    expect(filter).toHaveBeenCalledOnce();
    runtime.destroy();
  });
});

describe("gamepad face-button layout（Xbox/任天堂）", () => {
  beforeEach(() => localStorage.clear());

  function makePad(id: string): GamepadLike {
    return {
      id,
      connected: true,
      buttons: Array.from({ length: 16 }, () => ({ pressed: false, value: 0 })),
      axes: [0, 0],
    };
  }

  function makeRuntime(pad: GamepadLike) {
    return createGamepadFocusRuntime({
      navigator: { getGamepads: () => [pad] },
      clock: new Clock(),
      hasFocus: () => true,
    });
  }

  function press(pad: GamepadLike, index: number) {
    (pad.buttons[index] as { pressed: boolean }).pressed = true;
  }
  function release(pad: GamepadLike, index: number) {
    (pad.buttons[index] as { pressed: boolean }).pressed = false;
  }

  it("任天堂手柄：物理右键(索引1)=确认、物理下键(索引0)=返回", () => {
    const pad = makePad("Nintendo Switch Pro Controller");
    const runtime = makeRuntime(pad);
    const launch = vi.fn();
    const back = vi.fn();
    runtime.registerScope({ launch, back });
    runtime.poll(0);

    press(pad, 1); // 任天堂 A（右键）→ 确认
    runtime.poll(1);
    runtime.poll(2);
    expect(launch).toHaveBeenCalledOnce();
    expect(back).not.toHaveBeenCalled();

    release(pad, 1);
    runtime.poll(3);
    press(pad, 0); // 任天堂 B（下键）→ 返回
    runtime.poll(4);
    runtime.poll(5);
    expect(back).toHaveBeenCalledOnce();
    expect(launch).toHaveBeenCalledOnce();
    runtime.destroy();
  });

  it("Xbox 手柄保持标准语义：索引0=确认、索引1=返回", () => {
    const pad = makePad("Xbox 360 Controller (XInput STANDARD GAMEPAD)");
    const runtime = makeRuntime(pad);
    const launch = vi.fn();
    const back = vi.fn();
    runtime.registerScope({ launch, back });
    runtime.poll(0);

    press(pad, 0);
    runtime.poll(1);
    runtime.poll(2);
    expect(launch).toHaveBeenCalledOnce();
    expect(back).not.toHaveBeenCalled();

    release(pad, 0);
    runtime.poll(3);
    press(pad, 1);
    runtime.poll(4);
    runtime.poll(5);
    expect(back).toHaveBeenCalledOnce();
    runtime.destroy();
  });

  it("用户强制 xbox 布局时，任天堂手柄也按 Xbox 语义", () => {
    localStorage.setItem("moeplay-gamepad-layout-v1", "xbox");
    const pad = makePad("Nintendo Switch Pro Controller");
    const runtime = makeRuntime(pad);
    const launch = vi.fn();
    runtime.registerScope({ launch });
    runtime.poll(0);

    press(pad, 0); // 强制 xbox：索引0=确认
    runtime.poll(1);
    runtime.poll(2);
    expect(launch).toHaveBeenCalledOnce();
    runtime.destroy();
  });
});
