/**
 * reduced-motion 双信号守卫（概念站 reducedMotion.ts 的生产硬化版）：
 *   1. `prefers-reduced-motion: reduce` media query
 *   2. `document.documentElement.dataset.motion === "reduce"`（应用内 motionStore 写入）
 * 任一成立即视为减弱动态。无 window/document 环境下安全返回 false。
 */

export type KineticReducedMotionListener = (reduced: boolean) => void;

export interface KineticReducedMotionGuard {
  readonly active: boolean;
  dispose(): void;
}

export function mediaPrefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function documentPrefersReducedMotion(): boolean {
  return typeof document !== "undefined" && document.documentElement?.dataset.motion === "reduce";
}

export function isReducedMotionActive(): boolean {
  return mediaPrefersReducedMotion() || documentPrefersReducedMotion();
}

/**
 * 订阅双信号变化：media query 的 change 事件 + documentElement 上
 * data-motion 属性的 MutationObserver。回调立即以当前状态触发一次。
 */
export function createReducedMotionGuard(listener: KineticReducedMotionListener): KineticReducedMotionGuard {
  let active = isReducedMotionActive();
  let disposed = false;

  const query = typeof matchMedia === "function" ? matchMedia("(prefers-reduced-motion: reduce)") : null;
  const sync = () => {
    const next = isReducedMotionActive();
    if (next === active) return;
    active = next;
    if (!disposed) listener(active);
  };

  query?.addEventListener("change", sync);

  let observer: MutationObserver | null = null;
  if (typeof MutationObserver === "function" && typeof document !== "undefined") {
    observer = new MutationObserver(sync);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-motion"] });
  }

  listener(active);

  return {
    get active() {
      return active;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      query?.removeEventListener("change", sync);
      observer?.disconnect();
      observer = null;
    },
  };
}
