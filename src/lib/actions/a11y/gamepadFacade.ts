// 懒加载门面：App 等主包入口通过 loadGamepadApi() 按需加载手柄运行时。
let apiPromise: Promise<typeof import("./gamepadApi")> | null = null;

export function loadGamepadApi(): Promise<typeof import("./gamepadApi")> {
  apiPromise ??= import("./gamepadApi");
  return apiPromise;
}
