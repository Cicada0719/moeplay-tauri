import { gsap } from "gsap";
import type { ContentMode, MotionDriverContract, MotionQuality } from "../contracts";
import { createEnterAnimation, transitionContentMode } from "./gsapTransitions";

export interface MotionDriverOptions {
  quality: MotionQuality;
  reducedMotion?: boolean;
}

export class MotionDriver implements MotionDriverContract {
  quality: MotionQuality;
  reducedMotion: boolean;
  private readonly enterCleanups = new WeakMap<HTMLElement, () => void>();

  constructor(options: MotionDriverOptions) {
    this.quality = options.quality;
    this.reducedMotion = options.reducedMotion ?? false;
  }

  setPreferences(quality: MotionQuality, reducedMotion = this.reducedMotion): void {
    this.quality = quality;
    this.reducedMotion = reducedMotion;
  }

  enter(root: HTMLElement): () => void {
    this.enterCleanups.get(root)?.();
    const cleanup = createEnterAnimation(root, this);
    let active = true;
    const guardedCleanup = () => {
      if (!active) return;
      active = false;
      cleanup();
      this.enterCleanups.delete(root);
    };
    this.enterCleanups.set(root, guardedCleanup);
    return guardedCleanup;
  }

  transition(root: HTMLElement, from: ContentMode, to: ContentMode): Promise<void> {
    return transitionContentMode(root, from, to, this);
  }

  dispose(root?: HTMLElement): void {
    if (root) {
      this.enterCleanups.get(root)?.();
      gsap.killTweensOf(root.querySelectorAll("[data-motion-enter], [data-motion-mode]"));
    }
  }
}

export function createMotionDriver(options: MotionDriverOptions): MotionDriver {
  return new MotionDriver(options);
}
