import { describe, expect, it } from "vitest";
import { effectiveOrientation, enterMediaLandscape, enterVideoFullscreen, exitMediaLandscape, exitVideoFullscreen } from "./orientation-policy";

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

  it("keeps media pages landscape after video fullscreen exits", () => {
    const media = enterMediaLandscape({ preferred: "portrait", temporary: null, videoAutoLandscape: true });
    const fullscreen = enterVideoFullscreen(media);
    expect(effectiveOrientation(fullscreen)).toBe("landscape");
    const videoClosed = exitVideoFullscreen(fullscreen);
    expect(effectiveOrientation(videoClosed)).toBe("landscape");
    expect(effectiveOrientation(exitMediaLandscape(videoClosed))).toBe("portrait");
  });

  it("does not restore the preferred mode until every media surface exits", () => {
    const first = enterMediaLandscape({ preferred: "portrait", temporary: null, videoAutoLandscape: true });
    const second = enterMediaLandscape(first);
    expect(second.mediaLandscapeCount).toBe(2);
    expect(effectiveOrientation(exitMediaLandscape(second))).toBe("landscape");
    expect(effectiveOrientation(exitMediaLandscape(exitMediaLandscape(second)))).toBe("portrait");
  });
});
