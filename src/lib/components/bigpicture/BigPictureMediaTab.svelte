<script lang="ts">
  import { onMount } from "svelte";
  import { animeStore } from "../../stores/anime.svelte";
  import { comicStore } from "../../stores/comic.svelte";
  import Icon from "../Icon.svelte";
  import BPMediaRail from "../BPMediaRail.svelte";
  import { attachGamepad, type GamepadAttachment } from "../switch/useGamepad.svelte";

  interface MediaItem {
    id: string;
    title: string;
    cover: string | null;
    progress?: number;
    progressLabel?: string;
    type: "anime" | "comic";
  }

  let {
    active = false,
    onSelectMedia,
    onMoveToTop,
    onBack,
    onTabPrevious,
    onTabNext,
  }: {
    active?: boolean;
    onSelectMedia: (item: { type: string }) => void;
    onMoveToTop: () => void;
    onBack: () => void;
    onTabPrevious: () => void;
    onTabNext: () => void;
  } = $props();

  let rootEl = $state<HTMLDivElement>();
  let focusIdx = $state(0);
  let scope: GamepadAttachment | null = null;

  const continueAnime = $derived<MediaItem[]>(
    animeStore.history.filter((item) => item.lastEpisode > 0).slice(0, 10).map((item) => ({
      id: `anime-${item.key}`,
      title: item.name,
      cover: item.image ? animeStore.getImg(item.image) || item.image : null,
      progress: undefined,
      progressLabel: `第${item.lastEpisode}话`,
      type: "anime" as const,
    })),
  );

  const continueComics = $derived<MediaItem[]>(
    comicStore.readHistory.slice(0, 10).map((item) => ({
      id: `comic-${item.id || item.title}`,
      title: item.title,
      cover: null,
      progressLabel: item.last_title || undefined,
      type: "comic" as const,
    })),
  );

  const animeStart = $derived(0);
  const comicStart = $derived(continueAnime.length);
  const panelStart = $derived(continueAnime.length + continueComics.length);
  const itemCount = $derived(panelStart + 2);
  const continueCount = $derived(continueAnime.length + continueComics.length);

  const rows = $derived.by(() => {
    const result: number[][] = [];
    if (continueAnime.length) result.push(Array.from({ length: continueAnime.length }, (_, index) => animeStart + index));
    if (continueComics.length) result.push(Array.from({ length: continueComics.length }, (_, index) => comicStart + index));
    result.push([panelStart, panelStart + 1]);
    return result;
  });

  function rowPosition(index: number) {
    for (let row = 0; row < rows.length; row += 1) {
      const column = rows[row].indexOf(index);
      if (column >= 0) return { row, column };
    }
    return { row: rows.length - 1, column: 0 };
  }

  function setFocus(index: number) {
    focusIdx = Math.max(0, Math.min(itemCount - 1, index));
    if (!active) return;
    queueMicrotask(() => rootEl?.querySelector<HTMLElement>(`[data-media-index="${focusIdx}"]`)?.focus({ preventScroll: true }));
  }

  function moveHorizontal(delta: number) {
    const { row, column } = rowPosition(focusIdx);
    const rowItems = rows[row];
    setFocus(rowItems[Math.max(0, Math.min(rowItems.length - 1, column + delta))]);
  }

  function moveVertical(delta: number) {
    const { row, column } = rowPosition(focusIdx);
    const targetRow = row + delta;
    if (targetRow < 0) { onMoveToTop(); return; }
    if (targetRow >= rows.length) return;
    const target = rows[targetRow];
    setFocus(target[Math.min(column, target.length - 1)]);
  }

  function activateFocused() {
    rootEl?.querySelector<HTMLButtonElement>(`[data-media-index="${focusIdx}"]`)?.click();
  }

  function onMediaKeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowLeft": event.preventDefault(); moveHorizontal(-1); break;
      case "ArrowRight": event.preventDefault(); moveHorizontal(1); break;
      case "ArrowUp": event.preventDefault(); moveVertical(-1); break;
      case "ArrowDown": event.preventDefault(); moveVertical(1); break;
      case "Escape": event.preventDefault(); onBack(); break;
      case "Home": event.preventDefault(); setFocus(0); break;
      case "End": event.preventDefault(); setFocus(itemCount - 1); break;
    }
  }

  $effect(() => {
    if (focusIdx >= itemCount) focusIdx = Math.max(0, itemCount - 1);
    if (active) setFocus(focusIdx);
  });

  onMount(() => {
    scope = attachGamepad({
      left: () => moveHorizontal(-1),
      right: () => moveHorizontal(1),
      up: () => moveVertical(-1),
      down: () => moveVertical(1),
      launch: () => activateFocused(),
      activate: () => activateFocused(),
      back: () => onBack(),
      pageLeft: () => onTabPrevious(),
      pageRight: () => onTabNext(),
    }, { id: "big-picture-media", zone: "media", priority: 20 });
    return () => { scope?.(); scope = null; };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="bp-media"
  bind:this={rootEl}
  data-focus-zone="media"
  data-active={active ? "true" : "false"}
  onkeydown={onMediaKeydown}
  role="region"
  aria-label="媒体内容"
>
  <header class="bp-media-intro">
    <div>
      <span class="bp-media-kicker">MEDIA LOUNGE</span>
      <h1>继续你的故事</h1>
      <p>番剧和漫画统一汇聚到客厅界面，保持进度，随时从上次的位置继续。</p>
    </div>
    <div class="bp-media-summary" aria-label={`共有 ${continueCount} 个继续项目`}>
      <strong>{continueCount}</strong><span>待继续</span>
      <i></i>
      <strong>{animeStore.collection.length}</strong><span>追番</span>
      <i></i>
      <strong>{comicStore.favorites.length}</strong><span>漫画收藏</span>
    </div>
  </header>

  <div class="bp-media-scroll">
    {#if continueAnime.length > 0}
      <BPMediaRail
        title="继续观看"
        items={continueAnime}
        startIndex={animeStart}
        activeIndex={focusIdx}
        zoneActive={active}
        onfocusitem={setFocus}
        onselect={onSelectMedia}
      />
    {/if}
    {#if continueComics.length > 0}
      <BPMediaRail
        title="继续阅读"
        items={continueComics}
        startIndex={comicStart}
        activeIndex={focusIdx}
        zoneActive={active}
        onfocusitem={setFocus}
        onselect={onSelectMedia}
      />
    {/if}

    <section class="bp-media-spaces" aria-label="媒体入口">
      <div class="bp-media-section-heading">
        <span>探索空间</span>
        <small>选择一个内容世界</small>
      </div>
      <div class="bp-media-dual">
        <button
          class="bp-media-panel bp-media-anime"
          class:zone-focus={active && focusIdx === panelStart}
          data-media-index={panelStart}
          tabindex={active && focusIdx === panelStart ? 0 : -1}
          onclick={() => onSelectMedia({ type: "anime" })}
          onfocus={() => setFocus(panelStart)}
        >
          <div class="bp-media-panel-glow"></div>
          <div class="bp-media-panel-head">
            <span class="bp-media-panel-icon"><Icon name="film" size={28} /></span>
            <div><small>ANIME</small><h2>番剧空间</h2></div>
            <span class="bp-media-panel-badge">{animeStore.collection.length} 追番 · {animeStore.history.length} 历史</span>
          </div>
          <div class="bp-media-panel-body">
            {#if animeStore.recTrending.length > 0}
              <div class="bp-cover-rail">
                {#each animeStore.recTrending.slice(0, 8) as subject (subject.id)}
                  <div class="bp-cover-thumb">
                    {#if animeStore.getImg(subject.image)}<img src={animeStore.getImg(subject.image)} alt={subject.name_cn || subject.name} />
                    {:else}<div class="bp-cover-placeholder"><Icon name="film" size={20} /></div>{/if}
                    {#if subject.rating > 0}<span class="bp-cover-score">{subject.rating.toFixed(1)}</span>{/if}
                  </div>
                {/each}
              </div>
            {:else if animeStore.collection.length > 0}
              <div class="bp-cover-rail">
                {#each animeStore.collection.slice(0, 8) as item (item.key)}
                  <div class="bp-cover-thumb"><div class="bp-cover-placeholder"><Icon name="film" size={20} /></div></div>
                {/each}
              </div>
            {:else}<p class="bp-media-panel-hint">浏览番剧推荐、管理追番和观看记录。</p>{/if}
          </div>
          <div class="bp-media-panel-foot"><span>进入番剧</span><Icon name="arrowRight" size={18} /></div>
        </button>

        <button
          class="bp-media-panel bp-media-comic"
          class:zone-focus={active && focusIdx === panelStart + 1}
          data-media-index={panelStart + 1}
          tabindex={active && focusIdx === panelStart + 1 ? 0 : -1}
          onclick={() => onSelectMedia({ type: "comic" })}
          onfocus={() => setFocus(panelStart + 1)}
        >
          <div class="bp-media-panel-glow"></div>
          <div class="bp-media-panel-head">
            <span class="bp-media-panel-icon"><Icon name="book" size={28} /></span>
            <div><small>COMICS</small><h2>漫画空间</h2></div>
            {#if comicStore.isLoggedIn}<span class="bp-media-panel-badge">{comicStore.favorites.length} 收藏</span>{/if}
          </div>
          <div class="bp-media-panel-body">
            {#if comicStore.isLoggedIn && comicStore.favorites.length > 0}
              <div class="bp-cover-rail">
                {#each comicStore.favorites.slice(0, 8) as favorite (favorite.id)}
                  <div class="bp-cover-thumb">
                    {#if favorite.thumb_url}<img src={favorite.thumb_url} alt={favorite.title} />
                    {:else}<div class="bp-cover-placeholder"><Icon name="book" size={20} /></div>{/if}
                  </div>
                {/each}
              </div>
            {:else if comicStore.isLoggedIn}<p class="bp-media-panel-hint">浏览漫画分类、排行和你的收藏。</p>
            {:else}<p class="bp-media-panel-hint">登录漫画账号后，在大屏上继续阅读和管理收藏。</p>{/if}
          </div>
          <div class="bp-media-panel-foot"><span>{comicStore.isLoggedIn ? "进入漫画" : "前往登录"}</span><Icon name="arrowRight" size={18} /></div>
        </button>
      </div>
    </section>
  </div>
</div>

<style>
  .bp-media {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%; min-height: 0;
    padding: clamp(104px, 12vh, 142px) var(--bp-safe-x, 48px) 58px;
  }
  .bp-media-intro { display:flex; align-items:flex-end; justify-content:space-between; gap:32px; padding-bottom:clamp(18px,2.5vh,30px); }
  .bp-media-kicker { color:var(--scene-accent); font:850 9px var(--font-mono); letter-spacing:.24em; }
  .bp-media-intro h1 { max-width:10ch; margin:8px 0 6px; color:var(--scene-paper); font:900 clamp(42px,5.2vw,94px)/.9 var(--font-display); letter-spacing:-.065em; }
  .bp-media-intro h1::first-letter { color:var(--scene-accent); }
  .bp-media-intro p { max-width:54ch; margin:0; color:rgba(255,255,255,.52); font-size:clamp(10px,.74vw,13px); }
  .bp-media-summary { display:grid; grid-template-columns:auto auto 1px auto auto 1px auto auto; align-items:baseline; gap:8px; padding:11px 0; border-top:1px solid rgba(255,255,255,.18); border-bottom:1px solid rgba(255,255,255,.1); color:rgba(255,255,255,.45); }
  .bp-media-summary strong { color:var(--scene-paper); font:850 clamp(13px,1vw,18px) var(--font-mono); }
  .bp-media-summary span { font-size:8px; white-space:nowrap; }
  .bp-media-summary i { align-self:stretch; width:1px; background:rgba(255,255,255,.13); }

  .bp-media-scroll { min-height:0; overflow-x:hidden; overflow-y:auto; padding:4px 7px 24px; margin-inline:-7px; scrollbar-width:none; }
  .bp-media-scroll::-webkit-scrollbar { display:none; }
  .bp-media-spaces { margin-top:4px; }
  .bp-media-section-heading { display:flex; align-items:center; gap:12px; margin-bottom:10px; color:rgba(255,255,255,.5); }
  .bp-media-section-heading::before { content:""; width:32px; height:1px; background:var(--scene-accent); }
  .bp-media-section-heading span { color:var(--scene-paper); font:800 12px var(--font-ui); }
  .bp-media-section-heading small { font:750 8px var(--font-mono); letter-spacing:.15em; }
  .bp-media-dual { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:clamp(12px,1.2vw,22px); min-height:clamp(300px,48vh,520px); }

  .bp-media-panel {
    position:relative; isolation:isolate; display:grid; grid-template-rows:auto minmax(0,1fr) auto;
    min-width:0; overflow:hidden; padding:0; border:1px solid rgba(255,255,255,.14); border-radius:0;
    color:white; background:#0b0e16; cursor:pointer; text-align:left; outline:none;
    box-shadow:0 30px 100px -55px #000;
    transition:transform 360ms cubic-bezier(.2,.8,.2,1),border-color 220ms ease,box-shadow 220ms ease;
  }
  .bp-media-panel::before { position:absolute; right:-.03em; bottom:-.15em; z-index:-1; color:rgba(255,255,255,.055); font:950 clamp(90px,12vw,230px)/.8 var(--font-display); letter-spacing:-.08em; }
  .bp-media-anime::before { content:"映"; }
  .bp-media-comic::before { content:"読"; }
  .bp-media-panel::after { content:""; position:absolute; left:0; top:0; width:clamp(56px,5vw,96px); height:5px; background:var(--panel-accent); }
  .bp-media-anime { --panel-accent:#738dff; background:linear-gradient(125deg,rgba(41,54,112,.82),rgba(8,11,18,.92) 62%); }
  .bp-media-comic { --panel-accent:#ff72a4; background:linear-gradient(125deg,rgba(91,36,65,.78),rgba(8,11,18,.92) 62%); }
  .bp-media-panel-glow { position:absolute; inset:0; z-index:-2; opacity:.5; pointer-events:none; }
  .bp-media-anime .bp-media-panel-glow { background:radial-gradient(circle at 8% 4%,rgba(115,141,255,.5),transparent 43%); }
  .bp-media-comic .bp-media-panel-glow { background:radial-gradient(circle at 8% 4%,rgba(255,114,164,.45),transparent 43%); }
  .bp-media-panel:hover,.bp-media-panel:focus-visible,.bp-media-panel.zone-focus { transform:translateY(-5px); border-color:var(--panel-accent); outline:none !important; box-shadow:0 0 0 3px rgba(7,8,12,.92),0 0 0 6px var(--panel-accent),0 34px 90px -38px var(--panel-accent) !important; }

  .bp-media-panel-head { display:flex; align-items:center; gap:14px; padding:clamp(24px,2vw,36px) clamp(24px,2.2vw,42px) 0; }
  .bp-media-panel-icon { display:grid; place-items:center; width:clamp(48px,3.8vw,68px); aspect-ratio:1; border:1px solid rgba(255,255,255,.2); color:var(--panel-accent); background:rgba(7,8,12,.24); }
  .bp-media-panel-head>div { display:flex; flex-direction:column; gap:3px; }
  .bp-media-panel-head small { color:var(--panel-accent); font:850 8px var(--font-mono); letter-spacing:.22em; }
  .bp-media-panel-head h2 { margin:0; color:white; font:900 clamp(25px,2vw,39px)/1 var(--font-display); letter-spacing:-.04em; }
  .bp-media-panel-badge { margin-left:auto; color:rgba(255,255,255,.52); font:750 8px var(--font-mono); }
  .bp-media-panel-body { display:flex; align-items:center; min-height:0; padding:clamp(16px,1.5vw,26px) clamp(24px,2.2vw,42px); }
  .bp-media-panel-hint { max-width:36ch; margin:0; color:rgba(255,255,255,.58); font-size:clamp(11px,.78vw,14px); line-height:1.6; }
  .bp-cover-rail { display:flex; align-items:flex-end; gap:clamp(7px,.65vw,11px); width:100%; overflow:hidden; }
  .bp-cover-thumb { position:relative; flex:0 0 clamp(62px,5.4vw,104px); aspect-ratio:2/3; overflow:hidden; border:1px solid rgba(255,255,255,.12); border-radius:0; background:rgba(255,255,255,.045); box-shadow:0 14px 28px -18px #000; }
  .bp-cover-thumb:nth-child(even) { transform:translateY(9px); }
  .bp-cover-thumb img { width:100%; height:100%; object-fit:cover; display:block; }
  .bp-cover-placeholder { display:grid; place-items:center; width:100%; height:100%; color:rgba(255,255,255,.38); background:linear-gradient(145deg,rgba(255,255,255,.08),transparent); }
  .bp-cover-score { position:absolute; top:5px; right:5px; padding:3px 6px; color:#ffe084; background:rgba(0,0,0,.7); font:800 9px var(--font-mono); }
  .bp-media-panel-foot { display:flex; align-items:center; justify-content:space-between; padding:15px clamp(24px,2.2vw,42px); border-top:1px solid rgba(255,255,255,.1); color:rgba(255,255,255,.78); font:800 10px var(--font-mono); letter-spacing:.08em; }

  @media (max-width:1000px) { .bp-media-summary{display:none}.bp-media-dual{grid-template-columns:1fr}.bp-media-intro h1{font-size:clamp(34px,5vw,60px)} }
  @media (max-height:800px) {
    .bp-media { padding-top:78px; padding-bottom:44px; }
    .bp-media-intro { padding-bottom:12px; }
    .bp-media-intro p { display:none; }
    .bp-media-intro h1 { margin:3px 0 0; font-size:clamp(28px,3.8vw,48px); }
    .bp-media-dual { min-height:clamp(280px,48vh,360px); }
    .bp-media-panel-body { padding-block:10px; }
  }
  @media (prefers-reduced-motion:reduce) { .bp-media-panel{transition:none} }
  :global([data-motion="reduce"]) .bp-media-panel { transition:none; }
</style>
