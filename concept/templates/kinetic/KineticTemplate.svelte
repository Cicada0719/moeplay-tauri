<script lang="ts">
  import { onMount, tick } from "svelte";
  import { gsap } from "gsap";
  import type { ConceptContentItem, ConceptMediaAsset, TemplateViewProps } from "../../contracts";

  let {
    mode,
    module,
    items,
    selectedId,
    quality,
    reducedMotion,
    onSelect,
    onOpen,
    onBack,
  }: TemplateViewProps = $props();

  let root: HTMLElement;
  let current = 0;
  let target = 0;
  let velocity = 0;
  let activeIndex = $state(0);
  let dragging = $state(false);
  let moved = false;
  let pointerId: number | null = null;
  let dragStartY = 0;
  let dragStartTarget = 0;
  let dragDistance = 0;
  let raf = 0;
  let settleTimer = 0;
  let lastFrame = 0;
  let gsapContext: gsap.Context | undefined;
  let mounted = false;
  let interactionHint = $state<"DRAG" | "OPEN" | "CONTINUE">("DRAG");

  const DRAG_CLICK_THRESHOLD = 7;
  const WHEEL_SCALE = 0.00225;
  const DRAG_SCALE = 0.0045;

  const modulo = (value: number, length: number) => ((value % length) + length) % length;
  const wrappedDistance = (index: number, position: number, length: number) => {
    let distance = index - modulo(position, length);
    if (distance > length / 2) distance -= length;
    if (distance < -length / 2) distance += length;
    return distance;
  };

  function mediaFor(item: ConceptContentItem): ConceptMediaAsset | undefined {
    return item.media.find((asset) => asset.templateUsage.includes("kinetic")) ?? item.media[0];
  }

  function indexForId(id: string) {
    const found = items.findIndex((item) => item.id === id);
    return found < 0 ? 0 : found;
  }

  function nearestTargetFor(index: number) {
    if (!items.length) return 0;
    const base = Math.round(target / items.length) * items.length + index;
    const candidates = [base - items.length, base, base + items.length];
    return candidates.reduce((nearest, candidate) =>
      Math.abs(candidate - target) < Math.abs(nearest - target) ? candidate : nearest,
    );
  }

  function setTarget(index: number, announce = true) {
    if (!items.length) return;
    target = nearestTargetFor(modulo(index, items.length));
    interactionHint = "CONTINUE";
    scheduleSnap();
    if (announce) selectIndex(index);
  }

  function selectIndex(index: number) {
    const normalized = modulo(index, items.length);
    if (!items[normalized]) return;
    activeIndex = normalized;
    if (items[normalized].id !== selectedId) onSelect(items[normalized].id);
  }

  function step(direction: number) {
    if (!items.length) return;
    target = Math.round(target) + direction;
    interactionHint = "CONTINUE";
    selectIndex(Math.round(target));
    scheduleSnap();
  }

  function snap() {
    if (dragging || !items.length) return;
    target = Math.round(target);
    selectIndex(Math.round(target));
  }

  function scheduleSnap() {
    window.clearTimeout(settleTimer);
    settleTimer = window.setTimeout(snap, 130);
  }

  function updateDom() {
    if (!root || !items.length) return;
    const itemNodes = root.querySelectorAll<HTMLElement>("[data-kinetic-item]");
    const compact = mode === "index";
    const stepSize = compact ? 9.2 : mode === "scene" ? 46 : 34;

    itemNodes.forEach((node) => {
      const index = Number(node.dataset.index ?? 0);
      const distance = wrappedDistance(index, current, items.length);
      const absolute = Math.abs(distance);
      const scale = compact ? 1 : Math.max(0.68, 1 - absolute * (mode === "scene" ? 0.085 : 0.11));
      const opacity = Math.max(compact ? 0.16 : 0.1, 1 - absolute * (compact ? 0.24 : 0.29));
      const tilt = reducedMotion ? 0 : Math.max(-10, Math.min(10, velocity * -0.8 + distance * 1.3));
      node.style.setProperty("--item-y", `${distance * stepSize}vh`);
      node.style.setProperty("--item-scale", String(scale));
      node.style.setProperty("--item-opacity", String(opacity));
      node.style.setProperty("--item-tilt", `${tilt}deg`);
      node.style.zIndex = String(100 - Math.round(absolute * 10));
      node.toggleAttribute("data-active", absolute < 0.5);
      node.setAttribute("aria-hidden", absolute > (compact ? 4.5 : 2.5) ? "true" : "false");
    });

    root.style.setProperty("--kinetic-position", String(current));
    root.style.setProperty("--kinetic-target", String(target));
    root.style.setProperty("--kinetic-velocity", String(velocity));
    root.style.setProperty("--kinetic-speed", String(Math.min(1, Math.abs(velocity) / 5)));
    root.style.setProperty("--kinetic-direction", String(Math.sign(velocity)));
    root.dispatchEvent(new CustomEvent("kineticframe", {
      detail: { position: current, target, velocity, activeIndex },
    }));
  }

  function frame(time: number) {
    const delta = Math.min(34, time - lastFrame || 16.67) / 16.67;
    lastFrame = time;
    const previous = current;
    const easing = reducedMotion ? 0.5 : quality === "full" ? 0.105 : 0.16;
    current += (target - current) * (1 - Math.pow(1 - easing, delta));
    if (Math.abs(target - current) < 0.0005) current = target;
    velocity += (((current - previous) / Math.max(delta, 0.01)) * 60 - velocity) * 0.18;
    if (Math.abs(velocity) < 0.001) velocity = 0;
    updateDom();
    raf = requestAnimationFrame(frame);
  }

  function handleWheel(event: WheelEvent) {
    if (!items.length) return;
    event.preventDefault();
    const delta = Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
    target += Math.max(-1.15, Math.min(1.15, delta * WHEEL_SCALE));
    interactionHint = "CONTINUE";
    scheduleSnap();
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0 || !items.length) return;
    pointerId = event.pointerId;
    dragStartY = event.clientY;
    dragStartTarget = target;
    dragDistance = 0;
    moved = false;
    dragging = true;
    interactionHint = "DRAG";
    root.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent) {
    if (!dragging || pointerId !== event.pointerId) return;
    const delta = event.clientY - dragStartY;
    dragDistance = Math.max(dragDistance, Math.abs(delta));
    moved = dragDistance > DRAG_CLICK_THRESHOLD;
    target = dragStartTarget - delta * DRAG_SCALE;
  }

  function finishPointer(event: PointerEvent) {
    if (!dragging || pointerId !== event.pointerId) return;
    dragging = false;
    pointerId = null;
    if (root.hasPointerCapture(event.pointerId)) root.releasePointerCapture(event.pointerId);
    if (moved) {
      target += Math.max(-0.42, Math.min(0.42, velocity * 0.006));
      interactionHint = "CONTINUE";
      scheduleSnap();
    } else {
      interactionHint = "OPEN";
    }
  }

  function handleItemClick(item: ConceptContentItem, index: number) {
    if (moved) return;
    const isActive = modulo(Math.round(target), items.length) === index;
    if (isActive) onOpen(item.id);
    else setTarget(index);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      step(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      step(-1);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      const item = items[modulo(Math.round(target), items.length)];
      if (item) onOpen(item.id);
    } else if (event.key === "Escape") {
      onBack();
    }
  }

  $effect(() => {
    selectedId;
    items;
    if (!mounted || !items.length || dragging) return;
    const next = indexForId(selectedId);
    if (next !== modulo(Math.round(target), items.length)) {
      target = nearestTargetFor(next);
      activeIndex = next;
    }
  });

  $effect(() => {
    mode;
    if (!mounted) return;
    tick().then(updateDom);
  });

  onMount(() => {
    mounted = true;
    activeIndex = indexForId(selectedId);
    current = activeIndex;
    target = activeIndex;
    root.addEventListener("wheel", handleWheel, { passive: false });

    gsapContext = gsap.context(() => {
      if (!reducedMotion) {
        gsap.from("[data-kinetic-chrome]", {
          autoAlpha: 0,
          y: 12,
          duration: quality === "full" ? 0.75 : 0.35,
          stagger: 0.055,
          ease: "power3.out",
        });
      }
    }, root);

    raf = requestAnimationFrame(frame);

    return () => {
      mounted = false;
      root.removeEventListener("wheel", handleWheel);
      window.clearTimeout(settleTimer);
      cancelAnimationFrame(raf);
      gsapContext?.revert();
      gsap.killTweensOf(root.querySelectorAll("*"));
    };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={root}
  class:scene={mode === "scene"}
  class:index={mode === "index"}
  class:visual={mode === "visual"}
  class:dragging
  class="kinetic-template"
  data-concept-wheel="intent"
  data-concept-axis="horizontal"
  data-mode={mode}
  data-module={module}
  data-quality={quality}
  tabindex="0"
  role="application"
  aria-label={`Kinetic ${mode} view`}
  onkeydown={handleKeydown}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={finishPointer}
  onpointercancel={finishPointer}
>
  <div class="atmosphere" aria-hidden="true"></div>

  <header class="topbar" data-kinetic-chrome>
    <button class="back" type="button" onclick={onBack} aria-label="Back">←</button>
    <div class="identity">
      <span>MOEPLAY / KINETIC</span>
      <strong>{module.toUpperCase()}</strong>
    </div>
    <div class="mode-mark"><b>03</b><span>{mode.toUpperCase()}</span></div>
  </header>

  {#if items.length}
    <div class="stream" aria-live="polite">
      {#each items as item, index (item.id)}
        {@const media = mediaFor(item)}
        <article
          class="stream-item"
          class:active={activeIndex === index}
          data-kinetic-item
          data-index={index}
          aria-label={`${index + 1}. ${item.title}`}
        >
          <button class="item-hit" data-concept-cursor={mode === "scene" ? "CONTINUE" : mode === "index" ? "OPEN" : "DRAG"} data-testid="content-item" data-content-id={item.id} type="button" onclick={() => handleItemClick(item, index)}>
            {#if mode !== "index"}
              <div class="media-shell">
                {#if media?.mediaType === "video"}
                  <video
                    src={media.src}
                    poster={media.placeholder}
                    muted
                    loop
                    autoplay={!reducedMotion}
                    playsinline
                    preload="metadata"
                    style={`object-position:${media.focalPoint.x * 100}% ${media.focalPoint.y * 100}%;--media-tone:${media.dominantColor}`}
                  ></video>
                {:else if media}
                  <img
                    src={media.src}
                    alt=""
                    draggable="false"
                    loading={index < 2 ? "eager" : "lazy"}
                    style={`object-position:${media.focalPoint.x * 100}% ${media.focalPoint.y * 100}%;--media-tone:${media.dominantColor}`}
                  />
                {:else}
                  <div class="media-empty" aria-hidden="true"></div>
                {/if}
                <span class="media-index">{String(index + 1).padStart(2, "0")}</span>
              </div>
            {/if}

            <div class="copy">
              <span class="eyebrow">{item.status} · {item.progressLabel}</span>
              <h2>{item.title}</h2>
              <p class="subtitle">{item.subtitle}</p>
              {#if mode === "scene"}
                <p class="description">{item.description}</p>
              {/if}
              <div class="meta">
                {#each item.meta.slice(0, mode === "index" ? 3 : 2) as value}<span>{value}</span>{/each}
              </div>
            </div>

            {#if mode === "index"}
              <span class="index-progress" aria-label={`Progress ${item.progress}%`}>
                <i style={`--progress:${Math.max(0, Math.min(100, item.progress))}%`}></i>
                <b>{String(index + 1).padStart(2, "0")}</b>
              </span>
            {/if}
          </button>
        </article>
      {/each}
    </div>

    <nav class="thumb-map" aria-label="Media map" data-kinetic-chrome>
      {#each items as item, index (item.id)}
        {@const thumb = mediaFor(item)}
        <button
          type="button"
          class:active={activeIndex === index}
          aria-label={`Go to ${item.title}`}
          aria-current={activeIndex === index ? "true" : undefined}
          onclick={(event) => { event.stopPropagation(); setTarget(index); }}
        >
          {#if thumb}
            <img src={thumb.placeholder ?? thumb.src} alt="" draggable="false" loading="lazy" />
          {:else}
            <span></span>
          {/if}
          <i></i>
        </button>
      {/each}
    </nav>

    <div class="counter" data-kinetic-chrome aria-hidden="true">
      <strong>{String(activeIndex + 1).padStart(2, "0")}</strong>
      <span>/ {String(items.length).padStart(2, "0")}</span>
    </div>

    <div class="interaction-hint" data-kinetic-chrome aria-hidden="true">
      <span class:active={interactionHint === "OPEN"}>OPEN</span>
      <span class:active={interactionHint === "CONTINUE"}>CONTINUE</span>
      <span class:active={interactionHint === "DRAG"}>DRAG</span>
      <i></i>
      <b>↑↓</b>
    </div>
  {:else}
    <div class="empty" role="status">
      <span>00 / 00</span>
      <h2>NO MEDIA IN STREAM</h2>
      <button type="button" onclick={onBack}>RETURN</button>
    </div>
  {/if}
</div>

<style>
  :global(*) { box-sizing: border-box; }
  .kinetic-template {
    --ink: #f3f1e8;
    --muted: rgba(243, 241, 232, .56);
    --signal: #ff583d;
    --kinetic-velocity: 0;
    --kinetic-speed: 0;
    position: relative;
    width: 100%;
    min-height: 100dvh;
    overflow: hidden;
    isolation: isolate;
    color: var(--ink);
    background: #0a0a0a;
    font-family: "Helvetica Neue", "Arial Narrow", Arial, sans-serif;
    outline: none;
    touch-action: none;
    cursor: grab;
    user-select: none;
  }
  .kinetic-template.dragging { cursor: grabbing; }
  .kinetic-template:focus-visible { box-shadow: inset 0 0 0 2px var(--signal); }
  .atmosphere {
    position: absolute;
    inset: -15%;
    z-index: -1;
    background:
      radial-gradient(circle at calc(50% + var(--kinetic-direction) * 8%) 50%, rgba(255,88,61,.11), transparent 31%),
      repeating-linear-gradient(90deg, transparent 0 11.9vw, rgba(255,255,255,.035) 12vw);
    transform: skewY(calc(var(--kinetic-velocity) * -.03deg)) scale(1.1);
    pointer-events: none;
  }
  .topbar {
    position: absolute;
    inset: 1.5rem 1.75rem auto;
    z-index: 300;
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 1rem;
    pointer-events: none;
  }
  .topbar button, .thumb-map button { pointer-events: auto; }
  .back {
    width: 2.5rem; height: 2.5rem; border: 1px solid rgba(255,255,255,.22); border-radius: 50%;
    color: inherit; background: rgba(10,10,10,.2); font-size: 1rem; cursor: pointer;
  }
  .back:hover, .back:focus-visible { color: #0a0a0a; background: var(--ink); outline: none; }
  .identity { display: flex; align-items: baseline; gap: .8rem; font: 600 .62rem/1.1 monospace; letter-spacing: .14em; }
  .identity span { color: var(--muted); }
  .mode-mark { display: flex; align-items: baseline; gap: .65rem; font: 600 .62rem/1 monospace; letter-spacing: .15em; }
  .mode-mark b { color: var(--signal); font-size: 1.15rem; }
  .stream { position: absolute; inset: 0; }
  .stream-item {
    --item-y: 0vh; --item-scale: 1; --item-opacity: 1; --item-tilt: 0deg;
    position: absolute;
    inset: 50% auto auto 50%;
    width: min(76vw, 68rem);
    opacity: var(--item-opacity);
    transform: translate(-50%, calc(-50% + var(--item-y))) scale(var(--item-scale)) rotateZ(var(--item-tilt));
    transform-origin: center;
    will-change: transform, opacity;
    pointer-events: none;
  }
  .stream-item.active { pointer-events: auto; }
  .item-hit { display: grid; grid-template-columns: minmax(15rem, 1.42fr) minmax(13rem, .78fr); width: 100%; gap: clamp(1.2rem, 3vw, 3.5rem); align-items: end; padding: 0; border: 0; color: inherit; background: none; text-align: left; font: inherit; cursor: pointer; }
  .media-shell { position: relative; height: min(48vh, 31rem); overflow: hidden; background: #181818; clip-path: polygon(calc(var(--kinetic-speed) * 3%) 0, 100% 0, calc(100% - var(--kinetic-speed) * 4%) 100%, 0 100%); }
  .media-shell::after { content: ""; position: absolute; inset: 0; border: 1px solid rgba(255,255,255,.14); box-shadow: inset 0 -10rem 10rem -10rem #000; pointer-events: none; }
  .media-shell img, .media-shell video { width: 100%; height: 100%; display: block; object-fit: cover; filter: saturate(calc(.78 + var(--kinetic-speed) * .42)) contrast(1.05); transform: scale(calc(1.02 + var(--kinetic-speed) * .055)) translateY(calc(var(--kinetic-velocity) * -.08%)); will-change: transform; }
  .media-empty { width: 100%; height: 100%; background: linear-gradient(135deg, #242424, #101010); }
  .media-index { position: absolute; z-index: 2; left: 1rem; top: .85rem; font: 600 .65rem/1 monospace; letter-spacing: .12em; }
  .copy { min-width: 0; padding-bottom: 1rem; }
  .eyebrow { display: block; margin-bottom: .9rem; color: var(--signal); font: 600 .62rem/1.2 monospace; letter-spacing: .14em; text-transform: uppercase; }
  h2 { margin: 0; max-width: 12ch; font-size: clamp(2.5rem, 6.2vw, 7rem); line-height: .79; letter-spacing: -.072em; text-transform: uppercase; overflow-wrap: anywhere; }
  .subtitle { margin: 1.15rem 0 0; color: var(--muted); font-size: clamp(.8rem, 1.1vw, 1rem); line-height: 1.45; }
  .description { max-width: 35rem; margin: 1rem 0 0; color: rgba(243,241,232,.72); font-size: .86rem; line-height: 1.6; }
  .meta { display: flex; flex-wrap: wrap; gap: .45rem 1rem; margin-top: 1.2rem; color: var(--muted); font: 500 .62rem/1.2 monospace; letter-spacing: .08em; text-transform: uppercase; }
  .thumb-map { position: absolute; z-index: 300; right: 1.75rem; top: 50%; display: flex; flex-direction: column; gap: .42rem; transform: translateY(-50%); }
  .thumb-map button { position: relative; width: 2.6rem; height: 1.7rem; padding: 0; overflow: hidden; border: 0; opacity: .35; background: #333; cursor: pointer; transition: width .25s ease, opacity .25s ease; }
  .thumb-map button.active, .thumb-map button:hover, .thumb-map button:focus-visible { width: 4.2rem; opacity: 1; outline: none; }
  .thumb-map img, .thumb-map span { display: block; width: 100%; height: 100%; object-fit: cover; filter: grayscale(.25); }
  .thumb-map i { position: absolute; inset: auto 0 0; height: 2px; background: var(--signal); transform: scaleX(0); transform-origin: left; transition: transform .25s ease; }
  .thumb-map .active i { transform: scaleX(1); }
  .counter { position: absolute; z-index: 300; left: 1.75rem; bottom: 1.6rem; display: flex; align-items: baseline; gap: .5rem; font-family: monospace; }
  .counter strong { font-size: 2rem; color: var(--signal); }
  .counter span { color: var(--muted); font-size: .68rem; }
  .interaction-hint { position: absolute; z-index: 300; right: 1.75rem; bottom: 1.8rem; display: flex; align-items: center; gap: .65rem; font: 600 .6rem/1 monospace; letter-spacing: .13em; }
  .interaction-hint span { display: none; }
  .interaction-hint span.active { display: block; }
  .interaction-hint i { width: 3rem; height: 1px; background: rgba(255,255,255,.35); position: relative; overflow: hidden; }
  .interaction-hint i::after { content: ""; position: absolute; inset: 0; background: var(--signal); transform: translateX(calc(-100% + var(--kinetic-speed) * 100%)); }
  .interaction-hint b { color: var(--muted); }
  .empty { position: absolute; inset: 0; display: grid; place-content: center; justify-items: center; gap: 1rem; text-align: center; }
  .empty span { color: var(--signal); font: 600 .7rem monospace; letter-spacing: .15em; }
  .empty h2 { font-size: clamp(2.8rem, 8vw, 7rem); max-width: 10ch; }
  .empty button { padding: .8rem 1.3rem; border: 1px solid var(--ink); color: inherit; background: transparent; font: 600 .65rem monospace; letter-spacing: .15em; }

  /* Index is a dense typographic scanner, while retaining the same loop physics. */
  .index .stream-item { width: min(82vw, 72rem); }
  .index .item-hit { grid-template-columns: 1fr auto; align-items: center; min-height: 7.2vh; padding: .55rem 0; border-bottom: 1px solid rgba(255,255,255,.13); }
  .index .copy { display: grid; grid-template-columns: minmax(8rem, .55fr) minmax(12rem, 1.3fr) minmax(8rem, .8fr) auto; align-items: center; gap: 1.2rem; padding: 0; }
  .index .eyebrow { margin: 0; }
  .index h2 { max-width: none; font-size: clamp(1.25rem, 2.8vw, 2.7rem); line-height: .9; letter-spacing: -.045em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .index .subtitle { margin: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .index .meta { margin: 0; flex-wrap: nowrap; }
  .index-progress { display: grid; grid-template-columns: 4.8rem 2rem; align-items: center; gap: .75rem; font: 600 .65rem monospace; }
  .index-progress i { position: relative; width: 100%; height: 2px; background: rgba(255,255,255,.15); }
  .index-progress i::after { content: ""; position: absolute; inset: 0; background: var(--signal); transform: scaleX(calc(var(--progress) / 100%)); transform-origin: left; }
  .index .stream-item.active .item-hit { color: var(--signal); }
  .index .thumb-map button { width: .65rem; height: .65rem; border-radius: 50%; }
  .index .thumb-map button img { display: none; }
  .index .thumb-map button.active { width: 1.6rem; border-radius: 1rem; background: var(--signal); }

  /* Scene gives the active media nearly the full viewport and overlays narrative copy. */
  .scene .stream-item { width: min(86vw, 86rem); }
  .scene .item-hit { display: block; }
  .scene .media-shell { height: min(68vh, 48rem); }
  .scene .copy { position: absolute; z-index: 3; left: clamp(1.25rem, 4vw, 4rem); right: clamp(1.25rem, 42vw, 39rem); bottom: clamp(1.25rem, 4vw, 3.5rem); padding: 0; text-shadow: 0 2px 24px rgba(0,0,0,.82); }
  .scene h2 { max-width: 10ch; font-size: clamp(3.2rem, 7.6vw, 8.6rem); }
  .scene .subtitle, .scene .description, .scene .meta { color: rgba(255,255,255,.8); }

  @media (max-width: 760px) {
    .topbar { inset: 1rem 1rem auto; }
    .identity span { display: none; }
    .stream-item, .index .stream-item, .scene .stream-item { width: calc(100vw - 2rem); }
    .item-hit { display: block; }
    .media-shell { height: 46vh; }
    .copy { padding: 1.25rem .25rem 0; }
    h2 { max-width: 10ch; font-size: clamp(2.45rem, 13vw, 4.6rem); }
    .thumb-map { right: .65rem; top: auto; bottom: 4.65rem; max-width: calc(100vw - 1.3rem); flex-direction: row; transform: none; overflow: hidden; }
    .thumb-map button { width: 1.7rem; height: 1rem; flex: 0 0 auto; }
    .thumb-map button.active, .thumb-map button:hover { width: 2.8rem; }
    .counter { left: 1rem; bottom: 1rem; }
    .interaction-hint { right: 1rem; bottom: 1.2rem; }
    .index .item-hit { display: grid; min-height: 8vh; }
    .index .copy { grid-template-columns: 1fr; gap: .25rem; }
    .index .eyebrow, .index .subtitle, .index .meta { display: none; }
    .index h2 { font-size: 1.5rem; }
    .index-progress { grid-template-columns: 2.7rem 1.5rem; }
    .scene .media-shell { height: 70vh; }
    .scene .copy { left: 1.1rem; right: 1.1rem; bottom: 1.2rem; padding: 0; }
    .scene .description { display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  }

  @media (prefers-reduced-motion: reduce) {
    .kinetic-template *, .kinetic-template *::before, .kinetic-template *::after { scroll-behavior: auto !important; transition-duration: .01ms !important; animation-duration: .01ms !important; }
    .atmosphere, .media-shell, .media-shell img, .media-shell video { transform: none !important; clip-path: none; }
  }
</style>









