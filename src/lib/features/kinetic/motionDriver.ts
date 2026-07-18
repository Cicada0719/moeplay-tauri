/**
 * Kinetic 舞台运动驱动器（概念站 MotionDriver 的生产硬化版）：
 * 单一 rAF 循环的所有者，负责
 *   - `document.hidden` / 窗口失焦时自动暂停渲染循环
 *   - 恢复可见/聚焦时自动续跑
 *   - dispose 时完整摘除监听器并取消帧回调
 * 不依赖 gsap：舞台为连续环境动效，匀速采样即可，避免无谓 tween 分配。
 */

export type KineticTick = (timeMs: number, deltaMs: number) => void;

export class KineticMotionDriver {
  private tick: KineticTick | null = null;
  private frame = 0;
  private lastTime = 0;
  private running = false;
  private disposed = false;
  private readonly owned = typeof window !== "undefined" && typeof document !== "undefined";

  private readonly handleVisibility = (): void => {
    if (typeof document === "undefined") return;
    if (document.hidden) this.pause();
    else this.resume();
  };

  private readonly handleBlur = (): void => this.pause();
  private readonly handleFocus = (): void => this.resume();

  get paused(): boolean {
    return !this.running;
  }

  start(tick: KineticTick): void {
    if (this.disposed) return;
    this.tick = tick;
    if (this.running) return;
    this.running = true;
    if (this.owned) {
      document.addEventListener("visibilitychange", this.handleVisibility);
      window.addEventListener("blur", this.handleBlur);
      window.addEventListener("focus", this.handleFocus);
    }
    this.lastTime = this.now();
    this.schedule();
  }

  pause(): void {
    if (!this.running) return;
    this.running = false;
    if (this.frame) cancelAnimationFrame(this.frame);
    this.frame = 0;
  }

  resume(): void {
    if (this.disposed || this.running || !this.tick) return;
    if (typeof document !== "undefined" && document.hidden) return;
    this.running = true;
    this.lastTime = this.now();
    this.schedule();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.pause();
    this.tick = null;
    if (this.owned) {
      document.removeEventListener("visibilitychange", this.handleVisibility);
      window.removeEventListener("blur", this.handleBlur);
      window.removeEventListener("focus", this.handleFocus);
    }
  }

  private schedule(): void {
    if (!this.running || typeof requestAnimationFrame !== "function") return;
    this.frame = requestAnimationFrame(this.step);
  }

  private readonly step = (time: number): void => {
    this.frame = 0;
    if (!this.running || !this.tick) return;
    const delta = Math.min(64, Math.max(0, time - this.lastTime));
    this.lastTime = time;
    try {
      this.tick(time, delta);
    } catch {
      // 单帧异常不得打断循环，也不得冒泡到全局错误边界。
    }
    this.schedule();
  };

  private now(): number {
    return typeof performance === "undefined" ? Date.now() : performance.now();
  }
}

export function createKineticMotionDriver(): KineticMotionDriver {
  return new KineticMotionDriver();
}
