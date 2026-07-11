<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { ConceptContentItem, ConceptMediaAsset, ContentMode, ContentStageState, NavigationIntent } from "./contracts";
  import { DEMO_CONTENT_BY_MODULE, DEFAULT_SELECTED_ID_BY_MODULE, getDemoContent } from "./content";
  import { contentStage } from "./state";
  import { ConceptShell } from "./shell";
  import { toneClass } from "./core";
  import { detectMotionQuality, WebGLMediaStage } from "./rendering";
  import CinematicTemplate from "./templates/cinematic/CinematicTemplate.svelte";
  import EditorialTemplate from "./templates/editorial/EditorialTemplate.svelte";
  import KineticTemplate from "./templates/kinetic/KineticTemplate.svelte";
  import ReviewPanel, { type ReviewViewport } from "./review/ReviewPanel.svelte";

  let stageState: ContentStageState = $state(contentStage.getSnapshot());
  let reducedMotion = $state(false);
  let viewport = $state<ReviewViewport>("responsive");
  let lastFocusKey = $state("");

  const unsubscribe = contentStage.subscribe((value) => (stageState = value));
  const mode = $derived(stageState.modeByModule[stageState.module]);
  const items = $derived([...DEMO_CONTENT_BY_MODULE[stageState.module]]);
  const selectedId = $derived(stageState.selectedIdByModule[stageState.module] || items[0]?.id || "");
  const selected = $derived(items.find((item) => item.id === selectedId) ?? items[0]);
  const activeTone = $derived(activeMedia(selected, stageState.template)?.tone ?? "dark");
  const quality = $derived(reducedMotion ? "reduced" : stageState.quality);
  const detailItem = $derived(stageState.detailId ? getDemoContent(stageState.detailId) : undefined);
  const detailMedia = $derived(activeMedia(detailItem, stageState.template));
  const kineticAssets = $derived(items.map((item) => activeMedia(item, "kinetic")).filter((asset): asset is ConceptMediaAsset => Boolean(asset)));
  const kineticActiveIndex = $derived(Math.max(0, items.findIndex((item) => item.id === selectedId)));

  function activeMedia(item: ConceptContentItem | undefined, template: typeof stageState.template): ConceptMediaAsset | undefined {
    return item?.media.find((asset) => asset.templateUsage.includes(template)) ?? item?.media[0];
  }

  function ensureSelection() {
    const id = stageState.selectedIdByModule[stageState.module];
    if (!items.some((item) => item.id === id)) contentStage.select(items[0]?.id ?? "", stageState.module);
  }

  function select(id: string) {
    contentStage.select(id);
    contentStage.setFocus(`content:${id}`);
  }

  function openDetail(id: string) {
    lastFocusKey = `content:${id}`;
    contentStage.setFocus(lastFocusKey);
    contentStage.openDetail(id);
  }

  async function closeDetail() {
    contentStage.closeDetail();
    await tick();
    const id = lastFocusKey.replace(/^content:/, "");
    document.querySelector<HTMLElement>(`[data-content-id="${CSS.escape(id)}"]`)?.focus();
  }

  function moveSelection(direction: number) {
    if (!items.length) return;
    const current = Math.max(0, items.findIndex((item) => item.id === selectedId));
    select(items[(current + direction + items.length) % items.length].id);
  }

  function handleIntent(intent: NavigationIntent) {
    if (intent === "switch-mode-left") return contentStage.cycleMode(-1);
    if (intent === "switch-mode-right") return contentStage.cycleMode(1);
    if (intent === "back") return stageState.detailId ? closeDetail() : undefined;
    if (intent === "activate") return selectedId ? openDetail(selectedId) : undefined;
    if (intent === "next" || intent === "page-next") moveSelection(1);
    if (intent === "previous" || intent === "page-previous") moveSelection(-1);
  }

  function setViewport(next: ReviewViewport) { viewport = next; }

  function stageScrollElement(): HTMLElement | null {
    return document.querySelector<HTMLElement>(".concept-shell__stage");
  }

  function rememberScroll() {
    contentStage.setScroll(stageScrollElement()?.scrollTop ?? window.scrollY, stageState.module, mode);
  }

  async function restoreScroll() {
    await tick();
    const key = `${stageState.module}:${mode}`;
    const position = stageState.scrollPositionByModuleMode[key] ?? 0;
    stageScrollElement()?.scrollTo({ top: position, behavior: reducedMotion ? "auto" : "instant" });
  }

  onMount(() => {
    ensureSelection();
    contentStage.setReviewOpen(true);
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const sync = () => (reducedMotion = media.matches);
    sync(); media.addEventListener("change", sync);
    if (!media.matches && stageState.quality === "full") contentStage.setQuality(detectMotionQuality());
    const keys = (event: KeyboardEvent) => {
      if (event.key === "1") contentStage.setMode("visual");
      if (event.key === "2") contentStage.setMode("index");
      if (event.key === "3") contentStage.setMode("scene");
      if (event.key.toLowerCase() === "r") contentStage.setReviewOpen(!contentStage.getSnapshot().reviewOpen);
    };
    window.addEventListener("keydown", keys);
    const scrollRoot = stageScrollElement();
    scrollRoot?.addEventListener("scroll", rememberScroll, { passive: true });
    restoreScroll();
    return () => { unsubscribe(); media.removeEventListener("change", sync); window.removeEventListener("keydown", keys); scrollRoot?.removeEventListener("scroll", rememberScroll); };
  });
