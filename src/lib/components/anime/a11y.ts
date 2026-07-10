export type RovingDirectionKey = "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End";

export function nextRovingIndex(
  key: string,
  currentIndex: number,
  itemCount: number,
  orientation: "horizontal" | "vertical" | "both" = "horizontal",
): number | null {
  if (itemCount <= 0) return null;
  const current = Math.min(Math.max(currentIndex, 0), itemCount - 1);
  if (key === "Home") return 0;
  if (key === "End") return itemCount - 1;

  const previous = key === "ArrowLeft" || key === "ArrowUp";
  const next = key === "ArrowRight" || key === "ArrowDown";
  if (!previous && !next) return null;
  if (orientation === "horizontal" && (key === "ArrowUp" || key === "ArrowDown")) return null;
  if (orientation === "vertical" && (key === "ArrowLeft" || key === "ArrowRight")) return null;
  return previous ? (current - 1 + itemCount) % itemCount : (current + 1) % itemCount;
}

export function focusRovingItem(
  items: Array<HTMLElement | null | undefined>,
  index: number,
): void {
  queueMicrotask(() => items[index]?.focus({ preventScroll: true }));
}

export function focusWhenAvailable(
  target: string | (() => HTMLElement | null | undefined),
  attempts = 10,
): void {
  const resolve = () => typeof target === "string"
    ? document.querySelector<HTMLElement>(target)
    : target();
  let remaining = attempts;
  const focus = () => {
    const element = resolve();
    if (element && element.isConnected && !element.hasAttribute("disabled")) {
      element.focus({ preventScroll: true });
      element.scrollIntoView({ block: "nearest", inline: "nearest" });
      return;
    }
    remaining -= 1;
    if (remaining > 0) window.setTimeout(focus, 40);
  };
  queueMicrotask(focus);
}
