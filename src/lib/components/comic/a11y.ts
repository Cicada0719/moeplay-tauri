export type RovingOrientation = "horizontal" | "vertical";

export function nextComicRovingIndex(
  key: string,
  index: number,
  length: number,
  orientation: RovingOrientation = "horizontal",
): number | null {
  if (length <= 0) return null;
  const previousKey = orientation === "horizontal" ? "ArrowLeft" : "ArrowUp";
  const nextKey = orientation === "horizontal" ? "ArrowRight" : "ArrowDown";
  if (key === nextKey) return (index + 1) % length;
  if (key === previousKey) return (index - 1 + length) % length;
  if (key === "Home") return 0;
  if (key === "End") return length - 1;
  return null;
}

export function focusComicRovingItem(
  items: Array<HTMLElement | null | undefined>,
  index: number,
): void {
  queueMicrotask(() => items[index]?.focus({ preventScroll: true }));
}
