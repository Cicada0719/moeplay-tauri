import type { MotionQuality } from "../contracts";

export interface MotionCapabilitySnapshot {
  reducedMotion: boolean;
  hardwareConcurrency: number;
  deviceMemory?: number;
  devicePixelRatio: number;
  saveData: boolean;
  webgl2: boolean;
}

export interface MotionQualityDetectionOptions {
  preferred?: MotionQuality;
  snapshot?: Partial<MotionCapabilitySnapshot>;
  respectReducedMotion?: boolean;
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

export function readMotionCapabilities(): MotionCapabilitySnapshot {
  const navigatorLike = typeof navigator === "undefined" ? undefined : navigator;
  const connection = navigatorLike
    ? (navigatorLike as Navigator & { connection?: { saveData?: boolean } }).connection
    : undefined;
  const deviceMemory = navigatorLike
    ? (navigatorLike as Navigator & { deviceMemory?: number }).deviceMemory
    : undefined;

  return {
    reducedMotion: typeof matchMedia === "function"
      ? matchMedia("(prefers-reduced-motion: reduce)").matches
      : false,
    hardwareConcurrency: navigatorLike?.hardwareConcurrency ?? 4,
    deviceMemory,
    devicePixelRatio: typeof window === "undefined" ? 1 : window.devicePixelRatio || 1,
    saveData: connection?.saveData === true,
    webgl2: hasWebGL2(),
  };
}

export function detectMotionQuality(options: MotionQualityDetectionOptions = {}): MotionQuality {
  const capabilities = { ...readMotionCapabilities(), ...options.snapshot };
  const respectReducedMotion = options.respectReducedMotion !== false;

  if ((respectReducedMotion && capabilities.reducedMotion) || options.preferred === "reduced") {
    return "reduced";
  }

  if (capabilities.saveData || !capabilities.webgl2 || capabilities.hardwareConcurrency <= 2 ||
      (capabilities.deviceMemory !== undefined && capabilities.deviceMemory <= 2)) {
    return "reduced";
  }

  const constrained = capabilities.hardwareConcurrency <= 4 ||
    (capabilities.deviceMemory !== undefined && capabilities.deviceMemory <= 4) ||
    capabilities.devicePixelRatio >= 2.5;

  if (options.preferred === "balanced" || constrained) return "balanced";
  return options.preferred ?? "full";
}

export function recommendedRenderDpr(quality: MotionQuality, requested: number): number {
  const safeDpr = Number.isFinite(requested) ? Math.max(1, requested) : 1;
  if (quality === "reduced") return 1;
  return Math.min(safeDpr, quality === "balanced" ? 1.5 : 2);
}
