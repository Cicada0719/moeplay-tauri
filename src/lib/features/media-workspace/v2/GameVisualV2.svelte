<script lang="ts">
  import "../styles/media-workspace.css";
  import "../styles/game-visual.css";
  import { composeGameVisual, compareContinueCandidates, type VisualMediaSlot } from "../composition";
  import MediaArtwork from "../components/MediaArtwork.svelte";
  import { findAction, formatPlaytime, runAction, statusLabel } from "../components/viewHelpers";
  import type { MediaPresentationItem, MediaWorkspaceViewActions } from "../components/types";
  import { adjacentItem, createWheelStepper, normalizeWheelDelta, shouldCaptureStageInput, type StepDirection } from "./visualStepNavigation";

  interface Props extends MediaWorkspaceViewActions {
    items?: readonly MediaPresentationItem[];
    selectedId?: string | null;
  }

  let { items = [], selectedId = null, onAction, onImport }: Props = $props();

  const wheelStepper = createWheelStepper({ threshold: 72, cooldownMs: 450, gestureIdleMs: 180, axisLockRatio: 1.15 });
  let reducedMotion = $state(false);

  $effect(() => {
    if (typeof window === "undefined") return;
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => { reducedMotion = query.matches; };
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  });

  const composition = $derived(composeGameVisual(items, selectedId));
  const featured = $derived(composition.selectedItem);
  const navigationOrder = $derived([...items]
    .filter((item) => Boolean(findAction(item, "select")))
    .sort(compareContinueCandidates));
  const launch = $derived(featured ? findAction(featured, "launch") : undefined);
  const open = $derived(featured ? findAction(featured, "open") : undefined);
  const favorite = $derived(featured ? findAction(featured, "toggle-favorite") : undefined);

  function isInteractiveTarget(target: EventTarget | null, stage: HTMLElement): boolean {
    if (!(target instanceof Element)) return false;
    const interactive = target.closest("button, a[href], input, select, textarea, summary, [contenteditable], [role='button'], [role='link'], [role='slider'], [role='textbox'], [tabindex]");
    return interactive !== null && interactive !== stage;
  }

  function isScrollableSubregion(target: EventTarget | null, stage: HTMLElement): boolean {
    if (!(target instanceof Element)) return false;
    for (let node: Element | null = target; node && node !== stage; node = node.parentElement) {
      if (!(node instanceof HTMLElement)) continue;
      const style = getComputedStyle(node);
      const scrollsY = /(auto|scroll|overlay)/.test(style.overflowY) && node.scrollHeight > node.clientHeight + 1;
      const scrollsX = /(auto|scroll|overlay)/.test(style.overflowX) && node.scrollWidth > node.clientWidth + 1;
      if (scrollsY || scrollsX) return true;
    }
    return false;
  }

  function selectAdjacent(direction: StepDirection): void {
    if (!featured || navigationOrder.length < 2) return;
    const item = adjacentItem(navigationOrder, featured.id, direction);
    if (item && item.id !== featured.id) runAction(item, "select", onAction);
  }

  function handleWheel(event: WheelEvent): void {
    const stage = event.currentTarget as HTMLElement;
    if (!shouldCaptureStageInput({
      isInteractiveTarget: isInteractiveTarget(event.target, stage),
      isScrollableSubregion: isScrollableSubregion(event.target, stage),
    })) return;

    const viewportHeight = stage.clientHeight || window.innerHeight;
    const deltaX = normalizeWheelDelta(event.deltaX, event.deltaMode, viewportHeight);
    const deltaY = normalizeWheelDelta(event.deltaY, event.deltaMode, viewportHeight);
    if (Math.abs(deltaY) < Math.abs(deltaX) * 1.15 || deltaY === 0) return;

    event.preventDefault();
    const direction = wheelStepper.push({ deltaX, deltaY, time: performance.now() });
    if (direction) selectAdjacent(direction);
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    const stage = event.currentTarget as HTMLElement;
    if (!shouldCaptureStageInput({
      isInteractiveTarget: isInteractiveTarget(event.target, stage),
      isScrollableSubregion: isScrollableSubregion(event.target, stage),
    })) return;

    event.preventDefault();
    selectAdjacent(event.key === "ArrowDown" ? 1 : -1);
  }

  function activateSlot(slot: VisualMediaSlot): void {
    const action = slot.action;
    if (action.type === "none") return;
    const item = items.find((candidate) => candidate.id === action.itemId);
    if (!item || slot.ownerItemId !== item.id) return;
    if (action.type === "select-item") runAction(item, "select", onAction);
    else runAction(item, "open", onAction);
  }

  function slotDescription(slot: VisualMediaSlot): string {
    if (!slot.item) return `${slot.label}，暂无内容`;
    if (slot.action.type === "select-item") return `${slot.label}：切换到 ${slot.item.title}`;
    if (slot.action.type === "open-media") return `${slot.label}：查看 ${slot.item.title} 的媒体与详情`;
    return `${slot.label}：打开 ${slot.item.title}`;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="mw-v2-visual gv-stage"
  aria-labelledby="mw-v2-visual-title"
  aria-describedby="mw-v2-visual-navigation-hint"
  tabindex="0"
  data-reduced-motion={reducedMotion ? "true" : "false"}
  onwheel={handleWheel}
  onkeydown={handleKeydown}
>
  <span id="mw-v2-visual-navigation-hint" class="gv-sr-only">在舞台空白处滚动或按上下方向键可逐项切换游戏</span>

  {#if featured}
    {#if composition.backgroundAsset}
      <div class="gv-background" aria-hidden="true">
        <MediaArtwork src={composition.backgroundAsset.src} alt="" title={featured.title} eager />
      </div>
    {/if}
    <div class="gv-background-tint" aria-hidden="true"></div>
    <div class="gv-grid" aria-hidden="true"></div>

    <header class="gv-folio">
      <span>MOEPLAY / VISUAL ARCHIVE</span>
      <strong>{String(items.length).padStart(3, "0")} TITLES</strong>
    </header>

    <article class="gv-story">
      <p class="mw-v2-kicker">CURRENT ARCHIVE — {featured.metadata.releaseYear || "UNDATED"}</p>
      <h1 id="mw-v2-visual-title">{featured.title}</h1>
      <p class="gv-original">{featured.originalTitle || featured.metadata.developer || "PRIVATE GAME ARCHIVE"}</p>
      <p class="gv-summary">{featured.description || `重新进入 ${featured.title}。游玩记录、媒体、存档与收藏将在同一个作品档案中继续。`}</p>
      <dl class="gv-facts">
        <div><dt>状态</dt><dd>{statusLabel(featured.metadata.completionStatus)}</dd></div>
        <div><dt>时长</dt><dd>{formatPlaytime(featured.metadata.totalSeconds)}</dd></div>
        <div><dt>平台</dt><dd>{featured.metadata.platform || "PC"}</dd></div>
      </dl>
      <div class="gv-actions">
        {#if launch}<button class="mw-v2-action mw-v2-action--accent" onclick={() => runAction(featured, "launch", onAction)}><span>{launch.label}</span><i aria-hidden="true"></i></button>{/if}
        {#if open}<button class="mw-v2-action" onclick={() => runAction(featured, "open", onAction)}>打开作品档案</button>{/if}
        {#if favorite}<button class="mw-v2-action mw-v2-action--quiet" aria-pressed={favorite.active ?? featured.favorite} onclick={() => runAction(featured, "toggle-favorite", onAction)}>{favorite.active ?? featured.favorite ? "已收藏" : "收藏"}</button>{/if}
      </div>
    </article>

    <div class="gv-media" aria-label="作品媒体编排">
      {#each composition.slots as slot (slot.id)}
        <button
          class={`gv-slot gv-slot--${slot.role}`}
          class:gv-slot--empty={!slot.asset}
          type="button"
          disabled={slot.action.type === "none"}
          data-visual-slot={slot.role}
          data-owner-item-id={slot.ownerItemId ?? ""}
          data-action-type={slot.action.type}
          onclick={() => activateSlot(slot)}
          aria-label={slotDescription(slot)}
        >
          {#if slot.asset}
            <MediaArtwork src={slot.asset.src} alt={slot.asset.alt} title={slot.item?.title || featured.title} eager={slot.role === "lead"} />
          {:else}
            <span class="gv-placeholder" aria-hidden="true">
              <small>{slot.label.toUpperCase()}</small>
              <strong>{slot.role === "scene-a" || slot.role === "scene-b" ? "NO SCENE" : "AWAITING MEDIA"}</strong>
              <i></i>
            </span>
          {/if}
          <span class="gv-slot-shade" aria-hidden="true"></span>
          <span class="gv-slot-caption">
            <small>{slot.label}</small>
            <strong>{slot.item?.title || "媒体待补全"}</strong>
          </span>
        </button>
      {/each}
    </div>

    <nav class="mw-v2-visual__queue gv-queue" aria-label="最近游戏">
      {#each navigationOrder.slice(0, 6) as item, index (item.id)}
        <button class:active={item.id === featured.id} onclick={() => runAction(item, "select", onAction)} aria-current={item.id === featured.id ? "true" : undefined}>
          <span>{String(index + 1).padStart(2, "0")}</span>
          <strong>{item.title}</strong>
          <small>{statusLabel(item.metadata.completionStatus)}</small>
        </button>
      {/each}
    </nav>
  {:else}
    <div class="mw-v2-empty gv-empty">
      <span>ARCHIVE 000</span><h1 id="mw-v2-visual-title">建立你的第一份游戏档案</h1><p>导入游戏后，MoePlay 会以封面、场景、记录与存档重组你的私人媒体主页。</p>
      {#if onImport}<button class="mw-v2-action mw-v2-action--accent" onclick={() => void onImport?.()}><span>导入游戏</span><i aria-hidden="true"></i></button>{/if}
    </div>
  {/if}
</section>
