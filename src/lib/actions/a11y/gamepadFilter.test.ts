import { beforeEach, describe, expect, it, vi } from "vitest";
import { createGamepadFocusRuntime, type GamepadClock, type GamepadLike } from "./gamepadFocus";

class Clock implements GamepadClock {
  now() { return 0; }
  requestFrame() { return 1; }
  cancelFrame() {}
}

function makePad(id: string, index = 0): GamepadLike {
  return {
    id,
    index,
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

describe("gamepad multi-pad merge（串流/虚拟手柄场景）", () => {
  beforeEach(() => localStorage.clear());

  it("占位虚拟手柄在前时，后面的真实手柄按键照常生效", () => {
    const stub = makePad("Virtual HID Device", 0); // UU远程类工具的占位虚拟手柄
    const real = makePad("Xbox 360 Controller", 1);
    const runtime = createGamepadFocusRuntime({
      navigator: { getGamepads: () => [stub, real] },
      clock: new Clock(),
      hasFocus: () => true,
    });
    const launch = vi.fn();
    const back = vi.fn();
    runtime.registerScope({ launch, back });
    runtime.poll(0);

    press(real, 0); // 真实手柄 A
    runtime.poll(1);
    runtime.poll(2);
    expect(launch).toHaveBeenCalledOnce();
    expect(back).not.toHaveBeenCalled();
    runtime.destroy();
  });

  it("多手柄各自套用布局：前置 Xbox 不影响任天堂手柄的 A=确认", () => {
    const xbox = makePad("Xbox 360 Controller", 0);
    const nintendo = makePad("Nintendo Switch Pro Controller", 1);
    const runtime = createGamepadFocusRuntime({
      navigator: { getGamepads: () => [xbox, nintendo] },
      clock: new Clock(),
      hasFocus: () => true,
    });
    const launch = vi.fn();
    runtime.registerScope({ launch });
    runtime.poll(0);

    press(nintendo, 1); // 任天堂 A（右键）→ 确认
    runtime.poll(1);
    runtime.poll(2);
    expect(launch).toHaveBeenCalledOnce();
    runtime.destroy();
  });

  it("任一连接的摇杆方向都生效", () => {
    const stub = makePad("Virtual HID Device", 0);
    const real = makePad("Xbox 360 Controller", 1);
    const runtime = createGamepadFocusRuntime({
      navigator: { getGamepads: () => [stub, real] },
      clock: new Clock(),
      hasFocus: () => true,
    });
    const down = vi.fn();
    runtime.registerScope({ down });
    runtime.poll(0);

    (real.axes as number[])[1] = 0.8; // 真实手柄左摇杆向下
    runtime.poll(1);
    expect(down).toHaveBeenCalled();
    runtime.destroy();
  });
});
