import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createGamepadFocusRuntime,
  type GamepadClock,
  type GamepadLike,
} from "../../actions/a11y/gamepadFocus";
import { attachGamepad } from "./useGamepad.svelte";

class IdleClock implements GamepadClock {
  now() { return 0; }
  requestFrame() { return 1; }
  cancelFrame() {}
}

function harness() {
  const pad: GamepadLike = {
    connected: true,
    buttons: Array.from({ length: 16 }, () => ({ pressed: false, value: 0 })),
    axes: [0, 0],
  };
  const runtime = createGamepadFocusRuntime({
    navigator: { getGamepads: () => [pad] },
    clock: new IdleClock(),
    hasFocus: () => true,
  });
  cleanups.push(() => runtime.destroy());
  return { pad, runtime };
}

function press(pad: GamepadLike, index: number, value: boolean) {
  const button = pad.buttons[index] as { pressed: boolean; value?: number };
  button.pressed = value;
  button.value = value ? 1 : 0;
}

const cleanups: Array<() => void> = [];
afterEach(() => {
  while (cleanups.length) cleanups.pop()?.();
});

describe("attachGamepad compatibility", () => {
  it("remains a callable detach function and exposes pause/resume controls", () => {
    const { runtime } = harness();
    const attachment = attachGamepad({}, { runtime, id: "compat" });
    expect(typeof attachment).toBe("function");
    expect(attachment.id).toBe("compat");
    attachment.pause();
    attachment.resume();
    attachment.activate();
    attachment.setZone("wheel");
    attachment.setPriority(10);
    attachment.setOverlay(true);
    attachment.setEnabled(true);
    attachment();
    attachment();
    expect(runtime.getActiveScopeId()).toBeNull();
  });

  it("maps D-pad up/down to legacy pageLeft/pageRight handlers", () => {
    const { runtime, pad } = harness();
    const pageLeft = vi.fn();
    const pageRight = vi.fn();
    const detach = attachGamepad({ pageLeft, pageRight }, { runtime });
    runtime.poll(0);

    press(pad, 12, true);
    runtime.poll(1);
    expect(pageLeft).toHaveBeenCalledOnce();
    press(pad, 12, false);
    runtime.poll(2);
    press(pad, 13, true);
    runtime.poll(3);
    expect(pageRight).toHaveBeenCalledOnce();
    detach();
  });

  it("routes only to the newest equal-priority legacy attachment", () => {
    const { runtime, pad } = harness();
    const background = vi.fn();
    const foreground = vi.fn();
    const detachBackground = attachGamepad({ back: background }, { runtime });
    const detachForeground = attachGamepad({ back: foreground }, { runtime });
    runtime.poll(0);

    press(pad, 1, true);
    runtime.poll(1);
    expect(foreground).toHaveBeenCalledOnce();
    expect(background).not.toHaveBeenCalled();

    press(pad, 1, false);
    runtime.poll(2);
    detachForeground();
    runtime.poll(3);
    press(pad, 1, true);
    runtime.poll(4);
    expect(background).toHaveBeenCalledOnce();
    detachBackground();
  });
});
