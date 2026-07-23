import type { SpatialDirection } from "./domGamepadNavigation";

/**
 * Controller surface bridge.
 *
 * Home-stage components (GameVisualV2 / GameSceneV2) already expose a complete
 * keyboard contract on their root `<section data-controller-surface tabindex="0">`:
 * ArrowUp/ArrowDown switch the selected game, ArrowLeft/ArrowRight step media,
 * Enter opens the archive. The gamepad runtime only moves DOM focus, so without
 * this bridge the stick could never switch games directly. These helpers focus
 * the (stable, never re-keyed) surface root and forward synthetic key events so
 * gamepad and keyboard share one code path.
 */

const SURFACE_SELECTOR = "[data-controller-surface]";

const DIRECTION_KEYS: Record<SpatialDirection, string> = {
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
};

function isTypingTarget(element: Element): boolean {
  return Boolean(element.closest("input, textarea, select, [contenteditable='true']"));
}

function isSurfaceVisible(element: HTMLElement): boolean {
  if (element.hidden || element.closest("[hidden], [inert], [aria-hidden='true']")) return false;
  const style = typeof getComputedStyle === "function" ? getComputedStyle(element) : null;
  if (style && (style.display === "none" || style.visibility === "hidden")) return false;
  const rect = element.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

/** The visible controller surface containing `element`, if any. */
export function controllerSurfaceFor(element: Element | null): HTMLElement | null {
  if (!element || isTypingTarget(element)) return null;
  const surface = element.closest<HTMLElement>(SURFACE_SELECTOR);
  return surface && isSurfaceVisible(surface) ? surface : null;
}

/** The first visible controller surface under `root` (used for initial entry). */
export function findControllerSurface(root: ParentNode): HTMLElement | null {
  if (!("querySelectorAll" in root)) return null;
  return Array.from(root.querySelectorAll<HTMLElement>(SURFACE_SELECTOR)).find(isSurfaceVisible) ?? null;
}

/**
 * Focus the surface root and dispatch a synthetic keydown so the component's
 * own keyboard handler performs the action. Focus is deliberately pinned to the
 * surface section: per-game buttons inside are re-keyed on every selection
 * change, so holding focus there would be destroyed on the next game switch.
 */
export function dispatchSurfaceKey(surface: HTMLElement, key: string): boolean {
  surface.focus({ preventScroll: true });
  return surface.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

export function dispatchSurfaceDirection(surface: HTMLElement, direction: SpatialDirection): boolean {
  return dispatchSurfaceKey(surface, DIRECTION_KEYS[direction]);
}
