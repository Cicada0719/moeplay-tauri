type FocusTrapOptions = {
  initialFocus?: boolean;
};

const FOCUSABLE_SELECTORS = [
  'button:not([disabled])',
  'a[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

export function focusTrap(node: HTMLElement, options: FocusTrapOptions = {}) {
  let initialFocus = options.initialFocus ?? true;
  const previouslyFocused = document.activeElement as HTMLElement | null;

  function getInnerFocusables(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTORS)).filter(
      (el) => el.offsetParent !== null && el.tabIndex >= 0,
    );
  }

  function getFocusables(): HTMLElement[] {
    const inner = getInnerFocusables();
    return inner.length > 0 ? [node, ...inner] : [node];
  }

  function focusInitial() {
    if (!initialFocus) return;
    node.focus();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== "Tab") return;

    const focusable = getFocusables();
    if (focusable.length === 0) {
      event.preventDefault();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    const active = document.activeElement as HTMLElement;

    if (event.shiftKey) {
      if (active === first || !node.contains(active)) {
        event.preventDefault();
        last.focus();
      }
    } else {
      if (active === last || !node.contains(active)) {
        event.preventDefault();
        first.focus();
      }
    }
  }

  focusInitial();
  node.addEventListener("keydown", handleKeydown);

  return {
    update(next: FocusTrapOptions = {}) {
      initialFocus = next.initialFocus ?? true;
      if (initialFocus && document.activeElement !== node && !node.contains(document.activeElement as Node)) {
        focusInitial();
      }
    },
    destroy() {
      node.removeEventListener("keydown", handleKeydown);
      if (previouslyFocused && typeof previouslyFocused.focus === "function") {
        previouslyFocused.focus();
      }
    },
  };
}
