<script lang="ts">
  import type { DanmakuComment } from "../../stores/anime.svelte";
  import { animeStore } from "../../stores/anime.svelte";

  interface Props {
    comments: DanmakuComment[];
    currentTime: number;
    enabled: boolean;
  }

  let { comments, currentTime, enabled }: Props = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let containerEl = $state<HTMLDivElement | null>(null);

  // 从 store 读取弹幕设置
  const opacity = $derived(animeStore.danmakuOpacity);
  const speed = $derived(animeStore.danmakuSpeed);
  const fontSize = $derived(animeStore.danmakuFontSize);
  const area = $derived(animeStore.danmakuArea); // 0=1/4 1=1/2 2=全屏
  const blockScroll = $derived(animeStore.danmakuBlockScroll);
  const blockTop = $derived(animeStore.danmakuBlockTop);
  const blockBottom = $derived(animeStore.danmakuBlockBottom);
  const blockWords = $derived(animeStore.danmakuBlockWords);

  const FONT_FAMILY = '"PingFang SC", "Microsoft YaHei", "Hiragino Sans GB", sans-serif';

  // Danmaku renderer state
  interface ActiveComment {
    text: string;
    color: string;
    mode: number; // 1=scroll, 4=bottom, 5=top
    x: number;
    y: number;
    width: number;
    speed: number;
    opacity: number;
    startTime: number;
  }

  let activeComments: ActiveComment[] = [];
  let lastTime = 0;
  let canvasW = 0;
  let canvasH = 0;

  // 计算弹幕区域限制
  function getMaxY(): number {
    if (area === 0) return canvasH * 0.25; // 1/4 屏幕
    if (area === 1) return canvasH * 0.5;  // 1/2 屏幕
    return canvasH; // 全屏
  }

  // 检查弹幕是否应该被屏蔽
  function shouldBlock(comment: DanmakuComment): boolean {
    // 按模式屏蔽
    if (blockScroll && comment.mode === 1) return true;
    if (blockTop && comment.mode === 5) return true;
    if (blockBottom && comment.mode === 4) return true;
    // 按关键词屏蔽
    if (blockWords.length > 0 && blockWords.some(w => w.trim() && comment.text.includes(w.trim()))) return true;
    return false;
  }

  function getLane(mode: number, totalLanes: number, laneHeight: number): number {
    const isScroll = mode === 1;
    const isTop = mode === 5;
    const isBottom = mode === 4;

    for (let lane = 0; lane < totalLanes; lane++) {
      const occupied = activeComments.some(c => {
        if (isScroll && c.mode === 1) {
          return c.y === lane * laneHeight && c.x + c.width > canvasW * 0.3;
        }
        if (isTop && c.mode === 5) {
          return c.y === lane * laneHeight;
        }
        if (isBottom && c.mode === 4) {
          return c.y === canvasH - (lane + 1) * laneHeight;
        }
        return false;
      });
      if (!occupied) return lane;
    }
    return -1;
  }

  function spawnComments(time: number) {
    const tolerance = 0.3;
    const laneHeight = fontSize + 6;
    const scrollDuration = 8 / speed;
    for (const c of comments) {
      if (c.time >= lastTime - tolerance && c.time < time + tolerance) {
        // 屏蔽检查
        if (shouldBlock(c)) continue;

        const alreadyActive = activeComments.some(
          ac => ac.text === c.text && Math.abs(ac.startTime - c.time) < 0.5
        );
        if (alreadyActive) continue;

        const r = (c.color >> 16) & 0xFF;
        const g = (c.color >> 8) & 0xFF;
        const b = c.color & 0xFF;
        const colorStr = `rgb(${r},${g},${b})`;

        const ctx = canvasEl?.getContext('2d');
        if (!ctx) continue;
        ctx.font = `${fontSize}px ${FONT_FAMILY}`;
        const textWidth = ctx.measureText(c.text).width;

        const maxY = getMaxY();
        const totalLanes = Math.max(1, Math.floor(maxY / laneHeight) - 1);
        const lane = getLane(c.mode, totalLanes, laneHeight);
        if (lane < 0) continue;

        let x: number;
        let y: number;
        let spd: number;

        if (c.mode === 1) {
          x = canvasW;
          y = lane * laneHeight;
          spd = (canvasW + textWidth) / scrollDuration;
        } else if (c.mode === 5) {
          x = (canvasW - textWidth) / 2;
          y = lane * laneHeight;
          spd = 0;
        } else {
          x = (canvasW - textWidth) / 2;
          y = canvasH - (lane + 1) * laneHeight;
          spd = 0;
        }

        activeComments.push({
          text: c.text,
          color: colorStr,
          mode: c.mode,
          x,
          y: y + fontSize,
          width: textWidth,
          speed: spd,
          opacity: opacity,
          startTime: c.time,
        });
      }
    }
  }

  function render(time: number) {
    const canvas = canvasEl;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const parent = canvas.parentElement;
    if (parent) {
      const dpr = window.devicePixelRatio || 1;
      const w = parent.clientWidth;
      const h = parent.clientHeight;
      if (w !== canvasW || h !== canvasH) {
        canvasW = w;
        canvasH = h;
        canvas.width = w * dpr;
        canvas.height = h * dpr;
        canvas.style.width = `${w}px`;
        canvas.style.height = `${h}px`;
        ctx.scale(dpr, dpr);
      }
    }

    ctx.clearRect(0, 0, canvasW, canvasH);

    if (!enabled || comments.length === 0) {
      lastTime = time;
      return;
    }

    spawnComments(time);

    ctx.font = `${fontSize}px ${FONT_FAMILY}`;
    ctx.textBaseline = 'top';

    const dt = time - lastTime;
    activeComments = activeComments.filter(c => {
      if (c.mode !== 1 && time - c.startTime > 4) return false;
      if (c.mode === 1 && c.x + c.width < -10) return false;

      if (c.mode === 1) {
        c.x -= c.speed * dt;
      }

      ctx.fillStyle = 'rgba(0,0,0,0.5)';
      ctx.fillText(c.text, c.x + 1, c.y + 1);

      ctx.fillStyle = c.color;
      ctx.globalAlpha = opacity; // 用当前 store 值，调滑块时即时生效
      ctx.fillText(c.text, c.x, c.y);
      ctx.globalAlpha = 1;

      return true;
    });

    lastTime = time;
  }

  $effect(() => {
    const time = currentTime;
    if (!canvasEl) return;
    const id = requestAnimationFrame(() => render(time));
    return () => cancelAnimationFrame(id);
  });

  $effect(() => {
    const _ = comments;
    activeComments = [];
    lastTime = 0;
  });
</script>

<div class="danmaku-container" bind:this={containerEl}>
  <canvas bind:this={canvasEl}></canvas>
</div>

<style>
  .danmaku-container {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 5;
    overflow: hidden;
  }
  .danmaku-container canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
