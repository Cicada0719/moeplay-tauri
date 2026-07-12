<script lang="ts">
  import { onMount, tick } from "svelte";
  import { gsap } from "gsap";
  import "../styles/media-workspace.css";
  import "../styles/game-scene.css";
  import MediaArtwork from "../components/MediaArtwork.svelte";
  import { composeGameScene } from "../composition";
  import {
    adjacentGameIndex,
    adjacentMediaIndex,
    createWheelGestureTracker,
    dragSceneStep,
    wrapSceneIndex,
  } from "../input/sceneNavigation";
  import { findAction, runAction, statusLabel } from "../components/viewHelpers";
  import type { MediaPresentationItem, MediaWorkspaceViewActions } from "../components/types";

  interface Props extends MediaWorkspaceViewActions {
    items?: readonly MediaPresentationItem[];
    selectedId?: string | null;
  }

  let { items = [], selectedId = null, onAction, onImport }: Props = $props();

  const composition = $derived(composeGameScene(items, selectedId, 12));
  const entries = $derived(composition.entries);
  let activeIndex = $state(0);
  const normalizedIndex = $derived(wrapSceneIndex(activeIndex, entries.length));
  const activeEntry = $derived(entries[normalizedIndex] ?? null);
  const activeItem = $derived(activeEntry?.item ?? items.find((item) => item.id === selectedId) ?? null);
  const launch = $derived(activeItem ? findAction(activeItem, "launch") : undefined);
  const ownerFrameIndexes = $derived(
    activeEntry
      ? entries.reduce<number[]>((result, entry, index) => {
          if (entry.ownerItemId === activeEntry.ownerItemId) result.push(index);
          return result;
        }, [])
      : [],
  );
  const ownerFrameNumber = $derived(Math.max(0, ownerFrameIndexes.indexOf(normalizedIndex)) + 1);

  let root = $state<HTMLElement>(null!);
  let viewport = $state<HTMLElement>(null!);
  let track = $state<HTMLElement>(null!);
  let mounted = false;
  let reducedMotion = false;
  let resizeObserver: ResizeObserver | null = null;
  let alignRaf = 0;
  let tween: gsap.core.Tween | null = null;
  let lastCompositionKey = "";
  let suppressNextClick = false;
  const wheelGesture = createWheelGestureTracker();

  const drag = {
    active: false,
    pointerId: -1,
    startX: 0,
    lastX: 0,
    startTime: 0,
    lastTime: 0,
    baseX: 0,
    moved: false,
  };

  function entryElements(): HTMLElement[] {
    return track ? Array.from(track.querySelectorAll<HTMLElement>("[data-scene-entry]")) : [];
  }

  function targetTrackX(index = normalizedIndex): number {
    const slide = entryElements()[wrapSceneIndex(index, entries.length)];
    if (!slide || !viewport) return 0;
    return viewport.clientWidth / 2 - (slide.offsetLeft + slide.offsetWidth / 2);
  }

  function stopTween(): void {
    tween?.kill();
    tween = null;
    if (track) gsap.killTweensOf(track);
  }

  function alignTrack(immediate = reducedMotion): void {
    if (!mounted || !track || !viewport || entries.length === 0) return;
    const x = targetTrackX();
    stopTween();
    if (immediate) {
      gsap.set(track, { x });
      return;
    }
    tween = gsap.to(track, {
      x,
      duration: 0.68,
      ease: "power4.out",
      overwrite: true,
      onComplete: () => { tween = null; },
    });
  }

  function scheduleAlign(immediate = false): void {
    if (!mounted) return;
    cancelAnimationFrame(alignRaf);
    alignRaf = requestAnimationFrame(() => {
      alignRaf = 0;
      alignTrack(immediate || reducedMotion);
    });
  }

  function setActive(index: number, immediate = false): void {
    if (!entries.length) return;
    activeIndex = wrapSceneIndex(index, entries.length);
    scheduleAlign(immediate);
  }

  function stepStream(direction: -1 | 1): void {
    setActive(normalizedIndex + direction);
  }

  function stepMedia(direction: -1 | 1): void {
    setActive(adjacentMediaIndex(entries, normalizedIndex, direction));
  }

  function stepGame(direction: -1 | 1): void {
    const nextIndex = adjacentGameIndex(entries, normalizedIndex, direction);
    setActive(nextIndex);
    const item = entries[nextIndex]?.item;
    if (item && item.id !== selectedId) runAction(item, "select", onAction);
  }

  function openActive(): void {
    if (activeItem) runAction(activeItem, "open", onAction);
  }

  function handleWheel(event: WheelEvent): void {
    if (!entries.length || event.ctrlKey) return;
    event.preventDefault();
    const direction = wheelGesture.push(event.deltaX, event.deltaY, event.timeStamp);
    if (direction) stepStream(direction);
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    switch (event.key) {
      case "ArrowLeft":
        event.preventDefault();
        stepMedia(-1);
        break;
      case "ArrowRight":
        event.preventDefault();
        stepMedia(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        stepGame(-1);
        break;
      case "ArrowDown":
        event.preventDefault();
        stepGame(1);
        break;
      case "Enter":
        event.preventDefault();
        openActive();
        break;
    }
  }

  function handlePointerDown(event: PointerEvent): void {
    if (event.button !== 0 || entries.length <= 1 || !track) return;
    drag.active = true;
    drag.pointerId = event.pointerId;
    drag.startX = drag.lastX = event.clientX;
    drag.startTime = drag.lastTime = event.timeStamp;
    drag.baseX = Number(gsap.getProperty(track, "x")) || targetTrackX();
    drag.moved = false;
    stopTween();
    viewport.setPointerCapture(event.pointerId);
    root.dataset.dragging = "true";
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!drag.active || event.pointerId !== drag.pointerId || !track) return;
    const distance = event.clientX - drag.startX;
    drag.lastX = event.clientX;
    drag.lastTime = event.timeStamp;
    drag.moved ||= Math.abs(distance) > 5;
    if (drag.moved) event.preventDefault();
    gsap.set(track, { x: drag.baseX + distance });
  }

  function finishDrag(event: PointerEvent, cancelled = false): void {
    if (!drag.active || event.pointerId !== drag.pointerId) return;
    const distance = drag.lastX - drag.startX;
    const elapsed = Math.max(1, drag.lastTime - drag.startTime);
    const velocity = distance / elapsed;
    const step = cancelled ? 0 : dragSceneStep(distance, velocity, viewport.clientWidth);
    suppressNextClick = drag.moved;
    drag.active = false;
    drag.pointerId = -1;
    root.dataset.dragging = "false";
    if (viewport.hasPointerCapture(event.pointerId)) viewport.releasePointerCapture(event.pointerId);
    if (step) stepStream(step);
    else scheduleAlign();
  }

  function handleEntryClick(event: MouseEvent, index: number): void {
    if (suppressNextClick) {
      suppressNextClick = false;
      event.preventDefault();
      return;
    }
    setActive(index);
  }

  $effect(() => {
    const key = `${composition.selectedItemId ?? "none"}:${entries.map((entry) => entry.id).join("|")}`;
    if (key !== lastCompositionKey) {
      lastCompositionKey = key;
      activeIndex = wrapSceneIndex(composition.activeIndex, entries.length);
      void tick().then(() => scheduleAlign(true));
    }
  });

  $effect(() => {
    normalizedIndex;
    if (mounted && !drag.active) void tick().then(() => scheduleAlign());
  });

  onMount(() => {
    mounted = true;
    const motionQuery = matchMedia("(prefers-reduced-motion: reduce)");
    const syncMotion = () => {
      reducedMotion = motionQuery.matches;
      scheduleAlign(true);
    };
    syncMotion();
    motionQuery.addEventListener("change", syncMotion);
    resizeObserver = new ResizeObserver(() => scheduleAlign(true));
    resizeObserver.observe(viewport);
    void tick().then(() => scheduleAlign(true));

    return () => {
      mounted = false;
      motionQuery.removeEventListener("change", syncMotion);
      resizeObserver?.disconnect();
      resizeObserver = null;
      cancelAnimationFrame(alignRaf);
      alignRaf = 0;
      stopTween();
      wheelGesture.reset();
      if (track) gsap.set(track, { clearProps: "transform" });
    };
  });
