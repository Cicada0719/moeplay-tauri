/**
 * Kinetic 舞台质量分级（概念站 quality.ts 的生产版）：
 * high/medium/low 三档，按能力快照初判，再由帧率治理器按实测帧率自动降档。
 * 粒子仅在 high 档启用；low 档为最小着色负载，dpr 锁 1。
 */

import type { KineticQuality } from "./types";

export interface KineticCapabilitySnapshot {
  webgl2: boolean;
  hardwareConcurrency: number;
  deviceMemory?: number;
  devicePixelRatio: number;
  saveData: boolean;
}

function hasWebGL2(): boolean {
  if (typeof document === "undefined") return false;
  try {
    return Boolean(document.createElement("canvas").getContext("webgl2", {
      failIfMajorPerformanceCaveat: true,
    }));
  } catch {
    return false;
  }
}

export function readKineticCapabilities(): KineticCapabilitySnapshot {
  const navigatorLike = typeof navigator === "undefined" ? undefined : navigator;
  const connection = navigatorLike
    ? (navigatorLike as Navigator & { connection?: { saveData?: boolean } }).connection
    : undefined;
  const deviceMemory = navigatorLike
    ? (navigatorLike as Navigator & { deviceMemory?: number }).deviceMemory
    : undefined;

  return {
    webgl2: hasWebGL2(),
    hardwareConcurrency: navigatorLike?.hardwareConcurrency ?? 4,
    deviceMemory,
    devicePixelRatio: typeof window === "undefined" ? 1 : window.devicePixelRatio || 1,
    saveData: connection?.saveData === true,
  };
}

/**
 * 初判质量档。reduced-motion 不在此处处理（由调用方双信号守卫负责）；
 * 无 WebGL2 时返回 low，调用方通常据此直接走 fallback。
 */
export function detectKineticQuality(snapshot?: Partial<KineticCapabilitySnapshot>): KineticQuality {
  const capabilities = { ...readKineticCapabilities(), ...snapshot };

  if (
    capabilities.saveData || !capabilities.webgl2 || capabilities.hardwareConcurrency <= 2 ||
    (capabilities.deviceMemory !== undefined && capabilities.deviceMemory <= 2)
  ) {
    return "low";
  }

  const constrained = capabilities.hardwareConcurrency <= 4 ||
    (capabilities.deviceMemory !== undefined && capabilities.deviceMemory <= 4) ||
    capabilities.devicePixelRatio >= 2.5;

  return constrained ? "medium" : "high";
}

export function recommendedRenderDpr(quality: KineticQuality, requested: number): number {
  const safeDpr = Number.isFinite(requested) ? Math.max(1, requested) : 1;
  if (quality === "low") return 1;
  return Math.min(safeDpr, quality === "medium" ? 1.5 : 2);
}

const QUALITY_ORDER: KineticQuality[] = ["low", "medium", "high"];

export function lowerKineticQuality(quality: KineticQuality): KineticQuality | null {
  const index = QUALITY_ORDER.indexOf(quality);
  return index > 0 ? QUALITY_ORDER[index - 1] : null;
}

export interface KineticQualityGovernorOptions {
  /** 判定窗口内平均帧率低于该值触发降档 */
  degradeBelowFps?: number;
  /** low 档仍低于该值时建议回退到 CSS fallback */
  fallbackBelowFps?: number;
  /** 两次降档之间的最小驻留时间（毫秒），防止抖动 */
  dwellMs?: number;
  /** 采样窗口帧数 */
  windowSize?: number;
  now?: () => number;
}

export type KineticGovernorVerdict = KineticQuality | "fallback" | null;

/**
 * 帧率治理器：滚动窗口统计平均帧率，持续低于阈值时逐级降档；
 * 已在 low 档且帧率仍极差时给出 "fallback" 裁决，由调用方切到 CSS 版。
 */
export class KineticQualityGovernor {
  private quality: KineticQuality;
  private readonly degradeBelowFps: number;
  private readonly fallbackBelowFps: number;
  private readonly dwellMs: number;
  private readonly windowSize: number;
  private readonly now: () => number;
  private samples: number[] = [];
  private lastVerdictAt = 0;

  constructor(initial: KineticQuality, options: KineticQualityGovernorOptions = {}) {
    this.quality = initial;
    this.degradeBelowFps = options.degradeBelowFps ?? 45;
    this.fallbackBelowFps = options.fallbackBelowFps ?? 24;
    this.dwellMs = options.dwellMs ?? 2000;
    this.windowSize = options.windowSize ?? 90;
    this.now = options.now ?? (() => (typeof performance === "undefined" ? Date.now() : performance.now()));
  }

  get current(): KineticQuality {
    return this.quality;
  }

  /** 喂入一帧的毫秒增量；返回需要执行的降档裁决（无动作时为 null）。 */
  sample(deltaMs: number): KineticGovernorVerdict {
    // 超过 100ms 的帧大多是后台切换/GC 尖峰，不计入窗口。
    if (!Number.isFinite(deltaMs) || deltaMs <= 0 || deltaMs > 100) {
      return null;
    }

    this.samples.push(deltaMs);
    if (this.samples.length > this.windowSize) this.samples.shift();
    if (this.samples.length < Math.min(60, this.windowSize)) return null;

    const average = this.samples.reduce((sum, value) => sum + value, 0) / this.samples.length;
    const fps = 1000 / average;
    if (fps >= this.degradeBelowFps) return null;

    const at = this.now();
    if (at - this.lastVerdictAt < this.dwellMs) return null;

    const lowered = lowerKineticQuality(this.quality);
    if (lowered) {
      this.quality = lowered;
      this.lastVerdictAt = at;
      this.samples = [];
      return lowered;
    }

    if (fps < this.fallbackBelowFps) {
      this.lastVerdictAt = at;
      this.samples = [];
      return "fallback";
    }
    return null;
  }
}
