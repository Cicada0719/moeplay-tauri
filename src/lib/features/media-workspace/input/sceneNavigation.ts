export interface SceneEntryLike {
  ownerItemId: string | null;
}

export interface WheelGestureTracker {
  push(deltaX: number, deltaY: number, timestamp: number): -1 | 0 | 1;
  reset(): void;
}

export function createWheelGestureTracker(
  threshold = 22,
  quietWindowMs = 180,
): WheelGestureTracker {
  let accumulated = 0;
  let stepped = false;
  let lastTimestamp = Number.NEGATIVE_INFINITY;

  return {
    push(deltaX, deltaY, timestamp) {
      if (timestamp - lastTimestamp > quietWindowMs) {
        accumulated = 0;
        stepped = false;
      }
      lastTimestamp = timestamp;
      const delta = Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : deltaY;
      if (!Number.isFinite(delta) || delta === 0 || stepped) return 0;
      accumulated += delta;
      if (Math.abs(accumulated) < threshold) return 0;
      stepped = true;
      return accumulated > 0 ? 1 : -1;
    },
    reset() {
      accumulated = 0;
      stepped = false;
      lastTimestamp = Number.NEGATIVE_INFINITY;
    },
  };
}

export function wrapSceneIndex(index: number, length: number): number {
  if (length <= 0) return 0;
  return ((index % length) + length) % length;
}

export function adjacentMediaIndex(
  entries: readonly SceneEntryLike[],
  activeIndex: number,
  direction: -1 | 1,
): number {
  if (entries.length <= 1) return 0;
  const current = entries[wrapSceneIndex(activeIndex, entries.length)];
  const owned = entries
    .map((entry, index) => ({ entry, index }))
    .filter(({ entry }) => entry.ownerItemId === current?.ownerItemId)
    .map(({ index }) => index);
  if (owned.length <= 1) return wrapSceneIndex(activeIndex, entries.length);
  const ownedIndex = Math.max(0, owned.indexOf(wrapSceneIndex(activeIndex, entries.length)));
  return owned[wrapSceneIndex(ownedIndex + direction, owned.length)];
}

export function adjacentGameIndex(
  entries: readonly SceneEntryLike[],
  activeIndex: number,
  direction: -1 | 1,
): number {
  if (entries.length <= 1) return 0;
  const owners = entries.reduce<string[]>((result, entry) => {
    if (entry.ownerItemId && !result.includes(entry.ownerItemId)) result.push(entry.ownerItemId);
    return result;
  }, []);
  if (owners.length <= 1) return wrapSceneIndex(activeIndex, entries.length);
  const currentOwner = entries[wrapSceneIndex(activeIndex, entries.length)]?.ownerItemId;
  const ownerIndex = Math.max(0, owners.indexOf(currentOwner ?? ""));
  const nextOwner = owners[wrapSceneIndex(ownerIndex + direction, owners.length)];
  const nextIndex = entries.findIndex((entry) => entry.ownerItemId === nextOwner);
  return nextIndex >= 0 ? nextIndex : wrapSceneIndex(activeIndex, entries.length);
}

export function dragSceneStep(
  distance: number,
  velocity: number,
  viewportWidth: number,
): -1 | 0 | 1 {
  const threshold = Math.max(48, Math.min(120, viewportWidth * 0.09));
  if (Math.abs(distance) < threshold && Math.abs(velocity) < 0.42) return 0;
  return distance < 0 || velocity < -0.42 ? 1 : -1;
}
