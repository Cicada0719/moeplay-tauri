import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function source(path: string): string {
  return readFileSync(resolve(process.cwd(), path), "utf8");
}

/**
 * FR-06 播放器控制栏隐藏契约测试：
 * 断言散落的隐藏定时器/pointermove 监听已被 `useIdleTimer` 收敛，
 * 且控制栏复现/隐藏过渡耗时 ≤ 200ms。
 */
describe("player idle chrome contract", () => {
  it("控制栏隐藏逻辑收敛为单一 useIdleTimer，不再存在散落的 chrome 定时器/mousemove 监听", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");

    expect(player).toContain("use:idleTimer");
    expect(player).toContain("controlsVisible");
    expect(player).toContain("openMenuCount");
    expect(player).toContain("shouldPauseIdle");

    // 必须用稳定引用，避免每次渲染生成新对象触发 action.update() 重置空闲计时
    expect(player).toContain("const idleTimerOptions");
    expect(player).toContain("use:idleTimer={idleTimerOptions}");
    expect(player).not.toContain("use:idleTimer={{");

    // spec §3.2：controlsVisible → 容器 idle class 由模板单一绑定 class:idle={$controlsVisible === false}
    // 驱动；不再挂载重复的 controlsIdleClass action（单一职责：idleTimer 只管计时，
    // controlsVisible 为单一事实源，模板直接由此派生 .idle）
    expect(player).not.toContain("use:controlsIdleClass");
    expect(player).toMatch(/class:idle=\{\s*\$controlsVisible\s*===\s*false\s*\}/);
    expect(player).not.toContain("class:chrome-hidden={");

    // 旧实现中的散落隐藏逻辑必须被移除（根因注释允许提及旧名，故按定义判定）
    expect(player).not.toMatch(/(?:function|const)\s+(schedulePlayerChromeHide|revealPlayerChrome|handlePlayerPointerMove)\b/);
    expect(player).not.toContain("playerChromeTimer = window.setTimeout");
  });

  it("shell 顶部栏隐藏由 chromeVisible 属性驱动，不再 :global 侵入 anime-playback-shell 内部 class", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    // 空闲隐藏交给 shell 自带的 chromeVisible prop（--chrome-hidden 规则在 anime-player.css 内）
    expect(player).toContain("chromeVisible={$controlsVisible}");
    expect(player).not.toContain(":global(.anime-playback-shell__");
  });

  it("控制栏隐藏/复现过渡耗时 ≤ 200ms（shell CSS）", () => {
    const css = source("src/lib/styles/anime-player.css");
    const block =
      css.match(/\.anime-playback-shell--fullscreen\.anime-playback-shell--chrome-hidden[\s\S]*?\}/)?.[0] ?? "";

    expect(block).toContain("opacity 160ms");
    expect(block).toContain("transform 200ms");

    const durations = [...block.matchAll(/(\d+)ms/g)].map((m) => parseInt(m[1], 10));
    expect(durations.length).toBeGreaterThan(0);
    expect(Math.max(...durations)).toBeLessThanOrEqual(200);
  });

  it("class:idle 绑定的 .player-overlay.idle 下 Controls 隐藏过渡 ≤ 200ms（spec §3.2）", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    expect(player).toContain(".player-overlay.idle");
    expect(player).toContain("cursor: none");
    expect(player).toMatch(/opacity 160ms/);
    expect(player).toMatch(/transform 200ms/);
    expect(player).toMatch(/pointer-events: none/);
  });

  it("画质切换复用 video 元素：switchQuality 不销毁重建播放器容器", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    expect(player).toContain("switchQuality");
    expect(player).toContain("videoEnhancementMode");
    expect(player).toContain("不销毁重建 video 元素");
  });

  it("onDestroy 注销重试/源切换处理器与 store 状态，避免跨实例泄漏", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    expect(player).toContain("setRetryHandler(null)");
    expect(player).toContain("setSourceSwitchHandler(null)");
    expect(player).toContain("openMenuCount.set(0)");
    expect(player).toContain("clearPlayerError()");
  });

  it("switchQuality 直接替换 video 源并恢复进度，复用 HLS 实例（spec §3.3）", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    // 不再自增 reload token 重建整个媒体初始化 effect
    expect(player).not.toContain("mediaReloadToken");
    // 复用同一 video 元素，不销毁重建容器（FR-06 根因）
    expect(player).toContain("不销毁重建 video 元素");
    // 真正替换 source：HLS 复用实例 loadSource / 原生重新 src+load
    expect(player).toContain("activeHls.loadSource(targetSrc)");
    expect(player).toContain("el.src = targetSrc");
    // loadedmetadata 后按画质切换前的进度 seek 回原位置并恢复播放状态
    expect(player).toContain("pendingQualitySeek");
    expect(player).toContain("pendingQualitySeekSrc");
    expect(player).toContain("resumeAfterQualityLoad");
  });

  it("HLS 画质切换：MANIFEST_PARSED 后消费 pendingQualitySeek 恢复进度，并按切换前状态条件恢复播放", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    // switchQuality HLS 分支复用同一实例 loadSource（不销毁重建）
    expect(player).toContain("activeHls.loadSource(targetSrc)");

    // MANIFEST_PARSED handler 内必须消费 pendingQualitySeek：HLS 下 loadedmetadata 的
    // src 比对不适用（video.src 是 MediaSource/blob URL，不等于 m3u8 源地址），
    // 因此恢复 seek 与播放状态必须在 HLS 事件上完成。
    const manifestBlock =
      player.match(/hls\.on\(Hls\.Events\.MANIFEST_PARSED[\s\S]*?\n\s*\}\);/)?.[0] ?? "";
    expect(manifestBlock).toContain("pendingQualitySeekSrc === src");
    expect(manifestBlock).toContain("v.currentTime = pendingQualitySeek");
    expect(manifestBlock).toContain("pendingQualitySeek = 0");
    // 恢复后按切换前状态决定是否 play()：切换前 paused 保持暂停
    expect(manifestBlock).toContain("const shouldPlay = resumeAfterQualityLoad;");
    expect(manifestBlock).toMatch(/if \(shouldPlay\) v\.play\(\)\.catch/);
  });

  it("SourceSuggestSheet 消费适配层源健康 store，关闭时清除错误状态（空列表可退出）", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    expect(player).toContain("setSourceProvider(buildSuggestSources)");
    expect(player).toContain("setSourceProvider(null)");
    expect(player).toContain('contentType="anime"');
    expect(player).not.toContain("sources={suggestSources}");
    expect(player).toMatch(/showSourceSuggest\.set\(false\);\s*clearPlayerError\(\);/);
  });

  it("FR-07 错误降级组件与源切换适配层已接入", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    expect(player).toContain("<ErrorOverlay");
    expect(player).toContain("<SourceSuggestSheet");
    expect(player).toContain("switchSourceService");
    expect(player).toContain("reportPlayerError");
    expect(player).toContain("retryPlayback");
  });
});
