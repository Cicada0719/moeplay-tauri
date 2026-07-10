import { afterEach, describe, expect, it } from "vitest";
import { motionStore } from "../stores/motion.svelte";
import { motionDuration } from "./motion";

afterEach(() => {
  motionStore.setPreference("system");
  delete document.documentElement.dataset.motion;
});

describe("motion helpers", () => {
  it("uses the requested duration when motion is enabled", () => {
    motionStore.setPreference("full");
    expect(motionDuration(0.24)).toBe(0.24);
  });

  it("returns an immediate duration when reduced motion is enabled", () => {
    motionStore.setPreference("reduce");
    expect(motionDuration(0.24)).toBe(0);
  });
});