</script>

<svelte:head><title>MoePlay / SHIFTBRAIN Concept</title></svelte:head>

<div class="concept-viewport" data-testid="concept-viewport" data-viewport={viewport}>
  <ConceptShell
    template={stageState.template}
    module={stageState.module}
    {mode}
    toneClass={toneClass(activeTone)}
    cursorLabel={stageState.detailId ? "BACK" : mode === "scene" ? "CONTINUE" : "VIEW"}
    onTemplateChange={(value) => contentStage.setTemplate(value)}
    onModuleChange={(value) => { rememberScroll(); contentStage.setModule(value); queueMicrotask(() => { ensureSelection(); restoreScroll(); }); }}
    onModeChange={(value) => { rememberScroll(); contentStage.setMode(value); restoreScroll(); }}
    onIntent={(intent) => handleIntent(intent)}
    onSettings={() => contentStage.setReviewOpen(true)}
  >
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="concept-stage"
      data-testid="concept-stage"
      data-template={stageState.template}
      data-module={stageState.module}
      data-mode={mode}
      data-quality={quality}
      data-reduced-motion={String(reducedMotion)}
      data-selected-id={selectedId}
      tabindex="0"
      aria-label="MoePlay 概念内容舞台"
    >
      {#if stageState.template === "kinetic" && mode === "scene" && !detailItem}
        <div class="kinetic-webgl-stage" aria-hidden="true">
          {#key stageState.module}
            <WebGLMediaStage assets={kineticAssets} activeIndex={kineticActiveIndex} velocity={1} {quality} {reducedMotion} />
          {/key}
        </div>
      {/if}
      {#if detailItem}
        <article class={`concept-detail concept-detail--${stageState.template}`} data-testid="concept-detail" data-content-id={detailItem.id}>
          {#if detailMedia}<img src={detailMedia.src} alt="" style={`object-position:${detailMedia.focalPoint.x * 100}% ${detailMedia.focalPoint.y * 100}%`} />{/if}
          <div class="detail-scrim"></div>
          <button type="button" data-testid="detail-back" onclick={closeDetail}>← 返回原视图</button>
          <div class="detail-copy">
            <span>{stageState.template.toUpperCase()} / {stageState.module.toUpperCase()}</span>
            <h1>{detailItem.title}</h1>
            <p>{detailItem.description}</p>
            <div>{#each detailItem.meta as meta}<small>{meta}</small>{/each}</div>
          </div>
        </article>
      {:else if stageState.template === "cinematic"}
        <CinematicTemplate {mode} module={stageState.module} {items} {selectedId} {quality} {reducedMotion} onSelect={select} onOpen={openDetail} onBack={() => undefined} />
      {:else if stageState.template === "editorial"}
        <EditorialTemplate {mode} module={stageState.module} {items} {selectedId} {quality} {reducedMotion} onSelect={select} onOpen={openDetail} onBack={() => undefined} />
      {:else}
        <KineticTemplate {mode} module={stageState.module} {items} {selectedId} {quality} {reducedMotion} onSelect={select} onOpen={openDetail} onBack={() => undefined} />
      {/if}
    </div>
  </ConceptShell>

  <button class="review-trigger" type="button" aria-label="打开评审面板" onclick={() => contentStage.setReviewOpen(true)}>REVIEW / R</button>
  <ReviewPanel
    open={stageState.reviewOpen}
    template={stageState.template}
    module={stageState.module}
    {mode}
    quality={stageState.quality}
    muted={stageState.muted}
    {viewport}
    onTemplateChange={(value) => contentStage.setTemplate(value)}
    onModuleChange={(value) => { rememberScroll(); contentStage.setModule(value); queueMicrotask(() => { ensureSelection(); restoreScroll(); }); }}
    onModeChange={(value: ContentMode) => contentStage.setMode(value)}
    onQualityChange={(value) => contentStage.setQuality(value)}
    onMutedChange={(value) => contentStage.setMuted(value)}
    onViewportChange={setViewport}
    onClose={() => contentStage.setReviewOpen(false)}
  />
</div>










