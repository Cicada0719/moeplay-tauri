import { LogicalSize, type PhysicalSize, type Size } from "@tauri-apps/api/dpi";

// 窗口门面接口：只声明本模块用到的 Tauri Window 方法子集，
// 便于在 vitest 中注入假窗口对象做单元测试。
export interface WindowModeWindow {
  isFullscreen(): Promise<boolean>;
  setFullscreen(value: boolean): Promise<void>;
  isMaximized(): Promise<boolean>;
  maximize(): Promise<void>;
  unmaximize(): Promise<void>;
  setSize(size: LogicalSize | PhysicalSize | Size): Promise<void>;
  center(): Promise<void>;
}

export const WINDOWED_STARTUP_SIZE = { width: 1200, height: 800 } as const;

export interface StartupWindowModeOptions {
  maxAttempts?: number;
  retryDelayMs?: number;
  initialDelayMs?: number;
  sleep?: (ms: number) => Promise<void>;
}

const defaultSleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

// windowed / 历史值（如 dashboard）→ 目标非全屏；fullscreen / big-picture → 目标全屏。
function targetFullscreen(mode: string): boolean {
  return mode === "fullscreen" || mode === "big-picture";
}

/**
 * 按启动模式把窗口切到目标状态，带校验与重试。
 *
 * 为什么需要重试：窗口由 tauri.conf.json 的 fullscreen:true 原生全屏启动，
 * 而 Windows 上 WebView2 尚未完全初始化时 setFullscreen(false) 会静默失败
 * （不抛错但也不生效，见 src-tauri/src/lib.rs setup 注释），
 * 单次调用 + .catch(() => {}) 会让窗口永远停在全屏。
 * 因此每次尝试后回读 isFullscreen() 校验，未达标则等待后重试；
 * 达到最大次数仍失败则安静返回 false，绝不抛出——启动路径不允许被窗口操作拖死。
 */
export async function applyStartupWindowMode(
  mode: string,
  win: WindowModeWindow,
  opts: StartupWindowModeOptions = {},
): Promise<boolean> {
  const {
    maxAttempts = 4,
    retryDelayMs = 180,
    initialDelayMs = 120,
    sleep = defaultSleep,
  } = opts;
  const wantFullscreen = targetFullscreen(mode);

  // 先等 WebView2 就绪，再开始操作窗口。
  await sleep(initialDelayMs);

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      if (mode === "windowed") {
        // 还原成可调整大小的普通窗口：退出全屏 → 解除最大化 → 设定尺寸 → 居中。
        await win.setFullscreen(false);
        if (await win.isMaximized()) await win.unmaximize();
        await win.setSize(new LogicalSize(WINDOWED_STARTUP_SIZE.width, WINDOWED_STARTUP_SIZE.height));
        await win.center();
      } else if (wantFullscreen) {
        // 已是全屏就不要重复切换，避免 Windows 上无意义地重建窗口样式。
        if (!(await win.isFullscreen())) await win.setFullscreen(true);
      } else {
        // 历史值兜底（如 dashboard）：退出全屏后最大化。
        await win.setFullscreen(false);
        await win.maximize();
      }
      if ((await win.isFullscreen()) === wantFullscreen) return true;
    } catch {
      // 单步失败（IPC 错误等）同样进入重试，不向上抛。
    }
    if (attempt < maxAttempts) await sleep(retryDelayMs);
  }
  return false;
}