</script>

<section
  bind:this={root}
  class="mw-v2-scene mw-v2-scene--flow"
  aria-labelledby="mw-v2-scene-title"
  aria-describedby="mw-v2-scene-instructions"
  data-active-index={normalizedIndex}
  data-active-owner={activeEntry?.ownerItemId ?? ""}
  data-dragging="false"
>
  <header class="mw-v2-scene__header mw-v2-scene__header--overlay">
    <div>
      <p class="mw-v2-kicker">SCENE DIRECTORY / CONTINUOUS MEMORY</p>
      <h1 id="mw-v2-scene-title">场景流</h1>
    </div>
    {#if activeItem}
      <div class="mw-v2-scene__focus-readout" aria-live="polite">
        <span>FOCUS / {String(normalizedIndex + 1).padStart(2, "0")}</span>
        <strong>{activeItem.title}</strong>
        <small>{statusLabel(activeItem.metadata.completionStatus)} · FRAME {String(ownerFrameNumber).padStart(2, "0")}/{String(ownerFrameIndexes.length).padStart(2, "0")}</small>
      </div>
    {/if}
  </header>

  {#if entries.length}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      bind:this={viewport}
      class="mw-v2-scene__viewport"
      role="application"
      aria-label="连续媒体场景流"
      aria-describedby="mw-v2-scene-instructions"
      tabindex="0"
      onkeydown={handleKeydown}
      onwheel={handleWheel}
      onpointerdown={handlePointerDown}
      onpointermove={handlePointerMove}
      onpointerup={(event) => finishDrag(event)}
      onpointercancel={(event) => finishDrag(event, true)}
    >
      <div bind:this={track} class="mw-v2-scene__track">
        {#each entries as entry, index (entry.id)}
          <article
            class="mw-v2-scene__item mw-v2-scene__flow-item"
            class:active={index === normalizedIndex}
            class:near={Math.abs(index - normalizedIndex) === 1}
            data-scene-entry
            data-role={entry.role}
            data-owner={entry.ownerItemId ?? ""}
            aria-hidden={Math.abs(index - normalizedIndex) > 2 ? "true" : undefined}
          >
            <button
              type="button"
              onclick={(event) => handleEntryClick(event, index)}
              ondblclick={() => runAction(entry.item!, "open", onAction)}
              aria-label={`${index === normalizedIndex ? "当前场景" : "切换到"} ${entry.item?.title ?? "未命名游戏"}，第 ${index + 1} 帧`}
              aria-current={index === normalizedIndex ? "true" : undefined}
              tabindex={index === normalizedIndex ? 0 : -1}
            >
              <MediaArtwork
                src={entry.asset?.src}
                alt={entry.asset?.alt || ""}
                title={entry.item?.title ?? "未命名游戏"}
                eager={index < 3}
              />
              <span class="mw-v2-scene__shade" aria-hidden="true"></span>
              <span class="mw-v2-scene__index">{String(index + 1).padStart(2, "0")}</span>
              <span class="mw-v2-scene__caption">
                <strong>{entry.item?.title}</strong>
                <small>{entry.asset?.role || "archive"} / {entry.role}</small>
              </span>
            </button>
          </article>
        {/each}
      </div>

      <div class="mw-v2-scene__focus-frame" aria-hidden="true">
        <i></i><i></i><i></i><i></i>
      </div>

      <nav class="mw-v2-scene__map" aria-label="场景缩略图地图">
        <span>MAP / {String(entries.length).padStart(2, "0")}</span>
        <div>
          {#each entries as entry, index (entry.id)}
            <button
              type="button"
              class:active={index === normalizedIndex}
              onclick={() => setActive(index)}
              aria-label={`前往 ${entry.item?.title ?? "未命名游戏"} 第 ${index + 1} 帧`}
              aria-current={index === normalizedIndex ? "true" : undefined}
            >
              {#if entry.asset?.src}<img src={entry.asset.src} alt="" loading="lazy" draggable="false" />{:else}<span>{String(index + 1).padStart(2, "0")}</span>{/if}
              <i aria-hidden="true"></i>
            </button>
          {/each}
        </div>
      </nav>
    </div>

    {#if activeItem}
      <footer class="mw-v2-scene__footer mw-v2-scene__footer--overlay">
        <p>{activeItem.description || "从媒体切片、游玩状态与档案信息重新进入这部作品。"}</p>
        {#if launch}<button class="mw-v2-action mw-v2-action--accent" onclick={() => runAction(activeItem, "launch", onAction)}><span>{launch.label}</span><i aria-hidden="true"></i></button>{/if}
        <button class="mw-v2-action" onclick={openActive}>查看详情</button>
      </footer>
    {/if}

    <p id="mw-v2-scene-instructions" class="mw-v2-scene__instructions">
      滚轮或拖拽逐帧浏览；左右键切换当前游戏媒体，上下键切换游戏，Enter 打开详情。
    </p>
  {:else}
    <div class="mw-v2-empty"><span>SCENE 000</span><h1 id="mw-v2-scene-title">还没有可编排的场景</h1><p>补充封面与截图后，这里会形成连续、可滚动的媒体目录。</p>{#if onImport}<button class="mw-v2-action mw-v2-action--accent" onclick={() => void onImport?.()}><span>导入游戏</span><i aria-hidden="true"></i></button>{/if}</div>
  {/if}
</section>
