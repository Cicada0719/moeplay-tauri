import type { Page } from "@playwright/test";

export const STANDARD_GAMEPAD_BUTTONS = {
  a: 0,
  b: 1,
  x: 2,
  y: 3,
  leftBumper: 4,
  rightBumper: 5,
  back: 8,
  start: 9,
  dpadUp: 12,
  dpadDown: 13,
  dpadLeft: 14,
  dpadRight: 15,
} as const;

export type GamepadButtonName = keyof typeof STANDARD_GAMEPAD_BUTTONS;

export async function installGamepadMock(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const buttons = Array.from({ length: 17 }, () => ({
      pressed: false,
      touched: false,
      value: 0,
    }));
    const gamepad = {
      axes: [0, 0, 0, 0],
      buttons,
      connected: false,
      hapticActuators: [],
      id: "MoePlay Deterministic Gamepad",
      index: 0,
      mapping: "standard",
      timestamp: 0,
      vibrationActuator: null,
    };

    const emitConnection = (type: "gamepadconnected" | "gamepaddisconnected") => {
      const event = new Event(type);
      Object.defineProperty(event, "gamepad", { value: gamepad });
      window.dispatchEvent(event);
    };
    const touch = () => {
      gamepad.timestamp = performance.now();
    };

    const api = {
      connect() {
        if (gamepad.connected) return;
        gamepad.connected = true;
        touch();
        emitConnection("gamepadconnected");
      },
      disconnect() {
        if (!gamepad.connected) return;
        gamepad.connected = false;
        touch();
        emitConnection("gamepaddisconnected");
      },
      setButton(index: number, pressed: boolean, value = pressed ? 1 : 0) {
        const button = buttons[index];
        if (!button) throw new Error(`Unknown gamepad button index: ${index}`);
        button.pressed = pressed;
        button.touched = pressed;
        button.value = value;
        touch();
      },
      setAxis(index: number, value: number) {
        if (index < 0 || index >= gamepad.axes.length) {
          throw new Error(`Unknown gamepad axis index: ${index}`);
        }
        gamepad.axes[index] = Math.max(-1, Math.min(1, value));
        touch();
      },
      reset() {
        for (const button of buttons) {
          button.pressed = false;
          button.touched = false;
          button.value = 0;
        }
        gamepad.axes.fill(0);
        touch();
      },
    };

    Object.defineProperty(navigator, "getGamepads", {
      configurable: true,
      value: () => (gamepad.connected ? [gamepad, null, null, null] : [null, null, null, null]),
    });
    Object.defineProperty(window, "__MOEPLAY_GAMEPAD_MOCK__", {
      configurable: true,
      value: api,
    });
  });
}

export class GamepadController {
  constructor(private readonly page: Page) {}

  async connect(): Promise<void> {
    await this.call("connect");
  }

  async disconnect(): Promise<void> {
    await this.call("disconnect");
  }

  async press(button: GamepadButtonName | number, holdMs = 100): Promise<void> {
    const index = typeof button === "number" ? button : STANDARD_GAMEPAD_BUTTONS[button];
    await this.setButton(index, true);
    await this.page.waitForTimeout(holdMs);
    await this.setButton(index, false);
  }

  async setButton(index: number, pressed: boolean, value?: number): Promise<void> {
    await this.page.evaluate(
      ({ buttonIndex, isPressed, buttonValue }) => {
        (window as unknown as {
          __MOEPLAY_GAMEPAD_MOCK__: {
            setButton(index: number, pressed: boolean, value?: number): void;
          };
        }).__MOEPLAY_GAMEPAD_MOCK__.setButton(buttonIndex, isPressed, buttonValue);
      },
      { buttonIndex: index, isPressed: pressed, buttonValue: value },
    );
  }

  async setAxis(index: number, value: number): Promise<void> {
    await this.page.evaluate(
      ({ axisIndex, axisValue }) => {
        (window as unknown as {
          __MOEPLAY_GAMEPAD_MOCK__: { setAxis(index: number, value: number): void };
        }).__MOEPLAY_GAMEPAD_MOCK__.setAxis(axisIndex, axisValue);
      },
      { axisIndex: index, axisValue: value },
    );
  }

  async reset(): Promise<void> {
    await this.call("reset");
  }

  private async call(method: "connect" | "disconnect" | "reset"): Promise<void> {
    await this.page.evaluate((methodName) => {
      const api = (window as unknown as {
        __MOEPLAY_GAMEPAD_MOCK__: Record<string, () => void>;
      }).__MOEPLAY_GAMEPAD_MOCK__;
      api[methodName]();
    }, method);
  }
}
