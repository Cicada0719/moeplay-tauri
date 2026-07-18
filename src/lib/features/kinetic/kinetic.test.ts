import { readFileSync } from "node:fs";
import { cleanup, render, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import KineticStage from "./KineticStage.svelte";
import {
  KINETIC_STAGE_STORAGE_KEY,
  kineticStageStore,
  readKineticStageEnabled,
} from "./settings.svelte";
import { detectKineticQuality, KineticQualityGovernor } from "./quality";
import { isReducedMotionActive } from "./reducedMotion";

const { sceneMock } = vi.hoisted(() => ({
  sceneMock: {
    createKineticScene: vi.fn(),
    mount: vi.fn(),
    dispose: vi.fn(),
    setPalette: vi.fn(),
    setQuality: vi.fn(),
    resize: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
  },
}));

vi.mock("./webgl/KineticScene", () => ({
  createKineticScene: sceneMock.createKineticScene,
}));

function installMatchMedia(matches: (query: string) => boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn((query: string) => ({
      matches: matches(query),
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

function stubGetContext(webgl2Available: boolean) {
  const implementation = (contextId: string) =>
    webgl2Available && contextId === "webgl2" ? ({} as WebGL2RenderingContext) : null;
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    implementation as unknown as typeof HTMLCanvasElement.prototype.getContext,
  );
}

function stageRoot(container: HTMLElement): HTMLElement {
  const root = container.querySelector('[data-testid="kinetic-stage"]');
  expect(root).toBeInstanceOf(HTMLElement);
  return root as HTMLElement;
}

beforeEach(() => {
  installMatchMedia(() => false);
  localStorage.clear();
  sceneMock.createKineticScene.mockReset();
  sceneMock.mount.mockReset().mockResolvedValue(undefined);
  sceneMock.dispose.mockReset();
  sceneMock.setPalette.mockReset();
  sceneMock.setQuality.mockReset();
  sceneMock.createKineticScene.mockReturnValue({
    mount: sceneMock.mount,
    dispose: sceneMock.dispose,
    setPalette: sceneMock.setPalette,
    setQuality: sceneMock.setQuality,
    resize: sceneMock.resize,
    pause: sceneMock.pause,
    resume: sceneMock.resume,
  });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete document.documentElement.dataset.motion;
  kineticStageStore.setEnabled(true);
  localStorage.clear();
});

describe("kinetic stage switch (moeplay-kinetic-stage-v1)", () => {
  it("开关存在且默认开", () => {
    expect(KINETIC_STAGE_STORAGE_KEY).toBe("moeplay-kinetic-stage-v1");
    expect(readKineticStageEnabled()).toBe(true);
    expect(kineticStageStore.enabled).toBe(true);
  });

  it("关闭后持久化，重新读取为 false", () => {
    kineticStageStore.setEnabled(false);
    expect(kineticStageStore.enabled).toBe(false);
    expect(localStorage.getItem(KINETIC_STAGE_STORAGE_KEY)).toBe('{"enabled":false}');
    expect(readKineticStageEnabled()).toBe(false);
  });

  it("损坏的存储内容回退为默认开", () => {
    localStorage.setItem(KINETIC_STAGE_STORAGE_KEY, "{not-json");
    expect(readKineticStageEnabled()).toBe(true);
  });
});

describe("KineticStage 降级路径", () => {
  it("无 WebGL 上下文时渲染 fallback 且不抛错", async () => {
    stubGetContext(false);
    const { container } = render(KineticStage, { props: { enabled: true } });
    const stage = stageRoot(container);

    await waitFor(() => expect(stage.dataset.reason).toBe("no-webgl"));
    expect(stage.dataset.mode).toBe("fallback");
    expect(stage.querySelector(".kinetic-fallback")).toBeTruthy();
    expect(sceneMock.createKineticScene).not.toHaveBeenCalled();
  });

  it("开关关闭时渲染静止 fallback", async () => {
    stubGetContext(true);
    const { container } = render(KineticStage, { props: { enabled: false } });
    const stage = stageRoot(container);

    await waitFor(() => expect(stage.dataset.reason).toBe("disabled"));
    expect(stage.dataset.mode).toBe("fallback");
    const fallback = stage.querySelector(".kinetic-fallback") as HTMLElement;
    expect(fallback.dataset.animated).toBe("false");
    expect(sceneMock.createKineticScene).not.toHaveBeenCalled();
  });

  it("reduced-motion 信号一：media query 命中时走静止 fallback", async () => {
    installMatchMedia((query) => query.includes("prefers-reduced-motion"));
    stubGetContext(true);
    expect(isReducedMotionActive()).toBe(true);

    const { container } = render(KineticStage, { props: { enabled: true } });
    const stage = stageRoot(container);

    await waitFor(() => expect(stage.dataset.reason).toBe("reduced-motion"));
    expect(stage.dataset.mode).toBe("fallback");
    const fallback = stage.querySelector(".kinetic-fallback") as HTMLElement;
    expect(fallback.dataset.quality).toBe("reduced");
    expect(fallback.dataset.animated).toBe("false");
    expect(sceneMock.createKineticScene).not.toHaveBeenCalled();
  });

  it("reduced-motion 信号二：documentElement[data-motion=reduce] 时走静止 fallback", async () => {
    document.documentElement.dataset.motion = "reduce";
    stubGetContext(true);
    expect(isReducedMotionActive()).toBe(true);

    const { container } = render(KineticStage, { props: { enabled: true } });
    const stage = stageRoot(container);

    await waitFor(() => expect(stage.dataset.reason).toBe("reduced-motion"));
    expect(stage.dataset.mode).toBe("fallback");
    expect(sceneMock.createKineticScene).not.toHaveBeenCalled();
  });

  it("WebGL 路径挂载成功，卸载时完整 dispose", async () => {
    stubGetContext(true);
    const { container, unmount } = render(KineticStage, { props: { enabled: true } });
    const stage = stageRoot(container);

    await waitFor(() => expect(stage.dataset.mode).toBe("webgl"));
    expect(sceneMock.createKineticScene).toHaveBeenCalledTimes(1);
    expect(sceneMock.mount).toHaveBeenCalledTimes(1);
    expect(stage.querySelector(".kinetic-fallback")).toBeNull();

    unmount();
    expect(sceneMock.dispose).toHaveBeenCalledTimes(1);
  });
});

describe("质量分级", () => {
  it("能力快照映射 high/medium/low", () => {
    expect(detectKineticQuality({ webgl2: false })).toBe("low");
    expect(detectKineticQuality({ webgl2: true, hardwareConcurrency: 2 })).toBe("low");
    expect(detectKineticQuality({ webgl2: true, hardwareConcurrency: 8, saveData: true })).toBe("low");
    expect(detectKineticQuality({ webgl2: true, hardwareConcurrency: 4 })).toBe("medium");
    expect(detectKineticQuality({ webgl2: true, hardwareConcurrency: 8, deviceMemory: 8, devicePixelRatio: 1 })).toBe("high");
  });

  it("帧率治理器按实测帧率逐级降档，low 档仍卡顿则裁决 fallback", () => {
    let now = 0;
    const governor = new KineticQualityGovernor("high", { now: () => now });

    const feed = (deltaMs: number, frames: number) => {
      let verdict: ReturnType<typeof governor.sample> = null;
      for (let index = 0; index < frames; index += 1) {
        now += deltaMs;
        verdict = governor.sample(deltaMs) ?? verdict;
      }
      return verdict;
    };

    expect(feed(16.7, 90)).toBeNull();
    expect(feed(33, 90)).toBe("medium");
    now += 2500;
    expect(feed(33, 90)).toBe("low");
    now += 2500;
    expect(feed(80, 90)).toBe("fallback");
  });
});

describe("three 懒加载与设置页接线（源码契约）", () => {
  const readSource = (relative: string) => readFileSync(new URL(relative, import.meta.url), "utf8");

  it("three 仅经动态 import 进入独立 chunk", () => {
    const stageSource = readSource("./KineticStage.svelte");
    expect(stageSource).toMatch(/import\(\s*["']\.\/webgl\/KineticScene["']\s*\)/);
    expect(stageSource).not.toMatch(/from\s+["']three["']/);

    const sceneSource = readSource("./webgl/KineticScene.ts");
    expect(sceneSource).toMatch(/from\s+["']three["']/);

    // 静态入口（barrel）不得触达 WebGL 场景，避免 three 进入主 chunk。
    const indexSource = readSource("./index.ts");
    expect(indexSource).not.toContain("KineticScene");
  });

  it("设置页外观区存在电影化主视觉开关", () => {
    const settingsSource = readFileSync("src/lib/components/SettingsPage.svelte", "utf8");
    expect(settingsSource).toContain("电影化主视觉");
    expect(settingsSource).toContain("kineticStageStore");
  });
});
