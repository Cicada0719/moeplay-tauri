export type StepDirection = -1 | 1;

export interface WheelStepperOptions {
  threshold?: number;
  cooldownMs?: number;
  gestureIdleMs?: number;
  axisLockRatio?: number;
}

export interface WheelSample {
  deltaX: number;
  deltaY: number;
  time: number;
}

export interface WheelStepper {
  push(sample: WheelSample): StepDirection | null;
  reset(): void;
}

const DEFAULT_THRESHOLD = 72;
const DEFAULT_COOLDOWN_MS = 450;
const DEFAULT_GESTURE_IDLE_MS = 180;
const DEFAULT_AXIS_LOCK_RATIO = 1.15;

export function createWheelStepper(options: WheelStepperOptions = {}): WheelStepper {
  const threshold = Math.max(1, options.threshold ?? DEFAULT_THRESHOLD);
  const cooldownMs = Math.max(0, options.cooldownMs ?? DEFAULT_COOLDOWN_MS);
  const gestureIdleMs = Math.max(1, options.gestureIdleMs ?? DEFAULT_GESTURE_IDLE_MS);
  const axisLockRatio = Math.max(1, options.axisLockRatio ?? DEFAULT_AXIS_LOCK_RATIO);

  let accumulated = 0;
  let lockedDirection: StepDirection | null = null;
  let gestureConsumed = false;
  let lastSampleAt = Number.NEGATIVE_INFINITY;
  let lastStepAt = Number.NEGATIVE_INFINITY;

  function resetGesture(): void {
    accumulated = 0;
    lockedDirection = null;
    gestureConsumed = false;
  }

  return {
    push({ deltaX, deltaY, time }) {
      if (!Number.isFinite(time) || !Number.isFinite(deltaX) || !Number.isFinite(deltaY)) return null;
      if (time - lastSampleAt > gestureIdleMs) resetGesture();
      lastSampleAt = time;

      if (gestureConsumed) return null;

      const vertical = Math.abs(deltaY);
      const horizontal = Math.abs(deltaX);
      if (vertical === 0 || vertical < horizontal * axisLockRatio) return null;

      const direction: StepDirection = deltaY > 0 ? 1 : -1;
      if (lockedDirection !== null && lockedDirection !== direction) {
        accumulated = 0;
      }
      lockedDirection = direction;
      accumulated += vertical;

      if (accumulated < threshold) return null;

      gestureConsumed = true;
      accumulated = 0;
      if (time - lastStepAt < cooldownMs) return null;

      lastStepAt = time;
      return direction;
    },
    reset() {
      resetGesture();
      lastSampleAt = Number.NEGATIVE_INFINITY;
    },
  };
}

export function adjacentIndex(length: number, currentIndex: number, direction: StepDirection): number {
  if (length <= 0) return -1;
  const normalized = currentIndex >= 0 && currentIndex < length ? currentIndex : 0;
  return (normalized + direction + length) % length;
}

export function adjacentItem<T extends { id: string }>(
  items: readonly T[],
  selectedId: string | null | undefined,
  direction: StepDirection,
): T | null {
  if (items.length === 0) return null;
  const currentIndex = items.findIndex((item) => item.id === selectedId);
  return items[adjacentIndex(items.length, currentIndex, direction)] ?? null;
}

export function shouldCaptureStageInput(options: {
  isInteractiveTarget: boolean;
  isScrollableSubregion: boolean;
}): boolean {
  return !options.isInteractiveTarget && !options.isScrollableSubregion;
}

export function normalizeWheelDelta(delta: number, deltaMode: number, viewportHeight = 800): number {
  if (deltaMode === 1) return delta * 16;
  if (deltaMode === 2) return delta * Math.max(1, viewportHeight);
  return delta;
}
