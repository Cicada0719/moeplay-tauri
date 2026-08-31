import { beforeEach, describe, expect, it, vi } from "vitest";
import { createGamepadFocusRuntime, type GamepadClock, type GamepadLike } from "./gamepadFocus";
import { resetGamepadRemap, writeGamepadRemap } from "../../platform/gamepadRemap";
import { gamepadTuning } from "../../platform/gamepadTuning.svelte";

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

describe("gamepad 运行时集成：按键绑定与灵敏度调参", () => {
  beforeEach(() => {
    localStorage.clear();
    resetGamepadRemap();
    gamepadTuning.sensitivity = "standard";
    gamepadTuning.repeatSpeed = "standard";
  });

  it("重映射：把 launch 绑到物理 Y(3)，按 Y 触发启动", () => {
    writeGamepadRemap({ launch: 3 });
    const pad = makePad("Xbox 360 Controller");
    const runtime = makeRuntime(pad);
    const launch = vi.fn();
    const back = vi.fn();
    runtime.registerScope({ launch, back });
    runtime.poll(0);

    press(pad, 3); // 物理 Y
    runtime.poll(1);
    runtime.poll(2);
    expect(launch).toHaveBeenCalledOnce();
    expect(back).not.toHaveBeenCalled();
    runtime.destroy();
  });

  it("重映射优先于布局换位：任天堂手柄显式绑到物理下键(0)即启动", () => {
    writeGamepadRemap({ launch: 0 });
    const pad = makePad("Nintendo Switch Pro Controller");
    const runtime = makeRuntime(pad);
    const launch = vi.fn();
    runtime.registerScope({ launch });
    runtime.poll(0);

    press(pad, 0); // 任天堂默认下键=返回，但显式绑定覆盖
    runtime.poll(1);
    runtime.poll(2);
    expect(launch).toHaveBeenCalledOnce();
    runtime.destroy();
  });

  it("灵敏度：死区调紧(0.40)后 0.45 幅度的摇杆即可触发", () => {
    gamepadTuning.sensitivity = "tight";
    const pad = makePad("Xbox 360 Controller");
    const runtime = makeRuntime(pad);
    const down = vi.fn();
    runtime.registerScope({ down });
    runtime.poll(0);

    (pad.axes as number[])[1] = 0.45; // 标准死区 0.55 不触发，紧档 0.40 触发
    runtime.poll(1);
    expect(down).toHaveBeenCalledOnce();
    runtime.destroy();
  });

  it("灵敏度：死区调松(0.70)后 0.45 幅度不触发（防串流漂移）", () => {
    gamepadTuning.sensitivity = "loose";
    const pad = makePad("Xbox 360 Controller");
    const runtime = makeRuntime(pad);
    const down = vi.fn();
    runtime.registerScope({ down });
    runtime.poll(0);

    (pad.axes as number[])[1] = 0.45;
    runtime.poll(1);
    runtime.poll(2);
    expect(down).not.toHaveBeenCalled();
    runtime.destroy();
  });
});
