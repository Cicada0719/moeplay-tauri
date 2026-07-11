import { gsap } from "gsap";
import type { ContentMode, MotionQuality } from "../contracts";

const ENTER_SELECTOR = "[data-motion-enter]";
const MODE_SELECTOR = "[data-motion-mode]";

export interface MotionTransitionOptions {
  quality: MotionQuality;
  reducedMotion: boolean;
}

function durationFor(quality: MotionQuality, full: number): number {
  if (quality === "reduced") return 0;
  return quality === "balanced" ? full * 0.72 : full;
}

export function createEnterAnimation(
  root: HTMLElement,
  options: MotionTransitionOptions,
): () => void {
  const context = gsap.context(() => {
    const targets = gsap.utils.toArray<HTMLElement>(ENTER_SELECTOR, root);
    if (options.reducedMotion || options.quality === "reduced") {
      gsap.set(targets, { clearProps: "transform,opacity,visibility" });
      return;
    }

    gsap.fromTo(targets,
      { autoAlpha: 0, y: options.quality === "full" ? 28 : 16 },
      {
        autoAlpha: 1,
        y: 0,
        duration: durationFor(options.quality, 0.72),
        stagger: options.quality === "full" ? 0.055 : 0.035,
        ease: "power3.out",
        overwrite: "auto",
        clearProps: "transform,opacity,visibility",
      },
    );
  }, root);

  return () => context.revert();
}

export function transitionContentMode(
  root: HTMLElement,
  from: ContentMode,
  to: ContentMode,
  options: MotionTransitionOptions,
): Promise<void> {
  const targets = gsap.utils.toArray<HTMLElement>(MODE_SELECTOR, root);
  root.dataset.modeFrom = from;
  root.dataset.mode = to;

  if (options.reducedMotion || options.quality === "reduced" || targets.length === 0) {
    gsap.set(targets, { clearProps: "transform,opacity,visibility" });
    delete root.dataset.modeFrom;
    return Promise.resolve();
  }

  gsap.killTweensOf(targets);
  const distance = options.quality === "full" ? 22 : 12;
  const direction = modeOrder(to) >= modeOrder(from) ? 1 : -1;

  return new Promise((resolve) => {
    const timeline = gsap.timeline({
      defaults: { overwrite: "auto" },
      onComplete: () => {
        gsap.set(targets, { clearProps: "transform,opacity,visibility" });
        delete root.dataset.modeFrom;
        resolve();
      },
      onInterrupt: () => {
        delete root.dataset.modeFrom;
        resolve();
      },
    });
    timeline.fromTo(targets,
      { autoAlpha: 0, x: distance * direction },
      {
        autoAlpha: 1,
        x: 0,
        duration: durationFor(options.quality, 0.52),
        stagger: options.quality === "full" ? 0.025 : 0.015,
        ease: "power3.out",
      },
    );
  });
}

function modeOrder(mode: ContentMode): number {
  return mode === "visual" ? 0 : mode === "index" ? 1 : 2;
}
