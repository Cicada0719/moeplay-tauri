import {
  getDefaultGamepadFocusRuntime,
  type GamepadFocusRuntime,
  type GamepadScopeController,
  type GamepadScopeHandlers,
  type GamepadScopeOptions,
  type GamepadZone,
} from "../../actions/a11y/gamepadFocus";

// 兼容旧调用的手柄入口，同时委托给共享的 scope/zone runtime。
//
// 按键映射（Xbox / PS 通用下标）：
//   0 = A / ✕  → launch（启动）
//   1 = B / ○  → back
//   2 = X / □  → favorite
//   3 = Y / △  → activate（详情）
//   4/5 = LB/RB → pageLeft/pageRight
//   12-15 = 十字键 上/下/左/右
// 左摇杆双轴与十字键共享 320ms 初始延迟、100ms 连发。

export type GamepadHandlers = GamepadScopeHandlers;

export type AttachGamepadOptions = GamepadScopeOptions & {
  runtime?: GamepadFocusRuntime | null;
};

export type GamepadAttachment = (() => void) & {
  readonly id: string | null;
  pause(): void;
  resume(): void;
  activate(): void;
  setZone(zone: GamepadZone | null): void;
  setPriority(priority: number): void;
  setOverlay(overlay: boolean): void;
  setEnabled(enabled: boolean): void;
  updateHandlers(handlers: GamepadHandlers): void;
};

function inertAttachment(): GamepadAttachment {
  const detach = (() => {}) as GamepadAttachment;
  Object.defineProperty(detach, "id", { value: null });
  detach.pause = () => {};
  detach.resume = () => {};
  detach.activate = () => {};
  detach.setZone = () => {};
  detach.setPriority = () => {};
  detach.setOverlay = () => {};
  detach.setEnabled = () => {};
  detach.updateHandlers = () => {};
  return detach;
}

function adaptHandlers(handlers: GamepadHandlers): GamepadScopeHandlers {
  return {
    ...handlers,
    // Existing call sites used pageLeft/pageRight as their vertical movement
    // hooks. Keep that behavior until the visual components adopt explicit
    // up/down handlers in P0-05.
    up: handlers.up ?? handlers.pageLeft,
    down: handlers.down ?? handlers.pageRight,
  };
}

function attachmentFor(controller: GamepadScopeController): GamepadAttachment {
  let detached = false;
  const detach = (() => {
    if (detached) return;
    detached = true;
    controller.destroy();
  }) as GamepadAttachment;

  Object.defineProperty(detach, "id", { get: () => controller.id });
  detach.pause = () => controller.pause();
  detach.resume = () => controller.resume();
  detach.activate = () => controller.activate();
  detach.setZone = (zone) => controller.setZone(zone);
  detach.setPriority = (priority) => controller.setPriority(priority);
  detach.setOverlay = (overlay) => controller.setOverlay(overlay);
  detach.setEnabled = (enabled) => controller.setEnabled(enabled);
  detach.updateHandlers = (handlers) => controller.updateHandlers(adaptHandlers(handlers));
  return detach;
}

/**
 * Register a gamepad handler scope. Existing `attachGamepad(handlers)` callers
 * remain valid and receive a callable detach function; newer callers may use
 * the controller methods attached to that function.
 */
export function attachGamepad(
  handlers: GamepadHandlers,
  options: AttachGamepadOptions = {},
): GamepadAttachment {
  const { runtime = getDefaultGamepadFocusRuntime(), ...scopeOptions } = options;
  if (!runtime) return inertAttachment();
  return attachmentFor(runtime.registerScope(adaptHandlers(handlers), scopeOptions));
}
