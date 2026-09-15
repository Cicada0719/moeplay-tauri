import { afterEach, describe, expect, it, vi } from "vitest";
import { clearMockInvokeHandler, setMockInvokeHandler } from "./core";
import { animeCancelExtract, animeExtractVideoUrl } from "./videoExtractor";

describe("video extractor API", () => {
  afterEach(() => clearMockInvokeHandler());

  it("passes optional session and scope through while keeping the result metadata", async () => {
    const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      expect(command).toBe("anime_extract_video_url");
      expect(args).toMatchObject({ sessionId: "session-1", scope: "play:1" });
      return {
        url: "https://cdn.test/video.m3u8",
        source: "webresource:m3u8-body",
        tab_url: "https://player.test/episode/1",
        session_id: "session-1",
      };
    });
    setMockInvokeHandler(invoke);

    await expect(
      animeExtractVideoUrl({
        episodeUrl: "https://player.test/episode/1",
        useLegacyParser: false,
        sessionId: "session-1",
        scope: "play:1",
      }),
    ).resolves.toMatchObject({ session_id: "session-1" });
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("sends scope to the cancellation command", async () => {
    const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      expect(command).toBe("anime_cancel_extract");
      expect(args).toEqual({ scope: "play:1" });
    });
    setMockInvokeHandler(invoke);

    await animeCancelExtract("play:1");
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
