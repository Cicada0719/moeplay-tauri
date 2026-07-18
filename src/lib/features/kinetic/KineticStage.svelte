<!--
  Kinetic 电影化媒体舞台 · 编排层（概念站 WebGLMediaStage 的生产硬化移植）。

  降级决策树（任何异常都不允许白屏）：
    开关关闭            → fallback（静止）      data-reason="disabled"
    reduced-motion 双信号 → fallback（静止）     data-reason="reduced-motion"
    无 WebGL2 上下文     → fallback（动画 CSS）  data-reason="no-webgl"
    上下文丢失           → fallback（动画 CSS）  data-reason="context-lost"（恢复后自动回到 WebGL）
    帧率持续过低         → 自动降档 high→medium→low，仍不足 → fallback  data-reason="perf"
    加载/初始化异常      → fallback             data-reason="error"

  three 仅经 `await import("./webgl/KineticScene")` 动态加载，落在独立 chunk。
  根节点 pointer-events: none + aria-hidden，不干扰首页焦点管理与键鼠/手柄导航。
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import KineticFallback from "./KineticFallback.svelte";
  import { createReducedMotionGuard, isReducedMotionActive } from "./reducedMotion";
  import {
    detectKineticQuality,
    KineticQualityGovernor,
    readKineticCapabilities,
  } from "./quality";
  import { readKineticPalette, watchKineticPalette } from "./palette";
  import type {
    KineticFallbackReason,
    KineticQuality,
    KineticSceneContract,
  } from "./types";

  let {
    enabled = true,
    class: className = "",
  }: {
    enabled?: boolean;
    class?: string;
  } = $props();

  type StageMode = "webgl" | "fallback";

  let mode = $state<StageMode>("fallback");
  let reason = $state<KineticFallbackReason>("init");
  let quality = $state<KineticQuality>("low");
  let canvas = $state<HTMLCanvasElement>();

  let scene: KineticSceneContract | null = null;
  let governor: KineticQualityGovernor | null = null;
  let booting = false;
  let disposed = false;

  function teardown(nextReason: KineticFallbackReason): void {
    scene?.dispose();
    scene = null;
    governor = null;
    reason = nextReason;
    mode = "fallback";
  }

  function handleFrame(deltaMs: number): void {
    if (!governor || !scene) return;
    const verdict = governor.sample(deltaMs);
    if (!verdict) return;
    if (verdict === "fallback") {
      teardown("perf");
      return;
    }
    quality = verdict;
    scene.setQuality(verdict);
  }

  async function boot(): Promise<void> {
    if (scene || booting || disposed) return;
    if (!enabled) {
      teardown("disabled");
      return;
    }
    if (isReducedMotionActive()) {
      reason = "reduced-motion";
      mode = "fallback";
      return;
    }
    const capabilities = readKineticCapabilities();
    if (!capabilities.webgl2) {
      reason = "no-webgl";
      mode = "fallback";
      return;
    }
    const tier = detectKineticQuality(capabilities);
    const target = canvas;
    if (!target) return;

    booting = true;
    let instance: KineticSceneContract | null = null;
    try {
      // three 动态加载：只有真正进入 WebGL 路径才会拉取 three chunk。
      const { createKineticScene } = await import("./webgl/KineticScene");
      if (disposed || !enabled || isReducedMotionActive() || canvas !== target) return;
      instance = createKineticScene({
        quality: tier,
        palette: readKineticPalette(),
        onFrame: handleFrame,
        onContextLost: () => {
          reason = "context-lost";
          mode = "fallback";
        },
        onContextRestored: () => {
          if (!scene) return;
          mode = "webgl";
          scene.resume();
        },
      });
      await instance.mount(target);
      if (disposed) {
        instance.dispose();
        instance = null;
        return;
      }
      scene = instance;
      governor = new KineticQualityGovernor(tier);
      quality = tier;
      mode = "webgl";
    } catch {
      instance?.dispose();
      teardown("error");
    } finally {
      booting = false;
    }
  }

  $effect(() => {
    if (enabled) void boot();
    else teardown("disabled");
  });

  onMount(() => {
    disposed = false;
    const motionGuard = createReducedMotionGuard((reduced) => {
      if (reduced) teardown("reduced-motion");
      else if (enabled) void boot();
    });
    const unwatchPalette = watchKineticPalette((palette) => scene?.setPalette(palette));
    return () => {
      disposed = true;
      motionGuard.dispose();
      unwatchPalette();
      scene?.dispose();
      scene = null;
      governor = null;
    };
  });

  onDestroy(() => {
    disposed = true;
  });
</script>

<div
  class={`kinetic-stage ${className}`.trim()}
  data-testid="kinetic-stage"
  data-mode={mode}
  data-reason={reason}
  data-quality={quality}
  aria-hidden="true"
>
  <canvas
    bind:this={canvas}
    class="kinetic-stage__canvas"
    class:kinetic-stage__canvas--hidden={mode !== "webgl"}
  ></canvas>
  {#if mode === "fallback"}
    <KineticFallback
      quality={reason === "reduced-motion" ? "reduced" : quality}
      {reason}
      animated={reason !== "disabled" && reason !== "reduced-motion"}
    />
  {/if}
</div>

<style>
  .kinetic-stage {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .kinetic-stage__canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
  }

  .kinetic-stage__canvas--hidden {
    visibility: hidden;
  }
</style>
