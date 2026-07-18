export type SpatialDirection = "up" | "down" | "left" | "right";

export const DEFAULT_GAMEPAD_FOCUS_SELECTOR = [
  "button:not([disabled])",
  "a[href]",
  "input:not([disabled]):not([type='hidden'])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
  "[contenteditable='true']",
].join(",");

export interface SpatialNavigationOptions {
  root?: ParentNode;
  selector?: string;
  activeElement?: Element | null;
}

function isHTMLElement(value: unknown): value is HTMLElement {
  return typeof HTMLElement !== "undefined" && value instanceof HTMLElement;
}

function isVisible(element: HTMLElement): boolean {
  if (element.hidden || element.closest("[hidden], [inert], [aria-hidden='true']")) return false;
  const style = typeof getComputedStyle === "function" ? getComputedStyle(element) : null;
  if (style && (style.display === "none" || style.visibility === "hidden" || style.opacity !== "" && Number(style.opacity) === 0)) return false;
  const rect = element.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

export function collectGamepadFocusable(options: SpatialNavigationOptions = {}): HTMLElement[] {
  const root = options.root ?? (typeof document !== "undefined" ? document : undefined);
  if (!root || !("querySelectorAll" in root)) return [];
  const selector = options.selector ?? DEFAULT_GAMEPAD_FOCUS_SELECTOR;
  return Array.from(root.querySelectorAll(selector))
    .filter(isHTMLElement)
    .filter((element) => !element.matches(":disabled") && isVisible(element));
}

function centerOf(rect: DOMRect): { x: number; y: number } {
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

function directionalScore(origin: DOMRect, candidate: DOMRect, direction: SpatialDirection): number | null {
  const a = centerOf(origin);
  const b = centerOf(candidate);
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const primary = direction === "left" ? -dx : direction === "right" ? dx : direction === "up" ? -dy : dy;
  if (primary <= 1) return null;
  const secondary = direction === "left" || direction === "right" ? Math.abs(dy) : Math.abs(dx);
  const overlap = direction === "left" || direction === "right"
    ? Math.max(0, Math.min(origin.bottom, candidate.bottom) - Math.max(origin.top, candidate.top))
    : Math.max(0, Math.min(origin.right, candidate.right) - Math.max(origin.left, candidate.left));
  // Prefer the nearest item in the requested direction, strongly favoring rows/columns
  // that overlap with the current item. This works across cards, tabs and dense forms.
  const alignmentPenalty = secondary * (overlap > 0 ? 1.4 : 3.5);
  return primary * 10 + alignmentPenalty + Math.hypot(dx, dy) * 0.15 - overlap * 0.5;
}

function fallbackIndex(current: number, length: number, direction: SpatialDirection): number {
  const delta = direction === "left" || direction === "up" ? -1 : 1;
  return (current + delta + length) % length;
}

export function moveGamepadFocus(direction: SpatialDirection, options: SpatialNavigationOptions = {}): HTMLElement | null {
  const focusable = collectGamepadFocusable(options);
  if (focusable.length === 0) return null;

  const active = isHTMLElement(options.activeElement)
    ? options.activeElement
    : typeof document !== "undefined" && isHTMLElement(document.activeElement)
      ? document.activeElement
      : null;
  const currentIndex = active ? focusable.indexOf(active) : -1;
  if (currentIndex < 0 || !active) {
    const first = focusable[0];
    first.focus({ preventScroll: true });
    first.scrollIntoView({ block: "nearest", inline: "nearest" });
    return first;
  }

  const origin = active.getBoundingClientRect();
  let winner: HTMLElement | null = null;
  let winnerScore = Number.POSITIVE_INFINITY;
  for (const candidate of focusable) {
    if (candidate === active) continue;
    const score = directionalScore(origin, candidate.getBoundingClientRect(), direction);
    if (score != null && score < winnerScore) {
      winner = candidate;
      winnerScore = score;
    }
  }

  winner ??= focusable[fallbackIndex(currentIndex, focusable.length, direction)];
  winner.focus({ preventScroll: true });
  winner.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  return winner;
}

export function activateGamepadFocus(options: SpatialNavigationOptions = {}): HTMLElement | null {
  const active = isHTMLElement(options.activeElement)
    ? options.activeElement
    : typeof document !== "undefined" && isHTMLElement(document.activeElement)
      ? document.activeElement
      : null;
  if (active && collectGamepadFocusable(options).includes(active)) {
    active.click();
    return active;
  }
  const first = collectGamepadFocusable(options)[0] ?? null;
  if (!first) return null;
  first.focus({ preventScroll: true });
  first.scrollIntoView({ block: "nearest", inline: "nearest" });
  return first;
}

export function focusGamepadSearch(root: ParentNode = document): HTMLElement | null {
  const candidate = root.querySelector<HTMLElement>(
    "input[type='search']:not([disabled]), [data-search-scope] input:not([disabled]), input[placeholder*='搜索']:not([disabled])",
  );
  if (!candidate || !isVisible(candidate)) return null;
  candidate.focus({ preventScroll: true });
  candidate.scrollIntoView({ block: "center", inline: "nearest" });
  return candidate;
}
