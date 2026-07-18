import { describe, expect, it } from "vitest";
import { effectiveOrientation, enterVideoFullscreen, exitVideoFullscreen } from "./orientation-policy";

describe("mobile orientation policy", () => {
  it("temporarily enters landscape and restores portrait", () => {
    const preferred = { preferred: "portrait" as const, temporary: null, videoAutoLandscape: true };
    const fullscreen = enterVideoFullscreen(preferred);
    expect(effectiveOrientation(fullscreen)).toBe("landscape");
    expect(effectiveOrientation(exitVideoFullscreen(fullscreen))).toBe("portrait");
  });

  it("restores automatic orientation after video fullscreen", () => {
    const preferred = { preferred: "auto" as const, temporary: null, videoAutoLandscape: true };
    expect(effectiveOrientation(exitVideoFullscreen(enterVideoFullscreen(preferred)))).toBe("auto");
  });

  it("does not override the user's mode when auto-landscape is disabled", () => {
    const preferred = { preferred: "portrait" as const, temporary: null, videoAutoLandscape: false };
    expect(enterVideoFullscreen(preferred)).toBe(preferred);
    expect(effectiveOrientation(preferred)).toBe("portrait");
  });
});
