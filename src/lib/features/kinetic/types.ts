/**
 * Kinetic 电影化媒体舞台 · 共享契约。
 *
 * 配色一律消费主题 token（--bg-deep/--bg-elev/--accent 等），
 * 不在组件内硬编码颜色；解析结果以 0..1 浮点三元组交给 WebGL 层。
 */

export type KineticQuality = "high" | "medium" | "low";

/** 舞台降级原因，落在 data-reason 上便于测试与巡检。 */
export type KineticFallbackReason =
  | "init"
  | "disabled"
  | "reduced-motion"
  | "no-webgl"
  | "context-lost"
  | "perf"
  | "error";

export interface KineticRgb {
  r: number;
  g: number;
  b: number;
}

export interface KineticPalette {
  /** --bg-deep：舞台底色 */
  bg: KineticRgb;
  /** --bg-elev：远景层色 */
  surface: KineticRgb;
  /** --accent：主光带色 */
  accent: KineticRgb;
  /** 由 accent 向白色偏移得到的高光色（仍为 token 派生） */
  glow: KineticRgb;
}

export interface KineticSceneOptions {
  quality: KineticQuality;
  palette: KineticPalette;
  onContextLost?: () => void;
  onContextRestored?: () => void;
  /** 每帧回调（毫秒增量），供帧率治理器采样。 */
  onFrame?: (deltaMs: number) => void;
}

export interface KineticSceneContract {
  mount(canvas: HTMLCanvasElement): Promise<void>;
  setPalette(palette: KineticPalette): void;
  setQuality(quality: KineticQuality): void;
  resize(width: number, height: number, dpr: number): void;
  pause(): void;
  resume(): void;
  dispose(): void;
}
