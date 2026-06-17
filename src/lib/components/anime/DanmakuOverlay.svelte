<script lang="ts">
  import type { DanmakuComment } from "../../stores/anime.svelte";
  import { animeStore } from "../../stores/anime.svelte";
  import { onDestroy } from "svelte";

  interface Props {
    comments: DanmakuComment[];
    currentTime: number;
    enabled: boolean;
  }

  let { comments, currentTime, enabled }: Props = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let containerEl = $state<HTMLDivElement | null>(null);

  const opacity = $derived(animeStore.danmakuOpacity);
  const speed = $derived(animeStore.danmakuSpeed);
  const fontSize = $derived(animeStore.danmakuFontSize);
  const area = $derived(animeStore.danmakuArea);
  const blockScroll = $derived(animeStore.danmakuBlockScroll);
  const blockTop = $derived(animeStore.danmakuBlockTop);
  const blockBottom = $derived(animeStore.danmakuBlockBottom);
  const blockWords = $derived(animeStore.danmakuBlockWords);

  const FONT_FAMILY = '"PingFang SC", "Microsoft YaHei", "Hiragino Sans GB", sans-serif';

  interface ActiveComment {
    text: string;
    color: string;
    mode: number;
    x: number;
    y: number;
    width: number;
    speed: number;
    startTime: number;
  }

  let activeComments: ActiveComment[] = [];
  let lastSpawnTime = 0;
  let canvasW = 0;
  let canvasH = 0;
  let rafId: number | null = null;
  let lastFrameTs = 0;

  function getMaxY(): number {
    if (area === 0) return canvasH * 0.25;
    if (area === 1) return canvasH * 0.5;
    return canvasH;
  }

  function shouldBlock(comment: DanmakuComment): boolean {
    if (blockScroll && comment.mode === 1) return true;
    if (blockTop && comment.mode === 5) return true;
    if (blockBottom && comment.mode === 4) return true;
    if (blockWords.length > 0 && blockWords.some(w => w.trim() && comment.text.includes(w.trim()))) return true;
    return false;
  }

  function getLane(mode: number, totalLanes: number, laneHeight: number): number {
    for (let lane = 0; lane < totalLanes; lane++) {
      const occupied = activeComments.some(c => {
        if (mode === 1 && c.mode === 1) {
          return c.y === lane * laneHeight + fontSize && c.x + c.width > canvasW * 0.3;
        }
        if (mode === 5 && c.mode === 5) {
          return c.y === lane * laneHeight + fontSize;
        }
        if (mode === 4 && c.mode === 4) {
          return c.y === canvasH - (lane + 1) * laneHeight + fontSize;
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
      if (c.time >= lastSpawnTime - tolerance && c.time < time + tolerance) {
        if (shouldBlock(c)) continue;

        const alreadyActive = activeComments.some(
          ac => ac.text === c.text && Math.abs(ac.startTime - c.time) < 0.5
        );
        if (alreadyActive) continue;

        const ctx = canvasEl?.getContext('2d');
        if (!ctx) continue;
        ctx.font = `${fontSize}px ${FONT_FAMILY}`;
        const textWidth = ctx.measureText(c.text).width;

        const maxY = getMaxY();
        const totalLanes = Math.max(1, Math.floor(maxY / laneHeight) - 1);
        const lane = getLane(c.mode, totalLanes, laneHeight);
        if (lane < 0) continue;

        const r = (c.color >> 16) & 0xFF;
        const g = (c.color >> 8) & 0xFF;
        const b = c.color & 0xFF;
        const colorStr = `rgb(${r},${g},${b})`;

        let x: number;
        let y: number;
        let spd: number;

        if (c.mode === 1) {
          x = canvasW;
          y = lane * laneHeight + fontSize;
          spd = (canvasW + textWidth) / scrollDuration;
        } else if (c.mode === 5) {
          x = (canvasW - textWidth) / 2;
          y = lane * laneHeight + fontSize;
          spd = 0;
        } else {
          x = (canvasW - textWidth) / 2;
          y = canvasH - (lane + 1) * laneHeight + fontSize;
          spd = 0;
        }

        activeComments.push({
          text: c.text,
          color: colorStr,
          mode: c.mode,
          x, y, width: textWidth,
          speed: spd,
          startTime: c.time,
        });
      }
    }
  }

  function frame(now: number) {
    rafId = requestAnimationFrame(frame);

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
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      }
    }

    ctx.clearRect(0, 0, canvasW, canvasH);

    if (!enabled || comments.length === 0) {
      lastSpawnTime = currentTime;
      lastFrameTs = now;
      return;
    }

    spawnComments(currentTime);
    lastSpawnTime = currentTime;

    const dtSec = lastFrameTs > 0 ? (now - lastFrameTs) / 1000 : 0;
    lastFrameTs = now;
    const clampedDt = Math.min(dtSec, 0.1);

    ctx.font = `${fontSize}px ${FONT_FAMILY}`;
    ctx.textBaseline = 'top';

    activeComments = activeComments.filter(c => {
      if (c.mode !== 1 && currentTime - c.startTime > 4) return false;
      if (c.mode === 1 && c.x + c.width < -10) return false;

      if (c.mode === 1) {
        c.x -= c.speed * clampedDt;
      }

      ctx.fillStyle = 'rgba(0,0,0,0.5)';
      ctx.fillText(c.text, c.x + 1, c.y + 1);

      ctx.fillStyle = c.color;
      ctx.globalAlpha = opacity;
      ctx.fillText(c.text, c.x, c.y);
      ctx.globalAlpha = 1;

      return true;
    });
  }

  function startLoop() {
    if (rafId !== null) return;
    lastFrameTs = 0;
    rafId = requestAnimationFrame(frame);
  }

  function stopLoop() {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  }

  $effect(() => {
    if (enabled && comments.length > 0) {
      startLoop();
    } else {
      stopLoop();
      if (canvasEl) {
        const ctx = canvasEl.getContext('2d');
        if (ctx) ctx.clearRect(0, 0, canvasW, canvasH);
      }
    }
    return stopLoop;
  });

  $effect(() => {
    const _ = comments;
    activeComments = [];
    lastSpawnTime = 0;
    lastFrameTs = 0;
  });

  onDestroy(stopLoop);
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
