<script lang="ts">
  import type { ConceptContentItem, ConceptMediaAsset, TemplateViewProps } from "../../contracts";
  import CinematicMedia from "./CinematicMedia.svelte";
  import "./cinematic.css";

  export let mode: TemplateViewProps["mode"];
  export let module: TemplateViewProps["module"];
  export let items: TemplateViewProps["items"];
  export let selectedId: TemplateViewProps["selectedId"];
  export let quality: TemplateViewProps["quality"];
  export let reducedMotion: TemplateViewProps["reducedMotion"];
  export let onSelect: TemplateViewProps["onSelect"];
  export let onOpen: TemplateViewProps["onOpen"];
  export let onBack: TemplateViewProps["onBack"];

  const moduleLabels = {
    games: "GAME ARCHIVE",
    anime: "ANIMATION ARCHIVE",
    comics: "COMIC ARCHIVE",
  } as const;

  const modeLabels = {
    visual: "VISUAL",
    index: "INDEX",
    scene: "SCENE",
  } as const;

  $: selected = items.find((item) => item.id === selectedId) ?? items[0];
  $: selectedIndex = Math.max(0, items.findIndex((item) => item.id === selected?.id));
  $: hero = pickHero(selected);
  $: sceneFrames = selected?.media.length ? selected.media : [];

  function pickHero(item: ConceptContentItem | undefined): ConceptMediaAsset | undefined {
    return item?.media.find((asset) => asset.shotType === "hero") ?? item?.media[0];
  }

  function pickFrame(item: ConceptContentItem): ConceptMediaAsset | undefined {
    return item.media.find((asset) => asset.shotType === "scene") ?? pickHero(item);
  }

  function archiveNumber(index: number): string {
    return String(index + 1).padStart(3, "0");
  }

  function timecode(itemIndex: number, frameIndex = 0): string {
    const seconds = itemIndex * 47 + frameIndex * 11;
    const minutes = Math.floor(seconds / 60);
    const remainder = seconds % 60;
    const frames = (itemIndex * 7 + frameIndex * 13) % 24;
    return `00:${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}:${String(frames).padStart(2, "0")}`;
  }

  function progressText(progress: number): string {
    const normalized = progress <= 1 ? progress * 100 : progress;
    return `${Math.max(0, Math.min(100, Math.round(normalized)))}%`;
  }
</script>

<section
  class="cinematic"
  class:cinematic--reduced={reducedMotion}
  class:cinematic--empty={!selected}
  data-mode={mode}
  data-quality={quality}
  aria-label={`${moduleLabels[module]} ${modeLabels[mode]}`}
