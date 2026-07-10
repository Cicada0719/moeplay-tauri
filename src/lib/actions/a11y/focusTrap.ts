export type FocusTarget = string | HTMLElement | null | undefined | (() => HTMLElement | null | undefined);

export type InitialFocusTarget = "auto" | FocusTarget | false;
export type ReturnFocusTarget = boolean | FocusTarget;

export interface FocusTrapOptions {
  enabled?: boolean;
  trapFocus?: boolean;
  closeOnEscape?: boolean;
  initialFocus?: InitialFocusTarget;
  returnFocus?: ReturnFocusTarget;
  onEscape?: (event: KeyboardEvent) => void;
}

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "area[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type='hidden'])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "iframe",
  "audio[controls]",
  "video[controls]",
  "[contenteditable='true']",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

type FocusTrapEntry = {
  node: HTMLElement;
  options: Required<Pick<FocusTrapOptions, "enabled" | "trapFocus" | "closeOnEscape">> &
    Omit<FocusTrapOptions, "enabled" | "trapFocus" | "closeOnEscape">;
  trigger: HTMLElement | null;
};

const trapStack: FocusTrapEntry[] = [];
let listenersInstalled = false;

function isHTMLElement(value: unknown): value is HTMLElement {
  return typeof HTMLElement !== "undefined" && value instanceof HTMLElement;
}

function isVisible(element: HTMLElement): boolean {
  if (element.hidden || element.closest("[hidden], [inert], [aria-hidden='true']")) return false;
  if (element.getAttribute("aria-disabled") === "true") return false;
  if (element.closest("fieldset[disabled]")) return false;

  if (typeof window === "undefined") return true;
  let current: HTMLElement | null = element;
  while (current) {
    const style = window.getComputedStyle(current);
    if (style.display === "none" || style.visibility === "hidden") return false;
    current = current.parentElement;
  }
  return true;
}

export function getFocusableElements(node: HTMLElement): HTMLElement[] {
  return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (element) => element.tabIndex >= 0 && isVisible(element),
  );
}

function resolveTarget(
  target: FocusTarget,
  scope: ParentNode,
): HTMLElement | null {
  if (typeof target === "function") return target() ?? null;
  if (isHTMLElement(target)) return target;
  if (typeof target !== "string") return null;

  try {
    return scope.querySelector<HTMLElement>(target);
  } catch {
    return null;
  }
}

function currentTrap(): FocusTrapEntry | undefined {
  return trapStack.at(-1);
}

function focusEntry(entry: FocusTrapEntry, fallbackToContainer = true): boolean {
  const { initialFocus } = entry.options;
  if (initialFocus === false) return false;

  const explicit = initialFocus === "auto" || initialFocus == null
    ? null
    : resolveTarget(initialFocus, entry.node);
  const candidates = [
    explicit,
    entry.node.querySelector<HTMLElement>("[data-autofocus]"),
    ...getFocusableElements(entry.node),
    fallbackToContainer ? entry.node : null,
  ];

  for (const target of candidates) {
    if (!target?.isConnected) continue;
    target.focus({ preventScroll: true });
    if (document.activeElement === target) return true;
  }
  return false;
}

function trapTab(entry: FocusTrapEntry, event: KeyboardEvent) {
  const focusable = getFocusableElements(entry.node);
  if (focusable.length === 0) {
    event.preventDefault();
    event.stopImmediatePropagation();
    entry.node.focus({ preventScroll: true });
    return;
  }

  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  const active = isHTMLElement(document.activeElement) ? document.activeElement : null;
  const activeInside = active ? entry.node.contains(active) : false;

  if (event.shiftKey && (!activeInside || active === first || active === entry.node)) {
    event.preventDefault();
    event.stopImmediatePropagation();
    last.focus({ preventScroll: true });
  } else if (!event.shiftKey && (!activeInside || active === last)) {
    event.preventDefault();
    event.stopImmediatePropagation();
    first.focus({ preventScroll: true });
  }
}

function handleDocumentKeydown(event: KeyboardEvent) {
  const entry = currentTrap();
  if (!entry) return;

  if (event.key === "Escape" && entry.options.closeOnEscape && entry.options.onEscape) {
    event.preventDefault();
    event.stopImmediatePropagation();
    entry.options.onEscape(event);
    return;
  }

  if (event.key === "Tab" && entry.options.trapFocus) {
    trapTab(entry, event);
  }
}

function handleDocumentFocusIn(event: FocusEvent) {
  const entry = currentTrap();
  if (!entry?.options.trapFocus) return;
  const target = event.target;
  if (target instanceof Node && entry.node.contains(target)) return;

  queueMicrotask(() => {
    if (currentTrap() === entry && !entry.node.contains(document.activeElement)) {
      focusEntry(entry);
    }
  });
}

function installListeners() {
  if (listenersInstalled || typeof document === "undefined") return;
  document.addEventListener("keydown", handleDocumentKeydown, true);
  document.addEventListener("focusin", handleDocumentFocusIn, true);
  listenersInstalled = true;
}

function removeListenersWhenIdle() {
  if (!listenersInstalled || trapStack.length > 0 || typeof document === "undefined") return;
  document.removeEventListener("keydown", handleDocumentKeydown, true);
  document.removeEventListener("focusin", handleDocumentFocusIn, true);
  listenersInstalled = false;
}

function normalizeOptions(options: FocusTrapOptions): FocusTrapEntry["options"] {
  return {
    ...options,
    enabled: options.enabled ?? true,
    trapFocus: options.trapFocus ?? true,
    closeOnEscape: options.closeOnEscape ?? true,
    initialFocus: options.initialFocus ?? "auto",
    returnFocus: options.returnFocus ?? true,
  };
}

function restoreEntryFocus(entry: FocusTrapEntry) {
  const { returnFocus } = entry.options;
  if (returnFocus === false) return;

  const target = returnFocus === true || returnFocus == null
    ? entry.trigger
    : resolveTarget(returnFocus, document);

  queueMicrotask(() => {
    if (target?.isConnected) target.focus({ preventScroll: true });
  });
}

/**
 * Modal-quality focus management for Drawer/Dialog/DetailPanel primitives.
 *
 * Traps are kept in a stack so only the top-most overlay handles Tab/Escape.
 * Closing a nested trap restores focus to the element that opened it; closing
 * the outer trap then restores the original page trigger.
 */
export function focusTrap(node: HTMLElement, initialOptions: FocusTrapOptions = {}) {
  let entry: FocusTrapEntry | undefined;
  let options = normalizeOptions(initialOptions);

  function activate() {
    if (entry || !options.enabled) return;

    entry = {
      node,
      options,
      trigger: isHTMLElement(document.activeElement) ? document.activeElement : null,
    };
    trapStack.push(entry);
    installListeners();

    queueMicrotask(() => {
      if (entry && currentTrap() === entry && entry.node.isConnected) focusEntry(entry);
    });
  }

  function deactivate(restore = true) {
    if (!entry) return;
    const closingEntry = entry;
    const index = trapStack.indexOf(closingEntry);
    if (index >= 0) trapStack.splice(index, 1);
    entry = undefined;
    removeListenersWhenIdle();
    if (restore) restoreEntryFocus(closingEntry);
  }

  activate();

  return {
    update(nextOptions: FocusTrapOptions = {}) {
      const wasEnabled = options.enabled;
      options = normalizeOptions(nextOptions);

      if (entry) entry.options = options;
      if (wasEnabled && !options.enabled) deactivate();
      else if (!wasEnabled && options.enabled) activate();
    },
    destroy() {
      deactivate();
    },
  };
}
