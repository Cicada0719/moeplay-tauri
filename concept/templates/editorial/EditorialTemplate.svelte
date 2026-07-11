<script lang="ts">
  import type {
    ConceptContentItem,
    TemplateViewProps,
  } from "../../contracts";

  export let mode: TemplateViewProps["mode"];
  export let module: TemplateViewProps["module"];
  export let items: TemplateViewProps["items"];
  export let selectedId: TemplateViewProps["selectedId"];
  export let quality: TemplateViewProps["quality"];
  export let reducedMotion: TemplateViewProps["reducedMotion"];
  export let onSelect: TemplateViewProps["onSelect"];
  export let onOpen: TemplateViewProps["onOpen"];
  export let onBack: TemplateViewProps["onBack"];

  let rowButtons: HTMLButtonElement[] = [];

  const moduleLabels = {
    games: "游戏",
    anime: "动画",
    comics: "漫画",
  } as const;

  const clampProgress = (value: number) => Math.min(100, Math.max(0, value));
  const itemNumber = (index: number) => String(index + 1).padStart(2, "0");

  const editorialMedia = (item: ConceptContentItem) => {
    const scoped = item.media.filter((asset) => asset.templateUsage.includes("editorial"));
    return scoped.length > 0 ? scoped : item.media;
  };

  const focalPercent = (value: number) => `${value <= 1 ? value * 100 : value}%`;

  const preferredMedia = (item: ConceptContentItem | undefined) => {
    if (!item) return undefined;
    const media = editorialMedia(item);
    return (
      media.find((asset) => asset.shotType === "hero") ??
      media.find((asset) => asset.shotType === "cover") ??
      media[0]
    );
  };

  const sceneMedia = (item: ConceptContentItem) => {
    const media = editorialMedia(item);
    const ordered = [
      ...media.filter((asset) => asset.shotType === "scene"),
      ...media.filter((asset) => asset.shotType !== "scene"),
    ];
    return ordered.slice(0, quality === "reduced" ? 3 : 5);
  };

  const focusRow = (index: number) => {
    const nextIndex = (index + items.length) % items.length;
    const next = items[nextIndex];
    if (!next) return;
    onSelect(next.id);
    rowButtons[nextIndex]?.focus();
  };

  const handleIndexKeydown = (event: KeyboardEvent, index: number, id: string) => {
    if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      event.preventDefault();
      focusRow(index + 1);
    } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      event.preventDefault();
      focusRow(index - 1);
    } else if (event.key === "Home") {
      event.preventDefault();
      focusRow(0);
    } else if (event.key === "End") {
      event.preventDefault();
      focusRow(items.length - 1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      onOpen(id);
    }
  };

  $: selectedIndex = Math.max(0, items.findIndex((item) => item.id === selectedId));
  $: selectedItem = items[selectedIndex];
  $: selectedMedia = preferredMedia(selectedItem);
</script>

<svelte:head>
  <meta name="theme-color" content="#090a0a" />
</svelte:head>

<section
  class:reduced={reducedMotion}
  class="editorial"
  data-mode={mode}
  data-quality={quality}
  aria-label={`${moduleLabels[module]}编辑索引`}
