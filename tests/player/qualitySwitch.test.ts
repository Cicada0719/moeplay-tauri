import { describe, expect, it } from "vitest";
import { shouldReloadMedia, type QualitySwitchReloadContext } from "../../src/lib/player/qualitySwitch";

function ctx(overrides: Partial<QualitySwitchReloadContext> = {}): QualitySwitchReloadContext {
  return {
    el: {} as HTMLVideoElement,
    targetSrc: "http://127.0.0.1:17891/proxy/master.m3u8",
    loadedSrc: "http://127.0.0.1:17891/proxy/master.m3u8",
    quality: "balanced",
    currentQuality: "off",
    ...overrides,
  };
}

/**
 * spec §6.2 回归 + Kimi K3 复审 medium：
 * switchQuality 仅当 targetSrc 实际变化时才重载媒体。纯增强模式切换（本地超清化
 * off/均衡/质量，同源）只更新 enhancement 管线状态，不重载媒体。
 */
describe("switchQuality 媒体重载判定（shouldReloadMedia）", () => {
  it("纯增强模式切换（targetSrc 未变化）不重载", () => {
    // 同源 + 档位变化：本地超清化 off→均衡，不重载媒体
    expect(shouldReloadMedia(ctx())).toBe(false);
    // 同源 + 档位变化（均衡→质量）
    expect(
      shouldReloadMedia(ctx({ currentQuality: "balanced", quality: "quality" })),
    ).toBe(false);
  });

  it("targetSrc 实际变化时重载（复用同一 video 元素替换 source）", () => {
    expect(
      shouldReloadMedia(
        ctx({ loadedSrc: "http://127.0.0.1:17891/proxy/old.mp4", targetSrc: "http://127.0.0.1:17891/proxy/new.mp4" }),
      ),
    ).toBe(true);
  });

  it("档位未变化（重复点击同一档位）不重载", () => {
    expect(
      shouldReloadMedia(ctx({ currentQuality: "balanced", quality: "balanced" })),
    ).toBe(false);
  });

  it("video 元素缺失时不重载（静默忽略）", () => {
    expect(shouldReloadMedia(ctx({ el: null }))).toBe(false);
  });

  it("targetSrc 为空时不重载（媒体尚未就绪）", () => {
    expect(shouldReloadMedia(ctx({ targetSrc: "" }))).toBe(false);
  });
});
