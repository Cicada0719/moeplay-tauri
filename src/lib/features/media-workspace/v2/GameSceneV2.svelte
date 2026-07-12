<script lang="ts">
  import "../styles/media-workspace.css";
  import "../styles/game-scene.css";
  import { compareContinueCandidates, dedupePresentationItems } from "../composition";
  import MediaArtwork from "../components/MediaArtwork.svelte";
  import { findAction, formatPlaytime, runAction, statusLabel } from "../components/viewHelpers";
  import type { MediaPresentationItem, MediaWorkspaceViewActions, PresentationAsset } from "../components/types";
  import { adjacentIndex, createWheelStepper, normalizeWheelDelta, type StepDirection } from "./visualStepNavigation";

  interface Props extends MediaWorkspaceViewActions {
    items?: readonly MediaPresentationItem[];
    selectedId?: string | null;
  }

  let { items = [], selectedId = null, onAction, onImport }: Props = $props();

  const wheelStepper = createWheelStepper({ threshold: 58, cooldownMs: 420, gestureIdleMs: 170, axisLockRatio: 1.05 });
  let reducedMotion = $state(false);
  let pointerStartY = $state<number | null>(null);
  let pointerId = $state<number | null>(null);

  const games = $derived.by(() => dedupePresentationItems(items)
    .filter((item) => Boolean(findAction(item, "select")))
    .sort(compareContinueCandidates));
  const activeIndex = $derived.by(() => {
    const index = games.findIndex((item) => item.id === selectedId);
    return index >= 0 ? index : 0;
  });
  const active = $derived(games[activeIndex] ?? null);
  const launch = $derived(active ? findAction(active, "launch") : undefined);
  const open = $derived(active ? findAction(active, "open") : undefined);

  $effect(() => {
    if (typeof window === "undefined") return;
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => { reducedMotion = query.matches; };
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  });

  function preferredAsset(item: MediaPresentationItem): PresentationAsset | null {
    return item.hero ?? item.screenshots[0] ?? item.cover ?? item.media[0] ?? null;
  }

  function circularOffset(index: number): number {
    const length = games.length;
    if (length <= 1) return 0;
    let offset = index - activeIndex;
    if (offset > length / 2) offset -= length;
    if (offset < -length / 2) offset += length;
    return offset;
  }

  function selectIndex(index: number) {
    const item = games[index];
    if (!item || item.id === active?.id) return;
    runAction(item, "select", onAction);
  }

  function step(direction: StepDirection) {
    if (games.length < 2) return;
    selectIndex(adjacentIndex(games.length, activeIndex, direction));
  }

  function handleWheel(event: WheelEvent) {
    const target = event.target;
    if (target instanceof Element && target.closest("input,textarea,select,[contenteditable='true']")) return;
    const height = (event.currentTarget as HTMLElement).clientHeight || window.innerHeight;
    const deltaX = normalizeWheelDelta(event.deltaX, event.deltaMode, height);
    const deltaY = normalizeWheelDelta(event.deltaY, event.deltaMode, height);
    if (Math.abs(deltaY) < Math.abs(deltaX) * 1.05 || deltaY === 0) return;
    event.preventDefault();
    const direction = wheelStepper.push({ deltaX, deltaY, time: performance.now() });
    if (direction) step(direction);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      event.preventDefault(); step(1);
    } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      event.preventDefault(); step(-1);
    } else if (event.key === "Enter" && active) {
      event.preventDefault(); runAction(active, "open", onAction);
    }
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.pointerType === "mouse" && event.button !== 0) return;
    pointerStartY = event.clientY;
    pointerId = event.pointerId;
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  }

  function handlePointerUp(event: PointerEvent) {
    if (pointerId !== event.pointerId || pointerStartY === null) return;
    const delta = event.clientY - pointerStartY;
    pointerStartY = null;
    pointerId = null;
    if (Math.abs(delta) >= 42) step(delta < 0 ? 1 : -1);
  }

  function activateCard(item: MediaPresentationItem, index: number) {
    if (index === activeIndex) runAction(item, "open", onAction);
    else selectIndex(index);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="fs-stage"
  data-testid="game-film-sequence"
  data-active-owner={active?.id ?? ""}
  data-reduced-motion={reducedMotion}
  role="application"
  tabindex="0"
  aria-labelledby="fs-title"
  onwheel={handleWheel}
  onkeydown={handleKeydown}
  onpointerdown={handlePointerDown}
  onpointerup={handlePointerUp}
  onpointercancel={() => { pointerStartY = null; pointerId = null; }}
>
  {#if active}
    <header class="fs-header">
      <div><span>MOEPLAY / FILM SEQUENCE</span><strong id="fs-title">游戏影像序列</strong></div>
      <div><span>{String(activeIndex + 1).padStart(3, "0")} / {String(games.length).padStart(3, "0")}</span><span>SCROLL / DRAG / ARROWS</span></div>
    </header>

    <div class="fs-slash fs-slash--a" aria-hidden="true"></div>
    <div class="fs-slash fs-slash--b" aria-hidden="true"></div>

    <div class="fs-film" aria-label="游戏胶片目录">
      {#each games as item, index (item.id)}
        {@const offset = circularOffset(index)}
        {@const asset = preferredAsset(item)}
        <article
          class="fs-frame"
          class:active={index === activeIndex}
          class:near={Math.abs(offset) === 1}
          class:hidden={Math.abs(offset) > 3}
          style={`--offset:${offset};--distance:${Math.abs(offset)};--direction:${offset < 0 ? -1 : 1}`}
          data-film-game={item.id}
          data-film-index={index}
          aria-hidden={Math.abs(offset) > 2 ? "true" : undefined}
        >
          <button type="button" tabindex={index === activeIndex ? 0 : -1} onclick={() => activateCard(item, index)} aria-label={`${index === activeIndex ? "打开" : "切换到"} ${item.title}`}>
            <span class="fs-frame-number">{String(index + 1).padStart(3, "0")}</span>
            <span class="fs-image">
              {#if asset}<MediaArtwork src={asset.src} alt={asset.alt} title={item.title} eager={Math.abs(offset) <= 1} />{:else}<i>{item.title.slice(0, 1)}</i>{/if}
              <span aria-hidden="true"></span>
            </span>
            <span class="fs-caption"><strong>{item.title}</strong><small>{statusLabel(item.metadata.completionStatus)} / {formatPlaytime(item.metadata.totalSeconds)}</small></span>
          </button>
        </article>
      {/each}
    </div>

    <nav class="fs-progress" aria-label="游戏序列位置">
      {#each games as item, index (item.id)}
        <button type="button" class:active={index === activeIndex} onclick={() => selectIndex(index)} aria-label={`前往 ${item.title}`}><span></span></button>
      {/each}
    </nav>

    <footer class="fs-footer">
      <div class="fs-current-copy">
        <div class="fs-current-register"><span>CURRENT FRAME</span>{#if active.metadata.rating}<strong>{active.metadata.rating.toFixed(1)} / 10</strong>{/if}</div>
        <h2>{active.title}</h2>
        <p>{active.description || "每款游戏只占据一个镜头，滚轮推动整条影像序列。"}</p>
        <div class="fs-current-meta" aria-label="游戏档案摘要">
          <span>{active.metadata.releaseYear || "----"}</span><span>{active.metadata.developer || active.metadata.publisher || "PRIVATE ARCHIVE"}</span><span>{statusLabel(active.metadata.completionStatus)}</span>
          {#each active.metadata.tags.slice(0, 2) as tag}<span>{tag}</span>{/each}
        </div>
      </div>
      <div class="fs-actions">
        {#if launch}<button class="primary" type="button" onclick={() => runAction(active, "launch", onAction)}>{launch.label}</button>{/if}
        {#if open}<button type="button" onclick={() => runAction(active, "open", onAction)}>打开档案</button>{/if}
      </div>
    </footer>
  {:else}
    <div class="mw-v2-empty fs-empty"><span>FILM 000</span><h1 id="fs-title">还没有游戏影像</h1><p>导入游戏后，每款作品会在这里形成一个独立镜头。</p>{#if onImport}<button class="mw-v2-action mw-v2-action--accent" onclick={() => void onImport?.()}><span>导入游戏</span><i aria-hidden="true"></i></button>{/if}</div>
  {/if}
</section>