>
  <header class="masthead">
    <button class="back" type="button" on:click={onBack} aria-label="返回上一级">
      <span aria-hidden="true">←</span>
      <span>返回</span>
    </button>
    <div class="identity" aria-label="Editorial template">
      <span>MOEPLAY</span>
      <strong>EDITORIAL / 02</strong>
    </div>
    <div class="mode-stamp">
      <span>{moduleLabels[module]}</span>
      <strong>{mode}</strong>
      <span>{itemNumber(selectedIndex)} / {String(items.length).padStart(2, "0")}</span>
    </div>
  </header>

  {#if items.length === 0}
    <div class="empty" role="status">
      <span>00 / EMPTY</span>
      <h1>暂无可编目的内容</h1>
      <button type="button" on:click={onBack}>返回资料库</button>
    </div>
  {:else if mode === "visual" && selectedItem}
    <div class="visual-view" data-concept-wheel="intent" data-concept-axis="horizontal">
      <div class="visual-copy">
        <p class="eyebrow">{itemNumber(selectedIndex)} / {selectedItem.status}</p>
        <button class="title-trigger" data-concept-cursor="VIEW" data-testid="content-item" data-content-id={selectedItem.id} type="button" on:click={() => onOpen(selectedItem.id)}>
          <span>{selectedItem.title}</span>
          <small>{selectedItem.subtitle}</small>
        </button>
        <p class="description">{selectedItem.description}</p>
        <div class="visual-meta" aria-label="作品信息">
          {#each selectedItem.meta as entry}
            <span>{entry}</span>
          {/each}
        </div>
      </div>

      <div class="visual-media" style={`--media-color:${selectedMedia?.dominantColor ?? "#171918"}`}>
        {#if selectedMedia}
          {#if selectedMedia.mediaType === "video"}
            <video
              src={selectedMedia.src}
              poster={selectedMedia.placeholder}
              muted
              autoplay={!reducedMotion}
              loop
              playsinline
              aria-label={`${selectedItem.title} 媒体预览`}
              style={`object-position:${focalPercent(selectedMedia.focalPoint.x)} ${focalPercent(selectedMedia.focalPoint.y)}`}
            ></video>
          {:else}
            <img
              src={selectedMedia.src}
              alt={`${selectedItem.title} 媒体预览`}
              style={`object-position:${focalPercent(selectedMedia.focalPoint.x)} ${focalPercent(selectedMedia.focalPoint.y)}`}
            />
          {/if}
        {:else}
          <div class="media-missing" role="img" aria-label={`${selectedItem.title} 暂无媒体`}>
            <span>NO MEDIA / {itemNumber(selectedIndex)}</span>
          </div>
        {/if}
        <div class="crop-mark crop-mark--top" aria-hidden="true"></div>
        <div class="crop-mark crop-mark--bottom" aria-hidden="true"></div>
        <span class="media-caption">{selectedMedia?.shotType ?? "archive"} / {selectedMedia?.ratio ?? "none"}</span>
      </div>

      <nav class="visual-rail" aria-label="选择内容">
        {#each items as item, index}
          <button
            type="button"
            class:active={item.id === selectedItem.id}
            aria-current={item.id === selectedItem.id ? "true" : undefined}
            on:mouseenter={() => onSelect(item.id)}
            on:focus={() => onSelect(item.id)}
            on:click={() => onSelect(item.id)}
          >
            <span>{itemNumber(index)}</span>
            <span>{item.title}</span>
            <span>{item.progressLabel}</span>
          </button>
        {/each}
      </nav>
    </div>
  {:else if mode === "index" && selectedItem}
    <div class="index-view">
      <div class="index-heading">
        <div>
          <span>CATALOGUE / {moduleLabels[module]}</span>
          <span>选择条目以同步预览，回车打开详情</span>
        </div>
        <strong>{String(items.length).padStart(2, "0")}</strong>
      </div>

      <div class="index-body">
        <div class="index-list" role="listbox" aria-label={`${moduleLabels[module]}内容索引`}>
          <div class="column-labels" aria-hidden="true">
            <span>编号 / 标题</span><span>分类状态</span><span>进度</span><span>资料</span>
          </div>
          {#each items as item, index}
            <div class:active={item.id === selectedItem.id} class="index-row">
              <button
                bind:this={rowButtons[index]}
                data-testid="content-item"
                data-content-id={item.id}
                type="button"
                role="option"
                aria-selected={item.id === selectedItem.id}
                tabindex={item.id === selectedItem.id ? 0 : -1}
                on:mouseenter={() => onSelect(item.id)}
                on:focus={() => onSelect(item.id)}
                on:click={() => onSelect(item.id)}
                on:keydown={(event) => handleIndexKeydown(event, index, item.id)}
              >
                <span class="row-title">
                  <b>{itemNumber(index)}</b>
                  <span><strong>{item.title}</strong><small>{item.subtitle}</small></span>
                </span>
                <span class="row-status"><i></i>{item.status}</span>
                <span class="row-progress">
                  <span><b>{Math.round(clampProgress(item.progress))}%</b><small>{item.progressLabel}</small></span>
                  <i><i style={`width:${clampProgress(item.progress)}%`}></i></i>
                </span>
                <span class="row-meta">{item.meta.slice(0, 2).join(" / ") || "—"}</span>
              </button>
              <button class="row-open" data-concept-cursor="OPEN" type="button" on:click={() => onOpen(item.id)} aria-label={`打开 ${item.title} 详情`}>
                <span>OPEN</span><span aria-hidden="true">↗</span>
              </button>
            </div>
          {/each}
        </div>

        <aside class="index-preview" aria-live="polite">
          <div class="preview-frame" style={`--media-color:${selectedMedia?.dominantColor ?? "#171918"}`}>
            {#if selectedMedia}
              {#if selectedMedia.mediaType === "video"}
                <video
                  src={selectedMedia.src}
                  poster={selectedMedia.placeholder}
                  muted
                  autoplay={!reducedMotion}
                  loop
                  playsinline
                  aria-label={`${selectedItem.title} 媒体预览`}
                  style={`object-position:${focalPercent(selectedMedia.focalPoint.x)} ${focalPercent(selectedMedia.focalPoint.y)}`}
                ></video>
              {:else}
                <img
                  src={selectedMedia.src}
                  alt={`${selectedItem.title} 媒体预览`}
                  style={`object-position:${focalPercent(selectedMedia.focalPoint.x)} ${focalPercent(selectedMedia.focalPoint.y)}`}
                />
              {/if}
            {:else}
              <div class="media-missing" role="img" aria-label={`${selectedItem.title} 暂无媒体`}><span>NO MEDIA</span></div>
            {/if}
            <span>{itemNumber(selectedIndex)} / SELECTED</span>
          </div>
          <div class="preview-notes">
            <p>{selectedItem.description}</p>
            <button type="button" on:click={() => onOpen(selectedItem.id)}>查看完整档案 <span aria-hidden="true">→</span></button>
          </div>
        </aside>
      </div>
    </div>
  {:else if mode === "scene"}
    <div class="scene-view">
      <div class="scene-intro">
        <span>SCENE EXHIBITION / {moduleLabels[module]}</span>
        <h1>观看<br />而非浏览</h1>
        <p>以场景、人物与细节重组收藏。悬停同步当前条目，选择拼贴进入完整档案。</p>
      </div>

      <div class="scene-collage">
        {#each items as item, itemIndex}
          {@const assets = sceneMedia(item)}
          {#if assets.length > 0}
            {#each assets as asset, assetIndex}
              <button
                data-testid="content-item"
                data-content-id={item.id}
                type="button"
                class:selected={item.id === selectedItem?.id}
                data-concept-cursor="OPEN"
                class={`scene-tile scene-tile--${(itemIndex + assetIndex) % 7}`}
                style={`--media-color:${asset.dominantColor};--focal-x:${focalPercent(asset.focalPoint.x)};--focal-y:${focalPercent(asset.focalPoint.y)}`}
                on:mouseenter={() => onSelect(item.id)}
                on:focus={() => onSelect(item.id)}
                on:click={() => onOpen(item.id)}
                aria-label={`打开 ${item.title}，${asset.shotType} 场景`}
              >
                {#if asset.mediaType === "video"}
                  <video src={asset.src} poster={asset.placeholder} muted loop playsinline autoplay={!reducedMotion}></video>
                {:else}
                  <img src={asset.src} alt="" />
                {/if}
                <span class="scene-index">{itemNumber(itemIndex)}.{assetIndex + 1}</span>
                <span class="scene-label"><b>{item.title}</b><small>{asset.shotType}</small></span>
              </button>
            {/each}
          {:else}
            <button
              data-testid="content-item"
                data-content-id={item.id}
                type="button"
              class:selected={item.id === selectedItem?.id}
              data-concept-cursor="OPEN"
                class={`scene-tile scene-tile--${itemIndex % 7} scene-tile--empty`}
              style={`--media-color:#171918`}
              on:mouseenter={() => onSelect(item.id)}
              on:focus={() => onSelect(item.id)}
              on:click={() => onOpen(item.id)}
            >
              <span class="scene-index">{itemNumber(itemIndex)}.0</span>
              <span class="scene-label"><b>{item.title}</b><small>NO MEDIA</small></span>
            </button>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
</section>

<style>
  :global(body) {
    background: #090a0a;
  }

  .editorial {
    --ink: #f0efe9;
    --muted: #969991;
    --line: rgba(240, 239, 233, 0.22);
    --line-strong: rgba(240, 239, 233, 0.52);
    --accent: #d9ff43;
    min-height: 100dvh;
    height: 100dvh;
    overflow: auto;
    color: var(--ink);
    background:
      linear-gradient(90deg, transparent calc(25% - 0.5px), rgba(255,255,255,.045) 25%, transparent calc(25% + .5px)),
      linear-gradient(90deg, transparent calc(50% - 0.5px), rgba(255,255,255,.045) 50%, transparent calc(50% + .5px)),
      linear-gradient(90deg, transparent calc(75% - 0.5px), rgba(255,255,255,.045) 75%, transparent calc(75% + .5px)),
      #090a0a;
    font-family: "Outfit", "Microsoft YaHei", sans-serif;
  }

  button { color: inherit; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }

  .masthead {
    position: sticky;
    top: 0;
    z-index: 20;
    min-height: 74px;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    align-items: stretch;
    border-bottom: 1px solid var(--line-strong);
    background: rgba(9, 10, 10, 0.94);
    backdrop-filter: blur(14px);
  }

  .back, .identity, .mode-stamp { padding: 14px 22px; }
  .back {
    justify-self: start;
    display: flex;
    align-items: center;
    gap: 14px;
    border: 0;
    background: none;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: .12em;
    font-size: 11px;
  }
  .back span:first-child { font-size: 22px; transition: transform .25s ease; }
  .back:hover span:first-child { transform: translateX(-4px); }
  .identity { border-inline: 1px solid var(--line); display: flex; flex-direction: column; justify-content: center; }
  .identity span, .mode-stamp span { color: var(--muted); font: 10px/1.2 "JetBrains Mono", monospace; letter-spacing: .13em; }
  .identity strong { margin-top: 4px; font-size: 14px; letter-spacing: .08em; }
  .mode-stamp { display: grid; grid-template-columns: 1fr auto; align-content: center; gap: 3px 16px; text-align: right; }
  .mode-stamp strong { grid-row: 1 / 3; grid-column: 2; align-self: center; font-size: 18px; text-transform: uppercase; }

  .empty { min-height: calc(100dvh - 74px); display: grid; place-content: center; justify-items: start; padding: 7vw; }
  .empty span { color: var(--accent); font: 11px "JetBrains Mono", monospace; }
  .empty h1 { margin: 18px 0 38px; max-width: 8ch; font-size: clamp(54px, 10vw, 150px); line-height: .82; letter-spacing: -.07em; }
  .empty button { border: 1px solid var(--line-strong); padding: 12px 18px; background: transparent; cursor: pointer; }

  .visual-view { min-height: calc(100dvh - 74px); display: grid; grid-template-columns: minmax(300px, 5fr) minmax(420px, 7fr) 92px; }
  .visual-copy { display: flex; flex-direction: column; justify-content: flex-end; padding: clamp(24px, 4vw, 64px); border-right: 1px solid var(--line); }
  .eyebrow { margin: 0 0 auto; color: var(--accent); text-transform: uppercase; font: 11px/1 "JetBrains Mono", monospace; letter-spacing: .14em; }
  .title-trigger { padding: 0; border: 0; background: transparent; text-align: left; cursor: pointer; }
  .title-trigger > span { display: block; max-width: 8ch; font-size: clamp(58px, 8.5vw, 150px); font-weight: 700; line-height: .78; letter-spacing: -.075em; text-wrap: balance; }
  .title-trigger small { display: block; margin-top: 22px; color: var(--muted); font-size: clamp(16px, 1.5vw, 25px); font-weight: 400; }
  .description { max-width: 54ch; margin: 28px 0; color: #c0c1bb; font-size: 14px; line-height: 1.65; }
  .visual-meta { display: flex; flex-wrap: wrap; gap: 8px 20px; padding-top: 16px; border-top: 1px solid var(--line); color: var(--muted); font: 10px "JetBrains Mono", monospace; text-transform: uppercase; }

  .visual-media, .preview-frame, .scene-tile { position: relative; overflow: hidden; background: var(--media-color); }
  .visual-media img, .visual-media video, .preview-frame img, .preview-frame video, .scene-tile img, .scene-tile video { width: 100%; height: 100%; display: block; object-fit: cover; }
  .visual-media img, .visual-media video { transition: transform .8s cubic-bezier(.2,.8,.2,1), filter .5s ease; }
  .visual-media:hover img, .visual-media:hover video { transform: scale(1.018); }
  .visual-media::after, .preview-frame::after, .scene-tile::after { content: ""; position: absolute; inset: 0; pointer-events: none; background: linear-gradient(180deg, transparent 55%, rgba(0,0,0,.62)); }
  .media-caption { position: absolute; z-index: 2; right: 18px; bottom: 16px; font: 9px "JetBrains Mono", monospace; text-transform: uppercase; letter-spacing: .1em; }
  .media-missing { height: 100%; display: grid; place-items: center; background: repeating-linear-gradient(-45deg, transparent 0 16px, rgba(255,255,255,.035) 16px 17px); }
  .media-missing span { color: var(--muted); font: 10px "JetBrains Mono", monospace; letter-spacing: .16em; }
  .crop-mark { position: absolute; z-index: 3; left: 18px; width: 36px; height: 36px; border-left: 1px solid var(--ink); }
  .crop-mark--top { top: 18px; border-top: 1px solid var(--ink); }
  .crop-mark--bottom { bottom: 18px; border-bottom: 1px solid var(--ink); }

  .visual-rail { display: flex; flex-direction: column; overflow: auto; border-left: 1px solid var(--line); }
  .visual-rail button { min-height: 92px; display: grid; align-content: space-between; justify-items: start; padding: 12px 10px; border: 0; border-bottom: 1px solid var(--line); background: transparent; text-align: left; cursor: pointer; }
  .visual-rail button span:first-child, .visual-rail button span:last-child { color: var(--muted); font: 8px "JetBrains Mono", monospace; }
  .visual-rail button span:nth-child(2) { max-width: 8ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 10px; }
  .visual-rail button.active { background: var(--accent); color: #0a0b09; }
  .visual-rail button.active span { color: #0a0b09; }

  .index-view { min-height: calc(100dvh - 74px); padding: 20px 22px 38px; }
  .index-heading { display: grid; grid-template-columns: 1fr auto; align-items: end; min-height: clamp(118px, 18vh, 230px); border-bottom: 1px solid var(--line-strong); }
  .index-heading > div { display: flex; gap: 18px; padding-bottom: 14px; color: var(--muted); font: 10px "JetBrains Mono", monospace; text-transform: uppercase; }
  .index-heading strong { font-size: clamp(88px, 15vw, 220px); line-height: .66; letter-spacing: -.09em; }
  .index-body { display: grid; grid-template-columns: minmax(0, 8fr) minmax(260px, 3fr); gap: 22px; }
  .index-list { min-width: 0; }
  .column-labels, .index-row > button:first-child { display: grid; grid-template-columns: minmax(270px, 2.15fr) minmax(110px, .8fr) minmax(150px, 1fr) minmax(100px, .8fr); align-items: center; }
  .column-labels { min-height: 42px; border-bottom: 1px solid var(--line); color: var(--muted); font: 9px "JetBrains Mono", monospace; text-transform: uppercase; }
  .index-row { position: relative; display: grid; grid-template-columns: minmax(0, 1fr) 62px; border-bottom: 1px solid var(--line); transition: background-color .2s ease, color .2s ease; }
  .index-row > button:first-child { min-height: 83px; width: 100%; padding: 9px 0; border: 0; background: transparent; text-align: left; cursor: pointer; }
  .index-row.active { background: var(--ink); color: #0b0c0b; }
  .row-title { display: grid; grid-template-columns: 54px minmax(0, 1fr); align-items: center; min-width: 0; }
  .row-title > b { font: 11px "JetBrains Mono", monospace; }
  .row-title > span { min-width: 0; }
  .row-title strong, .row-title small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-title strong { font-size: clamp(18px, 2vw, 30px); letter-spacing: -.035em; }
  .row-title small { margin-top: 2px; color: var(--muted); font-size: 10px; font-weight: 400; }
  .row-status { display: flex; align-items: center; gap: 8px; font: 10px "JetBrains Mono", monospace; text-transform: uppercase; }
  .row-status i { width: 7px; height: 7px; background: var(--accent); }
  .row-progress { display: grid; gap: 8px; padding-right: 22px; }
  .row-progress > span { display: flex; justify-content: space-between; align-items: baseline; }
  .row-progress b { font: 18px "JetBrains Mono", monospace; }
  .row-progress small, .row-meta { color: var(--muted); font: 9px "JetBrains Mono", monospace; }
  .row-progress > i { display: block; height: 2px; background: rgba(128,128,128,.35); }
  .row-progress > i > i { display: block; height: 100%; background: var(--accent); }
  .row-meta { padding-right: 10px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .index-row.active .row-title small, .index-row.active .row-progress small, .index-row.active .row-meta { color: #555850; }
  .index-row.active .row-status i, .index-row.active .row-progress > i > i { background: #4f5d12; }
  .row-open { border: 0; border-left: 1px solid var(--line); background: transparent; cursor: pointer; display: grid; place-content: center; gap: 6px; font: 8px "JetBrains Mono", monospace; }
  .row-open span:last-child { font-size: 16px; }
  .index-row.active .row-open { border-color: rgba(9,10,10,.22); }

  .index-preview { position: sticky; top: 94px; align-self: start; padding-top: 42px; }
  .preview-frame { aspect-ratio: 4 / 5; }
  .preview-frame > span { position: absolute; z-index: 2; left: 13px; bottom: 12px; font: 9px "JetBrains Mono", monospace; letter-spacing: .1em; }
  .preview-notes { padding: 14px 0; border-bottom: 1px solid var(--line-strong); }
  .preview-notes p { margin: 0 0 16px; color: #b7b9b3; font-size: 12px; line-height: 1.55; }
  .preview-notes button { width: 100%; display: flex; justify-content: space-between; padding: 12px 0; border: 0; border-top: 1px solid var(--line); background: transparent; cursor: pointer; font-size: 11px; text-transform: uppercase; }

  .scene-view { padding: 0 22px 80px; }
  .scene-intro { min-height: min(68vh, 680px); display: grid; grid-template-columns: 1fr 2fr 1fr; align-items: end; gap: 24px; padding: 6vw 0 34px; border-bottom: 1px solid var(--line-strong); }
  .scene-intro > span { align-self: start; color: var(--accent); font: 10px "JetBrains Mono", monospace; text-transform: uppercase; }
  .scene-intro h1 { margin: 0; font-size: clamp(78px, 13vw, 190px); line-height: .72; letter-spacing: -.09em; }
  .scene-intro p { max-width: 30ch; margin: 0; color: var(--muted); font-size: 13px; line-height: 1.6; }
  .scene-collage { display: grid; grid-template-columns: repeat(12, minmax(0, 1fr)); grid-auto-rows: clamp(65px, 7vw, 110px); gap: 12px; padding-top: 12px; }
  .scene-tile { border: 0; padding: 0; cursor: pointer; text-align: left; grid-column: span 4; grid-row: span 4; }
  .scene-tile img, .scene-tile video { object-position: var(--focal-x) var(--focal-y); transition: transform .65s cubic-bezier(.2,.8,.2,1), filter .35s ease; }
  .scene-tile:hover img, .scene-tile:hover video, .scene-tile:focus-visible img, .scene-tile:focus-visible video { transform: scale(1.035); }
  .scene-tile:not(.selected) img, .scene-tile:not(.selected) video { filter: grayscale(.7) brightness(.72); }
  .scene-tile.selected { outline: 1px solid var(--accent); outline-offset: -1px; }
  .scene-tile--0 { grid-column: 1 / span 7; grid-row: span 6; }
  .scene-tile--1 { grid-column: 8 / span 5; grid-row: span 4; }
  .scene-tile--2 { grid-column: 8 / span 3; grid-row: span 5; }
  .scene-tile--3 { grid-column: span 5; grid-row: span 4; }
  .scene-tile--4 { grid-column: span 4; grid-row: span 6; }
  .scene-tile--5 { grid-column: span 7; grid-row: span 5; }
  .scene-tile--6 { grid-column: span 5; grid-row: span 7; }
  .scene-index, .scene-label { position: absolute; z-index: 2; }
  .scene-index { top: 12px; left: 12px; font: 9px "JetBrains Mono", monospace; }
  .scene-label { left: 12px; right: 12px; bottom: 12px; display: flex; justify-content: space-between; align-items: end; gap: 12px; }
  .scene-label b { font-size: clamp(16px, 2vw, 30px); letter-spacing: -.03em; }
  .scene-label small { font: 8px "JetBrains Mono", monospace; text-transform: uppercase; }
  .scene-tile--empty { background: repeating-linear-gradient(-45deg, var(--media-color) 0 18px, #20221f 18px 19px); }

  .reduced *, .reduced *::before, .reduced *::after { scroll-behavior: auto !important; animation-duration: .001ms !important; animation-iteration-count: 1 !important; transition-duration: .001ms !important; }

  @media (max-width: 980px) {
    .visual-view { grid-template-columns: minmax(260px, 4fr) minmax(340px, 6fr); }
    .visual-rail { grid-column: 1 / -1; height: 76px; flex-direction: row; border-top: 1px solid var(--line); border-left: 0; }
    .visual-rail button { min-width: 120px; min-height: 75px; border-right: 1px solid var(--line); }
    .index-body { grid-template-columns: 1fr; }
    .index-preview { position: relative; top: 0; display: grid; grid-template-columns: 220px 1fr; gap: 18px; padding-top: 20px; }
    .column-labels, .index-row > button:first-child { grid-template-columns: minmax(240px, 2fr) minmax(100px, .75fr) minmax(140px, 1fr); }
    .column-labels span:last-child, .row-meta { display: none; }
    .scene-intro { grid-template-columns: 1fr 2fr; }
    .scene-intro p { grid-column: 2; }
  }

  @media (max-width: 680px) {
    .editorial { background: #090a0a; }
    .masthead { min-height: 62px; grid-template-columns: auto 1fr auto; }
    .back, .identity, .mode-stamp { padding: 10px 12px; }
    .back span:last-child, .identity span, .mode-stamp span:first-child { display: none; }
    .identity { border-left: 1px solid var(--line); }
    .identity strong { font-size: 11px; }
    .mode-stamp { display: flex; align-items: center; gap: 8px; }
    .mode-stamp strong { font-size: 12px; }
    .mode-stamp span:last-child { display: block; }

    .visual-view { min-height: calc(100dvh - 62px); grid-template-columns: 1fr; grid-template-rows: minmax(46vh, 1fr) auto 72px; }
    .visual-media { grid-row: 1; }
    .visual-copy { grid-row: 2; padding: 22px 16px; border-top: 1px solid var(--line); border-right: 0; }
    .eyebrow { margin-bottom: 40px; }
    .title-trigger > span { font-size: clamp(54px, 18vw, 94px); }
    .description { margin-bottom: 16px; }
    .visual-rail { grid-row: 3; grid-column: 1; height: 72px; }
    .visual-rail button { min-height: 71px; }

    .index-view { padding: 12px 12px 28px; }
    .index-heading { min-height: 112px; }
    .index-heading > div { display: grid; gap: 5px; }
    .index-heading strong { font-size: 76px; }
    .column-labels { display: none; }
    .index-row { grid-template-columns: minmax(0, 1fr) 48px; }
    .index-row > button:first-child { min-height: 94px; grid-template-columns: 1fr 92px; gap: 10px; }
    .row-title { grid-template-columns: 36px minmax(0, 1fr); }
    .row-title strong { font-size: 20px; }
    .row-status { grid-column: 1; padding-left: 36px; }
    .row-progress { grid-column: 2; grid-row: 1 / 3; padding-right: 8px; }
    .row-meta { display: none; }
    .index-preview { grid-template-columns: 110px 1fr; }
    .preview-notes p { display: -webkit-box; overflow: hidden; -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3; }

    .scene-view { padding: 0 12px 50px; }
    .scene-intro { min-height: 58vh; grid-template-columns: 1fr; padding-top: 32px; }
    .scene-intro h1 { font-size: clamp(74px, 24vw, 120px); }
    .scene-intro p { grid-column: 1; }
    .scene-collage { grid-template-columns: repeat(2, minmax(0, 1fr)); grid-auto-rows: 95px; gap: 8px; }
    .scene-tile, .scene-tile--0, .scene-tile--1, .scene-tile--2, .scene-tile--3, .scene-tile--4, .scene-tile--5, .scene-tile--6 { grid-column: span 1; grid-row: span 3; }
    .scene-tile:nth-child(4n + 1) { grid-column: span 2; grid-row: span 4; }
  }

  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after { scroll-behavior: auto !important; animation-duration: .001ms !important; animation-iteration-count: 1 !important; transition-duration: .001ms !important; }
  }
</style>





