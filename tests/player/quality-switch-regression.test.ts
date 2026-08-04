import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { get } from "svelte/store";

import AnimePlayer from "../../src/lib/components/anime/AnimePlayer.svelte";
import { animeStore } from "../../src/lib/stores/anime.svelte";
import { setMockInvokeHandler, clearMockInvokeHandler } from "../../src/lib/api/core";
import {
  clearPlayerError,
  controlsVisible,
  openMenuCount,
  playerError,
  type PlayerQuality,
} from "../../src/lib/stores/player";
import { settingsStore } from "../../src/lib/stores/settings.svelte";

const PROXY_URL = "http://127.0.0.1:17891/proxy/video.mp4";

function installInvokeMocks() {
  setMockInvokeHandler((command) => {
    switch (command) {
      case "anime_build_url":
        return "http://episode/1";
      case "anime_extract_video_url":
        return { url: "http://cdn.example/video.mp4", tab_url: "http://page.example" };
      case "get_video_proxy_port":
        return 17891;
      case "anime_get_proxy_url":
        return PROXY_URL;
      default:
        return null;
    }
  });
}

function primePlaybackRoad() {
  animeStore.setRoadsForPlayback(
    [{ name: "线路1", episodes: [{ name: "第1集", url: "http://episode/1" }] }],
    "ruleA",
    "http://source",
  );
}

/**
 * spec §6.2 回归路径（核心）：超清 → 普清 → 超清 来回切换 5 次。
 *
 * 完整 Tauri 运行时（真实 HLS 解码 / WebGL2 增强）无法在 Vitest/happy-dom 中运行，
 * 故按「组件级测试 + 说明」策略落地：挂载真实 AnimePlayer，mock IPC（源提取/代理），
 * 驱动到播放就绪态后点击「切换本地超清化」按钮 5 次，断言：
 *  - 复用同一 <video> 元素（不销毁重建，FR-06 根因）；
 *  - 纯增强模式切换（同源）不重载媒体（el.load() 调用数不增长，Kimi K3 复审）；
 *  - 画质档位按 off→均衡→质量→off→均衡→质量 轮转；
 *  - 全程不触发播放错误（ErrorOverlay 不弹出）。
 */
describe("AnimePlayer 5 次超清/普清来回切换回归（spec §6.2）", () => {
  beforeEach(() => {
    clearPlayerError();
    controlsVisible.set(true);
    openMenuCount.set(0);
    animeStore.videoEnhancementMode = "off";
    // 避免 onMount 全屏守卫定时器在测试环境触发 Tauri 调用（噪声）
    settingsStore.settings.startup_mode = "windowed";
    installInvokeMocks();
    primePlaybackRoad();
  });

  afterEach(() => {
    clearMockInvokeHandler();
  });

  it("来回切换 5 次：video 元素复用、同源不重载、档位轮转正确", async () => {
    render(AnimePlayer);
    await animeStore.playEpisode(0, 0);

    // 等播放器进入 found 态、<video> 元素渲染完成
    await waitFor(() => {
      expect(document.querySelector(".player-video")).toBeTruthy();
    });
    const video = document.querySelector(".player-video") as HTMLVideoElement;
    expect(video).toBeTruthy();

    // 记录初始媒体加载（attachNative 的 el.load()）基线，之后纯增强切换不得再触发重载
    const loadSpy = vi.spyOn(HTMLMediaElement.prototype, "load").mockImplementation(() => {});
    const loadCallsBefore = loadSpy.mock.calls.length;

    const toggleButton = () => screen.getByRole("button", { name: "切换本地超清化模式" });

    const expectedCycle: PlayerQuality[] = ["balanced", "quality", "off", "balanced", "quality"];
    try {
      for (const mode of expectedCycle) {
        await userEvent.click(toggleButton());
        // 档位已更新
        expect(animeStore.videoEnhancementMode).toBe(mode);
        // 同一 <video> 元素被复用，未被销毁重建
        expect(document.querySelector(".player-video")).toBe(video);
        // 纯增强模式切换不重载同源媒体
        expect(loadSpy.mock.calls.length).toBe(loadCallsBefore);
        // 不触发播放错误
        expect(get(playerError)).toBeNull();
      }
    } finally {
      loadSpy.mockRestore();
    }

    // 5 次切换后源地址保持不变（全程同源）
    expect(animeStore.playerVideoSrc).toBe(PROXY_URL);
  });
});