>
  <div class="cinematic__grain" aria-hidden="true"></div>

  <header class="cinematic__header">
    <button class="cinematic__back" type="button" on:click={onBack} aria-label="返回上一级">
      <span aria-hidden="true">←</span>
      <span>BACK</span>
    </button>
    <div class="cinematic__identity">
      <span>MOEPLAY / 01</span>
      <strong>{moduleLabels[module]}</strong>
    </div>
    <div class="cinematic__status" aria-label={`当前模式 ${modeLabels[mode]}`}>
      <span>{modeLabels[mode]}</span>
      <span>{archiveNumber(selectedIndex)}</span>
      <i aria-hidden="true"></i>
    </div>
  </header>

  {#if !selected}
    <div class="cinematic__empty-state">
      <span>ARCHIVE OFFLINE</span>
      <h2>没有可放映的内容</h2>
      <button type="button" on:click={onBack}>返回</button>
    </div>
  {:else if mode === "visual"}
    <div class="visual-stage" data-concept-wheel="intent" data-concept-axis="horizontal">
      <button
        class="visual-stage__hero"
        data-concept-cursor="OPEN"
        data-testid="content-item"
        data-content-id={selected.id}
        type="button"
        on:click={() => onOpen(selected.id)}
        aria-label={`打开 ${selected.title}`}
      >
        <CinematicMedia asset={hero} alt={`${selected.title} 主视觉`} eager />
        <span class="visual-stage__shade" aria-hidden="true"></span>
        <span class="visual-stage__timecode">TC {timecode(selectedIndex)}</span>
        <span class="visual-stage__ratio">{hero?.ratio ?? "NO MEDIA"}</span>
      </button>

      <div class="visual-stage__copy">
        <div class="visual-stage__kicker">
          <span>FEATURE PRESENTATION</span>
          <span>{selected.status}</span>
        </div>
        <h1>{selected.title}</h1>
        <p class="visual-stage__subtitle">{selected.subtitle}</p>
        <p class="visual-stage__description">{selected.description}</p>
        <div class="visual-stage__meta">
          {#each selected.meta as meta}
            <span>{meta}</span>
          {/each}
        </div>
        <button class="cinematic__open" data-concept-cursor="OPEN" type="button" on:click={() => onOpen(selected.id)}>
          <span>ENTER ARCHIVE</span><span aria-hidden="true">↗</span>
        </button>
      </div>

      <nav class="filmstrip" aria-label="电影帧序列">
        {#each items as item, index (item.id)}
          <button
            class:filmstrip__frame--active={item.id === selected.id}
            class="filmstrip__frame"
            type="button"
            on:click={() => onSelect(item.id)}
            aria-pressed={item.id === selected.id}
            aria-label={`选择 ${item.title}`}
          >
            <span class="filmstrip__perforation" aria-hidden="true"></span>
            <CinematicMedia asset={pickFrame(item)} alt="" decorative />
            <span class="filmstrip__caption">
              <b>{archiveNumber(index)}</b>
              <span>{timecode(index)}</span>
            </span>
          </button>
        {/each}
      </nav>
    </div>
  {:else if mode === "index"}
    <div class="index-archive">
      <div class="index-archive__heading">
        <span>RULED CATALOGUE / {String(items.length).padStart(2, "0")} ENTRIES</span>
        <h1>INDEX<br />OF WORKS</h1>
        <p>以编号、进度与状态组织的规则档案。选择条目以校准放映焦点。</p>
      </div>

      <div class="index-archive__list" role="list" aria-label="内容索引">
        {#each items as item, index (item.id)}
          <article class:index-row--active={item.id === selected.id} class="index-row" role="listitem">
            <button
              class="index-row__select"
              type="button"
              on:click={() => onSelect(item.id)}
              aria-pressed={item.id === selected.id}
              aria-label={`选择档案 ${item.title}`}
            >
              <span class="index-row__number">{archiveNumber(index)}</span>
              <span class="index-row__title"><b>{item.title}</b><small>{item.subtitle}</small></span>
              <span class="index-row__status">{item.status}</span>
              <span class="index-row__progress">
                <i style:--progress={progressText(item.progress)} aria-hidden="true"></i>
                <small>{item.progressLabel || progressText(item.progress)}</small>
              </span>
              <span class="index-row__time">{timecode(index)}</span>
            </button>
            <button class="index-row__open" data-concept-cursor="OPEN" type="button" on:click={() => onOpen(item.id)} aria-label={`打开 ${item.title}`}>↗</button>
          </article>
        {/each}
      </div>

      <aside class="index-archive__preview" aria-live="polite">
        <div class="index-archive__preview-frame">
          <CinematicMedia asset={hero} alt={`${selected.title} 预览`} />
        </div>
        <span>{archiveNumber(selectedIndex)} / SELECTED FILE</span>
        <strong>{selected.title}</strong>
        <p>{selected.description}</p>
      </aside>
    </div>
  {:else}
    <div class="scene-archive">
      <div class="scene-archive__masthead">
        <span>CHAPTER CUT / {archiveNumber(selectedIndex)}</span>
        <h1>{selected.title}</h1>
        <div>
          <p>{selected.subtitle}</p>
          <button class="cinematic__open" data-concept-cursor="OPEN" type="button" on:click={() => onOpen(selected.id)}>
            <span>OPEN CHAPTER</span><span aria-hidden="true">↗</span>
          </button>
        </div>
      </div>

      <div class="scene-reel" aria-label={`${selected.title} 章节镜头`}>
        {#if sceneFrames.length}
          {#each sceneFrames as frame, frameIndex (frame.id)}
            <figure class:scene-shot--lead={frameIndex === 0} class="scene-shot">
              <div class="scene-shot__image"><CinematicMedia asset={frame} alt={`${selected.title} 镜头 ${frameIndex + 1}`} /></div>
              <figcaption>
                <span>SCENE {archiveNumber(frameIndex)}</span>
                <span>{timecode(selectedIndex, frameIndex)}</span>
                <span>{frame.shotType} / {frame.ratio}</span>
              </figcaption>
            </figure>
          {/each}
        {:else}
          <div class="scene-reel__missing">NO SCENE MATERIAL</div>
        {/if}
      </div>

      <footer class="scene-archive__chapters">
        <span>CHAPTER SELECT</span>
        <div>
          {#each items as item, index (item.id)}
            <button
              class:scene-chapter--active={item.id === selected.id}
              class="scene-chapter"
              type="button"
              on:click={() => onSelect(item.id)}
              aria-pressed={item.id === selected.id}
            >
              <span>{archiveNumber(index)}</span>
              <b>{item.title}</b>
            </button>
          {/each}
        </div>
      </footer>
    </div>
  {/if}
</section>





