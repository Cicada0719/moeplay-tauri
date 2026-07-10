import { gsap } from "gsap";
import { motionStore } from "../stores/motion.svelte";

export type MotionCleanup = () => void;
export type ScopedMotionSetup = () => void;

/**
 * Run GSAP work inside a component-scoped context and always expose `revert`
 * as the cleanup. When motion is reduced no tween is created and no inline
 * animation state is applied.
 */
export function createScopedMotion(scope: Element, setup: ScopedMotionSetup): MotionCleanup {
  const context = gsap.context(() => {
    if (!motionStore.reduced) {
      setup();
    }
  }, scope);

  return () => context.revert();
}

/**
 * Animate a panel entrance with a reduction-safe duration and return a cleanup
 * that kills the tween if its component disappears early.
 */
export function animatePanelEntrance(element: Element): MotionCleanup {
  if (motionStore.reduced) {
    gsap.set(element, { clearProps: "transform,opacity" });
    return () => undefined;
  }

  const tween = gsap.fromTo(
    element,
    { autoAlpha: 0, x: 12 },
    {
      autoAlpha: 1,
      x: 0,
      duration: 0.22,
      ease: "power2.out",
      clearProps: "transform,opacity,visibility",
    },
  );

  return () => tween.kill();
}

/**
 * Use this instead of hard-coded durations for small component transitions.
 */
export function motionDuration(seconds: number): number {
  return motionStore.reduced ? 0 : seconds;
}