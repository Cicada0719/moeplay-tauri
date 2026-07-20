<script lang="ts">
  import "../styles/media-workspace.css";
  import "../styles/game-visual.css";
  import { compareContinueCandidates, dedupePresentationItems, normalizeMediaIdentity, presentationIdentity } from "../composition";
  import MediaArtwork from "../components/MediaArtwork.svelte";
  import { findAction, formatPlaytime, runAction, statusLabel } from "../components/viewHelpers";
  import type { MediaPresentationItem, MediaWorkspaceViewActions, PresentationAsset } from "../components/types";
  import { adjacentItem, createWheelStepper, normalizeWheelDelta, type StepDirection } from "./visualStepNavigation";

  interface Props extends MediaWorkspaceViewActions {
    items?: readonly MediaPresentationItem[];
    selectedId?: string | null;
  }

  let { items = [], selectedId = null, onAction, onImport }: Props = $props();

  const wheelStepper = createWheelStepper({ threshold: 64, cooldownMs: 420, gestureIdleMs: 170, axisLockRatio: 1.1 });
  let activeMediaIndex = $state(0);
  let folded = $state(false);
  let shifting = $state(false);
  let shiftDirection = $state<StepDirection>(1);
  let reducedMotion = $state(false);
  let shiftTimer: ReturnType<typeof setTimeout> | null = null;

  const uniqueItems = $derived.by(() => dedupePresentationItems(items)
    .filter((item) => Boolean(findAction(item, "select")))
    .sort(compareContinueCandidates));
  const requestedItem = $derived(items.find((item) => item.id === selectedId) ?? null);
  const featured = $derived(
    uniqueItems.find((item) => item.id === selectedId)
      ?? uniqueItems.find((item) => requestedItem && presentationIdentity(item) === presentationIdentity(requestedItem))
      ?? uniqueItems[0]
      ?? null,
  );
  const activeGameIndex = $derived(featured ? Math.max(0, uniqueItems.findIndex((item) => item.id === featured.id)) : 0);
  const mediaAssets = $derived.by(() => {
    if (!featured) return [] as PresentationAsset[];
    const seen = new Set<string>();
    return [featured.hero, ...featured.screenshots, featured.cover].filter((asset): asset is PresentationAsset => {
      if (!asset?.src) return false;
      const identity = normalizeMediaIdentity(asset.src);
      if (!identity || seen.has(identity)) return false;
      seen.add(identity);
      return true;
    }).slice(0, 6);
  });
  const activeAsset = $derived(mediaAssets[activeMediaIndex] ?? mediaAssets[0] ?? null);
  const launch = $derived(featured ? findAction(featured, "launch") : undefined);
  const open = $derived(featured ? findAction(featured, "open") : undefined);
  const favorite = $derived(featured ? findAction(featured, "toggle-favorite") : undefined);
  const directoryItems = $derived.by(() => {
    if (uniqueItems.length <= 9) return uniqueItems.map((item, index) => ({ item, index }));
    const start = Math.max(0, Math.min(activeGameIndex - 4, uniqueItems.length - 9));
    return uniqueItems.slice(start, start + 9).map((item, offset) => ({ item, index: start + offset }));
  });

  $effect(() => {
    featured?.id;
    activeMediaIndex = 0;
  });

  $effect(() => {
    if (typeof window === "undefined") return;
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => { reducedMotion = query.matches; };
    update();
    query.addEventListener("change", update);
    return () => {
      query.removeEventListener("change", update);
      if (shiftTimer) clearTimeout(shiftTimer);
    };
  });

  function beginShift(direction: StepDirection) {
    shiftDirection = direction;
    shifting = true;
    if (shiftTimer) clearTimeout(shiftTimer);
    shiftTimer = setTimeout(() => { shifting = false; }, reducedMotion ? 0 : 480);
  }

  function selectGame(item: MediaPresentationItem, direction?: StepDirection) {
    if (item.id === featured?.id) return;
    const inferred = direction ?? (uniqueItems.findIndex((candidate) => candidate.id === item.id) >= activeGameIndex ? 1 : -1);
    beginShift(inferred);
    runAction(item, "select", onAction);
  }

  function activateGame(item: MediaPresentationItem) {
    if (item.id === featured?.id) {
      runAction(item, "open", onAction);
      return;
    }
    selectGame(item);
  }

  function selectAdjacent(direction: StepDirection) {
    if (!featured || uniqueItems.length < 2) return;
    const item = adjacentItem(uniqueItems, featured.id, direction);
    if (item) selectGame(item, direction);
  }

  function selectAdjacentMedia(direction: StepDirection) {
    if (mediaAssets.length < 2) return;
    activeMediaIndex = (activeMediaIndex + direction + mediaAssets.length) % mediaAssets.length;
  }

  function isTypingTarget(target: EventTarget | null) {
    return target instanceof Element && Boolean(target.closest("input, textarea, select, [contenteditable='true']"));
  }

  function handleWheel(event: WheelEvent) {
    if (isTypingTarget(event.target)) return;
    const viewportHeight = (event.currentTarget as HTMLElement).clientHeight || window.innerHeight;
    const deltaX = normalizeWheelDelta(event.deltaX, event.deltaMode, viewportHeight);
    const deltaY = normalizeWheelDelta(event.deltaY, event.deltaMode, viewportHeight);
    if (Math.abs(deltaY) < Math.abs(deltaX) * 1.1 || deltaY === 0) return;
    event.preventDefault();
    const direction = wheelStepper.push({ deltaX, deltaY, time: performance.now() });
    if (direction) selectAdjacent(direction);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (isTypingTarget(event.target)) return;
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      selectAdjacent(event.key === "ArrowDown" ? 1 : -1);
    } else if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      selectAdjacentMedia(event.key === "ArrowRight" ? 1 : -1);
    } else if (event.key === "Enter" && featured) {
      event.preventDefault();
      runAction(featured, "open", onAction);
    } else if (event.key.toLowerCase() === "f") {
      event.preventDefault();
      folded = !folded;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="nd-stage"
  data-route-focus
  data-controller-surface
  data-focus-key="game-visual-stage"
  data-gamepad-group
  class:nd-stage--folded={folded}
  class:nd-stage--shifting={shifting}
  class:nd-stage--previous={shiftDirection < 0}
  data-testid="game-unified-stage"
  data-selected-game={featured?.id ?? ""}
  data-reduced-motion={reducedMotion}
  role="application"
  tabindex="0"
  aria-labelledby="nd-stage-title"
  onwheel={handleWheel}
  onkeydown={handleKeydown}
>
  {#if featured}
    <div class="nd-ambient" aria-hidden="true">
      {#if activeAsset}<MediaArtwork src={activeAsset.src} alt="" title={featured.title} eager />{/if}
    </div>

    <header class="nd-register">
      <div><span>MOEPLAY / GAME CUBE</span><strong>{String(activeGameIndex + 1).padStart(3, "0")} — {String(uniqueItems.length).padStart(3, "0")}</strong></div>
      <div class="nd-register-actions">
        <span>WHEEL / GAME</span><span>← → / MEDIA</span>
        <button type="button" data-focus-key="game-visual-fold-toggle" data-gamepad-activate={folded ? "展开档案" : "折叠档案"} aria-pressed={folded} onclick={() => (folded = !folded)}>F / {folded ? "展开" : "折叠"}</button>
      </div>
    </header>

    <div class="nd-cube-wrap">
      <div class="nd-cube">
        <article class="nd-face nd-face--media" aria-label="当前游戏媒体">
          <button class="nd-lead" type="button" data-focus-key={`game-visual-lead-${featured.id}`} data-gamepad-activate="打开档案" onclick={() => runAction(featured, "open", onAction)} aria-label={`打开 ${featured.title}`}>
            {#if activeAsset}
              <MediaArtwork src={activeAsset.src} alt={activeAsset.alt} title={featured.title} eager />
            {:else}
              <span class="nd-letter" aria-hidden="true">{featured.title.slice(0, 1)}</span>
            {/if}
            <span class="nd-lead-shade" aria-hidden="true"></span>
            <span class="nd-lead-caption"><small>MEDIA / {String(activeMediaIndex + 1).padStart(2, "0")}</small><strong>{activeAsset?.role ?? "archive"}</strong></span>
          </button>

          <nav class="nd-media-map" aria-label="当前游戏媒体索引">
            {#each mediaAssets as asset, index (asset.id)}
              <button
                type="button"
                class:active={index === activeMediaIndex}
                aria-current={index === activeMediaIndex ? "true" : undefined}
                data-focus-key={`game-visual-media-${featured.id}-${asset.id}`}
                data-gamepad-activate="切换媒体"
                onclick={() => (activeMediaIndex = index)}
              >
                <span>{String(index + 1).padStart(2, "0")}</span>
                <strong>{asset.role}</strong>
              </button>
            {/each}
            {#if mediaAssets.length === 0}<span class="nd-media-empty">NO MEDIA / 等待补充封面与截图</span>{/if}
          </nav>
          <span class="nd-face-label">FRONT LEFT / MEDIA</span>
        </article>

        <article class="nd-face nd-face--archive" aria-label="游戏档案与目录">
          <div class="nd-archive-head">
          <div class="nd-title-block">
            <div class="nd-title-register">
              <span id="nd-stage-title">GAME ARCHIVE / {statusLabel(featured.metadata.completionStatus)}</span>
              {#if featured.metadata.rating}
                <div class="nd-score" aria-label={`评分 ${featured.metadata.rating} 分`}>
                  <strong>{featured.metadata.rating.toFixed(1)}</strong><small>/ 10</small>
                </div>
              {/if}
            </div>
            <h1>{featured.title}</h1>
            {#if featured.originalTitle}<p class="nd-original">{featured.originalTitle}</p>{/if}
            <p class="nd-summary">{featured.description || "以封面、截图、游玩状态和本地资料构成这份私人游戏档案。"}</p>
            {#if featured.metadata.tags.length}
              <div class="nd-tags" aria-label="游戏标签">
                {#each featured.metadata.tags.slice(0, 4) as tag}<span>{tag}</span>{/each}
              </div>
            {/if}
          </div>
          {#if featured.cover}
            <button class="nd-cover-window" type="button" data-focus-key={`game-visual-cover-${featured.id}`} data-gamepad-activate="打开档案" onclick={() => runAction(featured, "open", onAction)} aria-label={`打开 ${featured.title} 详情`}>
              <MediaArtwork src={featured.cover.src} alt={featured.cover.alt} title={featured.title} eager />
              <span>COVER / {String(activeGameIndex + 1).padStart(3, "0")}</span>
            </button>
          {/if}
          </div>

          <dl class="nd-facts">
            <div><dt>PLAYTIME</dt><dd>{formatPlaytime(featured.metadata.totalSeconds)}</dd></div>
            <div><dt>STATUS</dt><dd>{statusLabel(featured.metadata.completionStatus)}</dd></div>
            <div><dt>YEAR</dt><dd>{featured.metadata.releaseYear || "----"}</dd></div>
            <div><dt>STUDIO</dt><dd>{featured.metadata.developer || featured.metadata.publisher || featured.metadata.platform || "PC"}</dd></div>
          </dl>

          <nav class="nd-directory" aria-label="游戏目录">
            {#each directoryItems as entry (entry.item.id)}
              <button
                type="button"
                class:active={entry.item.id === featured.id}
                aria-current={entry.item.id === featured.id ? "true" : undefined}
                data-directory-game={entry.item.id}
                data-focus-key={`game-visual-directory-${entry.item.id}`}
                aria-label={entry.item.id === featured.id ? `打开 ${entry.item.title} 档案` : `切换到 ${entry.item.title}`}
                data-gamepad-activate={entry.item.id === featured.id ? "打开档案" : "切换游戏"}
                onclick={() => activateGame(entry.item)}
              >
                <span>{String(entry.index + 1).padStart(3, "0")}</span>
                <strong>{entry.item.title}</strong>
                <small>{statusLabel(entry.item.metadata.completionStatus)}</small>
              </button>
            {/each}
          </nav>

          <div class="nd-actions">
            {#if launch}<button class="primary" type="button" data-focus-key={`game-visual-launch-${featured.id}`} data-gamepad-activate="启动游戏" onclick={() => runAction(featured, "launch", onAction)}>{launch.label}</button>{/if}
            {#if open}<button type="button" data-focus-key={`game-visual-open-${featured.id}`} data-gamepad-activate="打开档案" onclick={() => runAction(featured, "open", onAction)}>打开档案</button>{/if}
            {#if favorite}<button type="button" data-focus-key={`game-visual-favorite-${featured.id}`} data-gamepad-secondary-action data-gamepad-activate={favorite.active ? "取消收藏" : "收藏"} aria-pressed={favorite.active} onclick={() => runAction(featured, "toggle-favorite", onAction)}>{favorite.active ? "已收藏" : "收藏"}</button>{/if}
          </div>
          <span class="nd-face-label">FRONT RIGHT / DIRECTORY</span>
        </article>
      </div>
    </div>

    <footer class="nd-footer"><span>SCROLL TO ROTATE THE ARCHIVE</span><strong>↑ ↓ 切换游戏 · ← → 切换媒体 · ENTER 打开 · F 折叠</strong></footer>
  {:else}
    <div class="mw-v2-empty nd-empty">
      <span>ARCHIVE 000</span><h1 id="nd-stage-title">建立你的第一份游戏档案</h1><p>导入游戏后，这里会成为可以旋转和切换的私人媒体目录。</p>
      {#if onImport}<button class="mw-v2-action mw-v2-action--accent" onclick={() => void onImport?.()}><span>导入游戏</span><i aria-hidden="true"></i></button>{/if}
    </div>
  {/if}
</section>
