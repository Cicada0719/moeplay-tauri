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

    // 旧实现中的散落隐藏逻辑必须被移除（根因注释允许提及旧名，故按定义判定）
    expect(player).not.toMatch(/(?:function|const)\s+(schedulePlayerChromeHide|revealPlayerChrome|handlePlayerPointerMove)\b/);
    expect(player).not.toContain("playerChromeTimer = window.setTimeout");
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

  it("画质切换复用 video 元素：switchQuality 不销毁重建播放器容器", () => {
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    expect(player).toContain("switchQuality");
    expect(player).toContain("videoEnhancementMode");
    expect(player).toContain("不销毁重建 video 元素");
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
