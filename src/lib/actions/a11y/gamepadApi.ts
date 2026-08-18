// a11y 手柄运行时聚合导出：主包按需加载本模块（动态 import），
// 无手柄/未触发手柄交互时不下加载整个手柄导航与轮询运行时。
export { attachGamepad, type GamepadAttachment, type GamepadHandlers } from "../../components/switch/useGamepad.svelte";
export { getDefaultGamepadFocusRuntime, type GamepadInputMode, type GamepadScopeHandlers } from "./gamepadFocus";
export {
  activateGamepadFocus,
  activateGamepadSecondaryFocus,
  collectGamepadFocusable,
  focusGamepadSearch,
  moveGamepadFocus,
} from "./domGamepadNavigation";
export {
  controllerSurfaceFor,
  dispatchSurfaceDirection,
  dispatchSurfaceKey,
  findControllerSurface,
} from "./controllerSurface";
export { adjustFocusedGamepadControl } from "./gamepadSemantics";
