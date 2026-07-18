import { describe, expect, it } from "vitest";
import { ENHANCEMENT_PROFILES, resolveEnhancementSize } from "./localVideoEnhancement";

describe("local video enhancement sizing", () => {
  it("upscales 720p to a 1080p-class balanced target", () => {
    expect(resolveEnhancementSize(1280, 720, 1280, 720, 1, "balanced")).toEqual({ width: 1920, height: 1080 });
  });

  it("caps quality mode at 1440p to protect playback latency", () => {
    expect(resolveEnhancementSize(1920, 1080, 1920, 1080, 2, "quality")).toEqual({ width: 2560, height: 1440 });
    expect(ENHANCEMENT_PROFILES.quality.strength).toBeGreaterThan(ENHANCEMENT_PROFILES.balanced.strength);
  });

  it("preserves portrait and non-16:9 source aspect ratios", () => {
    expect(resolveEnhancementSize(720, 1280, 720, 1280, 1, "balanced")).toEqual({ width: 608, height: 1080 });
  });
});
