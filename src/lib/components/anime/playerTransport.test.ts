import { describe, expect, it } from "vitest";
import { shouldPreferHls } from "./playerTransport";

describe("anime player transport", () => {
  it("prefers hls.js for Android proxied playlists even when native HLS says maybe", () => {
    expect(shouldPreferHls({ isM3u8: true, nativeHls: true, hlsSupported: true, isAndroid: true })).toBe(true);
  });

  it("keeps native HLS as the desktop first choice", () => {
    expect(shouldPreferHls({ isM3u8: true, nativeHls: true, hlsSupported: true, isAndroid: false })).toBe(false);
  });

  it("does not select hls.js for direct files or unsupported browsers", () => {
    expect(shouldPreferHls({ isM3u8: false, nativeHls: false, hlsSupported: true, isAndroid: true })).toBe(false);
    expect(shouldPreferHls({ isM3u8: true, nativeHls: false, hlsSupported: false, isAndroid: true })).toBe(false);
  });
});
