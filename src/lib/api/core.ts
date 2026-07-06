// 萌游 MoeGame · Tauri invoke 统一封装与测试 mock 注入点
//
// 所有前端调用 Rust 命令都应通过 invokeCmd，以便在单元测试/组件测试中注入 mock。

import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export type InvokeMockHandler = (
  command: string,
  args?: Record<string, unknown>
) => unknown;

let mockHandler: InvokeMockHandler | undefined;

/** 注册一个全局 mock 处理器；测试环境中所有 invokeCmd 调用都会先经过它。 */
export function setMockInvokeHandler(handler: InvokeMockHandler): void {
  mockHandler = handler;
}

/** 清除 mock 处理器。 */
export function clearMockInvokeHandler(): void {
  mockHandler = undefined;
}

/** 判断当前是否处于 mock 模式。 */
export function isMockEnabled(): boolean {
  return typeof mockHandler !== "undefined";
}

/**
 * 调用 Tauri 命令。
 *
 * 若已注册 mock handler（通常在测试 setup 中），则直接交给 handler，
 * 否则透传给 `@tauri-apps/api/core` 的 `invoke`。
 */
export async function invokeCmd<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (mockHandler) {
    const result = mockHandler(command, args);
    // 支持 handler 返回同步值或 Promise
    return (await result) as T;
  }
  return tauriInvoke<T>(command, args);
}

/**
 * 构造一个针对指定命令的 mock handler。
 *
 * 示例：
 * ```ts
 * setMockInvokeHandler(mockRouter({
 *   get_games: () => [],
 *   get_settings: () => ({ theme: "dark" }),
 * }));
 * ```
 */
export function mockRouter(
  handlers: Record<string, InvokeMockHandler>
): InvokeMockHandler {
  return (command, args) => {
    const handler = handlers[command];
    if (!handler) {
      throw new Error(`[mock] 未注册命令: ${command}`);
    }
    return handler(command, args ?? {});
  };
}
