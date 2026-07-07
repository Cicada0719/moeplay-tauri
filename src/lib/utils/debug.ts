/**
 * 开发期调试日志。
 *
 * 仅在本机 `npm run dev`（import.meta.env.DEV === true）时输出；
 * 生产构建（vite build）中 `import.meta.env.DEV` 被静态替换为 `false`，
 * 调用点会被 Vite/Rollup 摇树消除，不会进入最终安装包。
 *
 * 用途：保留番剧播放/换源管线的本地排障日志，同时避免生产环境控制台噪声。
 * 严禁在此写入任何敏感信息。
 */
export function debugLog(...args: unknown[]): void {
  if (import.meta.env.DEV) {
    // 开发期有意输出，非生产代码噪声
    console.log(...args);
  }
}
