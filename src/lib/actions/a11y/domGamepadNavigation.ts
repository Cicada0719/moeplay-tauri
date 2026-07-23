export type SpatialDirection = "up" | "down" | "left" | "right";

export const DEFAULT_GAMEPAD_FOCUS_SELECTOR = [
  "button:not([disabled])",
  "a[href]",
  "input:not([disabled]):not([type='hidden'])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[role='button']:not([aria-disabled='true'])",
  "[role='tab']:not([aria-disabled='true'])",
  "[role='option']:not([aria-disabled='true'])",
  "[role='menuitem']:not([aria-disabled='true'])",
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

function isEligibleTabStop(element: HTMLElement): boolean {
  const rawTabIndex = element.getAttribute("tabindex");
  if (rawTabIndex == null || Number(rawTabIndex) >= 0) return true;
  if (element.dataset.gamepadForce === "true") return true;
  return ["tab", "option", "menuitem"].includes(element.getAttribute("role") ?? "");
}

function isVisible(element: HTMLElement): boolean {
  if (element.hidden || element.closest("[hidden], [inert], [aria-hidden='true'], [data-gamepad-ignore='true']")) return false;
  if (element.matches("[data-gamepad-skip='true']")) return false;
  if (element.getAttribute("aria-disabled") === "true") return false;
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
    .filter((element) => !element.matches(":disabled") && isEligibleTabStop(element) && isVisible(element));
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

  const tabList = active.getAttribute("role") === "tab" ? active.closest<HTMLElement>("[role='tablist']") : null;
  const tabOrientation = tabList?.getAttribute("aria-orientation") ?? "horizontal";
  const movesWithinTabList = tabList && (
    (tabOrientation === "vertical" && (direction === "up" || direction === "down"))
    || (tabOrientation !== "vertical" && (direction === "left" || direction === "right"))
  );
  if (tabList && movesWithinTabList) {
    const tabs = collectGamepadFocusable({ root: tabList, selector: "[role='tab']:not([aria-disabled='true'])" });
    const index = tabs.indexOf(active);
    if (index >= 0 && tabs.length > 1) {
      const delta = direction === "left" || direction === "up" ? -1 : 1;
      const target = tabs[(index + delta + tabs.length) % tabs.length];
      target.focus({ preventScroll: true });
      target.scrollIntoView({ block: "nearest", inline: "nearest" });
      return target;
    }
  }

  const overrideSelector = active.getAttribute(`data-gamepad-nav-${direction}`)?.trim();
  if (overrideSelector) {
    let override: HTMLElement | null = null;
    try {
      const candidate = (options.root ?? document).querySelector(overrideSelector);
      override = isHTMLElement(candidate) ? candidate : null;
    } catch {
      // Ignore invalid author-provided selectors and keep geometric navigation available.
    }
    if (override && collectGamepadFocusable({ ...options, root: override.parentNode ?? options.root }).includes(override)) {
      override.focus({ preventScroll: true });
      override.scrollIntoView({ block: "nearest", inline: "nearest" });
      return override;
    }
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

  if (!winner) return active;
  winner.focus({ preventScroll: true });
  winner.scrollIntoView({ block: "nearest", inline: "nearest" });
  return winner;
}

function activateElement(element: HTMLElement): void {
  if (element instanceof HTMLSelectElement) {
    try {
      const showPicker = (element as HTMLSelectElement & { showPicker?: () => void }).showPicker;
      if (typeof showPicker === "function") {
        showPicker.call(element);
        return;
      }
    } catch {
      // WebView/browser denied showPicker; click is the portable fallback.
    }
  }
  element.click();
}

export function activateGamepadFocus(options: SpatialNavigationOptions = {}): HTMLElement | null {
  const active = isHTMLElement(options.activeElement)
    ? options.activeElement
    : typeof document !== "undefined" && isHTMLElement(document.activeElement)
      ? document.activeElement
      : null;
  if (active && collectGamepadFocusable(options).includes(active)) {
    activateElement(active);
    return active;
  }
  const first = collectGamepadFocusable(options)[0] ?? null;
  if (!first) return null;
  first.focus({ preventScroll: true });
  first.scrollIntoView({ block: "nearest", inline: "nearest" });
  return first;
}

export function activateGamepadSecondaryFocus(options: SpatialNavigationOptions = {}): HTMLElement | null {
  const active = isHTMLElement(options.activeElement)
    ? options.activeElement
    : typeof document !== "undefined" && isHTMLElement(document.activeElement)
      ? document.activeElement
      : null;
  if (!active) return activateGamepadFocus(options);
  const group = active.closest<HTMLElement>("[data-gamepad-group]");
  const secondary = group?.querySelector<HTMLElement>("[data-gamepad-secondary-action]:not([disabled])") ?? null;
  if (secondary && isVisible(secondary)) {
    activateElement(secondary);
    return secondary;
  }
  return activateGamepadFocus(options);
}

export function focusGamepadSearch(root: ParentNode = document): HTMLElement | null {
  const candidates = Array.from(root.querySelectorAll<HTMLElement>(
    "input[type='search']:not([disabled]), [data-search-scope] input:not([disabled]), input[placeholder*='搜索']:not([disabled])",
  ));
  const candidate = candidates.find((element) => isEligibleTabStop(element) && isVisible(element)) ?? null;
  if (!candidate) return null;
  candidate.focus({ preventScroll: true });
  candidate.scrollIntoView({ block: "center", inline: "nearest" });
  return candidate;
}
