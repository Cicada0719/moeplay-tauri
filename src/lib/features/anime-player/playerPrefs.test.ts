import { beforeEach, describe, expect, it } from "vitest";
import { playerPrefs } from "./playerPrefs.svelte";

describe("playerPrefs.svelte 播放器偏好", () => {
  beforeEach(() => {
    localStorage.clear();
    // 重置到默认档
    playerPrefs.autoNext = true;
    playerPrefs.playbackRate = 1;
    playerPrefs.longPressRate = 3;
    playerPrefs.skipOpening = 0;
    playerPrefs.skipEnding = 0;
    playerPrefs.autoWebFallback = true;
    playerPrefs.videoEnhancementMode = "off";
  });

  it("默认值正确", () => {
    expect(playerPrefs.autoNext).toBe(true);
    expect(playerPrefs.playbackRate).toBe(1);
    expect(playerPrefs.longPressRate).toBe(3);
    expect(playerPrefs.skipOpening).toBe(0);
    expect(playerPrefs.skipEnding).toBe(0);
    expect(playerPrefs.autoWebFallback).toBe(true);
    expect(playerPrefs.videoEnhancementMode).toBe("off");
  });

  it("写入并持久化到 localStorage", () => {
    playerPrefs.autoNext = false;
    playerPrefs.playbackRate = 1.5;
    playerPrefs.skipOpening = 8;
    playerPrefs.videoEnhancementMode = "balanced";
    expect(localStorage.getItem("player-auto-next")).toBe("false");
    expect(localStorage.getItem("player-playback-rate")).toBe("1.5");
    expect(localStorage.getItem("player-skip-opening")).toBe("8");
    expect(localStorage.getItem("player-video-enhancement")).toBe(JSON.stringify("balanced"));
    expect(playerPrefs.playbackRate).toBe(1.5);
  });

  it("非法画质增强值被归一化为 off", () => {
    playerPrefs.videoEnhancementMode = "garbage" as never;
    expect(playerPrefs.videoEnhancementMode).toBe("off");
  });
});
