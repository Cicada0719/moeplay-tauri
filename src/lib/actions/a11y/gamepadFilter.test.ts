import { describe, expect, it, vi } from "vitest";
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
