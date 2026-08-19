<script lang="ts">
  import { paletteStore } from "./store.svelte";
  import { rankPaletteEntries, type PaletteEntry } from "./matcher";
  import { gameStore } from "../../stores/games.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { navigateTo } from "../../stores/router.svelte";
  import { developerOf, originalNameOf } from "../../utils/game";
  import Icon from "../../components/Icon.svelte";

  const NAV_VIEWS: Array<{ id: string; label: string; view: string; hint: string; aliases: string[] }> = [
    { id: "nav-home", label: "游戏库", view: "home", hint: "前往游戏主页", aliases: ["主页", "library"] },
    { id: "nav-anime", label: "番剧", view: "anime", hint: "前往番剧主页", aliases: ["动画", "anime"] },
    { id: "nav-comic", label: "漫画", view: "comic", hint: "前往漫画主页", aliases: ["manga"] },
    { id: "nav-novel", label: "小说", view: "novel", hint: "前往小说主页", aliases: ["novel"] },
    { id: "nav-continue", label: "今日继续", view: "continue", hint: "跨媒体最近进度", aliases: ["continue"] },
    { id: "nav-tasks", label: "任务中心", view: "tasks", hint: "下载与后台任务", aliases: ["任务", "tasks"] },
    { id: "nav-settings", label: "设置", view: "settings", hint: "应用偏好与主题", aliases: ["settings"] },
  ];

  const ACTION_ENTRIES: PaletteEntry[] = [
    ...NAV_VIEWS.map((nav) => ({
      id: nav.id,
      label: nav.label,
      hint: nav.hint,
      aliases: nav.aliases,
      group: "导航",
      run: () => { navigateTo(nav.view); },
    })),
    {
      id: "big-picture",
      label: "大屏模式",
      hint: "进入沉浸式大屏（手柄/键盘导航）",
      aliases: ["大屏", "big picture", "tv"],
      group: "动作",
      run: () => { uiStore.setBigPicture(true); },
    },
  ];

  let query = $state("");
  let activeIndex = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  const gameEntries = $derived.by<PaletteEntry[]>(() =>
    gameStore.allGames.map((game) => {
      const original = originalNameOf(game);
      const developer = developerOf(game);
      const rawName = game.name || "未命名游戏";
      return {
        id: `game:${game.id}`,
        label: rawName,
        hint: [developer, original && original !== rawName ? original : ""].filter(Boolean).join(" · ") || undefined,
        aliases: original && original !== rawName ? [original] : [],
        group: "游戏",
        run: () => {
          gameStore.selectGame(game.id);
          navigateTo("game-detail", { entity: { kind: "game", id: game.id }, focus: "start" });
        },
      };
    }),
  );

  const results = $derived(rankPaletteEntries<PaletteEntry>(query, [...ACTION_ENTRIES, ...gameEntries]));

  function close() { paletteStore.close(); }

  function runEntry(entry: PaletteEntry) {
    paletteStore.close();
    void entry.run();
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
      return;
    }
    if (results.length === 0) return;
    if (event.key === "ArrowDown") { event.preventDefault(); activeIndex = Math.min(activeIndex + 1, results.length - 1); return; }
    if (event.key === "ArrowUp") { event.preventDefault(); activeIndex = Math.max(activeIndex - 1, 0); return; }
    if (event.key === "Enter") {
      event.preventDefault();
      const hit = results[activeIndex];
      if (hit) runEntry(hit.entry);
    }
  }

  $effect(() => {
    if (paletteStore.open) {
      query = "";
      activeIndex = 0;
      queueMicrotask(() => inputEl?.focus());
    }
  });
</script>

{#if paletteStore.open && !uiStore.bigPictureActive}
  <div class="palette-backdrop" role="presentation" onmousedown={close}>
    <div class="palette-panel" role="dialog" aria-modal="true" aria-label="命令面板" tabindex="-1" onclick={(event) => event.stopPropagation()} onkeydown={(event) => { if (event.key === "Escape") close(); }}>
      <div class="palette-field">
        <Icon name="search" size={16} />
        <input
          bind:this={inputEl}
          type="search"
          value={query}
          oninput={(event) => { query = event.currentTarget.value; activeIndex = 0; }}
          onkeydown={onInputKeydown}
          placeholder="搜索游戏或命令… (Esc 关闭)"
          aria-label="搜索游戏或命令"
          autocomplete="off"
          spellcheck="false"
        />
        <button class="palette-close" type="button" onclick={close} aria-label="关闭命令面板"><Icon name="x" size={14} /></button>
      </div>
      {#if results.length > 0}
        <ul class="palette-results" role="listbox" aria-label="匹配结果">
          {#each results as result, index (result.entry.id)}
            <li>
              <button
                type="button"
                role="option"
                aria-selected={index === activeIndex}
                class:active={index === activeIndex}
                onmouseenter={() => (activeIndex = index)}
                onclick={() => runEntry(result.entry)}
              >
                <span class="palette-group">{result.entry.group ?? "命令"}</span>
                <span class="palette-label"><strong>{result.entry.label}</strong>{#if result.entry.hint}<small>{result.entry.hint}</small>{/if}</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else if query.trim()}
        <p class="palette-empty" role="status">没有匹配结果</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .palette-backdrop { position: fixed; inset: 0; z-index: 200; display: flex; align-items: flex-start; justify-content: center; padding-top: 12vh; background: rgba(4,6,10,.55); backdrop-filter: blur(3px); }
  .palette-panel { width: min(34rem, 92vw); overflow: hidden; border: 1px solid var(--border, rgba(255,255,255,.1)); border-radius: var(--radius-lg, 16px); background: var(--bg-elev, #161b27); box-shadow: 0 1.2rem 3rem rgba(0,0,0,.5); }
  .palette-field { display: grid; grid-template-columns: auto minmax(0,1fr) auto; align-items: center; gap: .6rem; padding: .85rem 1rem; border-bottom: 1px solid rgba(255,255,255,.08); color: var(--text-muted); }
  .palette-field input { width: 100%; min-width: 0; border: 0; background: transparent; color: var(--text-primary); font: 1rem/1.3 var(--font-ui); outline: none; }
  .palette-close { display: grid; width: 2rem; height: 2rem; place-items: center; border: 0; border-radius: 8px; background: transparent; color: var(--text-muted); cursor: pointer; }
  .palette-close:hover { background: rgba(255,255,255,.06); }
  .palette-close:focus-visible, .palette-panel:focus-within { outline: none; }
  .palette-panel:focus-within { box-shadow: 0 1.2rem 3rem rgba(0,0,0,.5), 0 0 0 1px rgba(232,85,127,.35); }
  .palette-results { max-height: min(46vh, 26rem); margin: 0; padding: .4rem; overflow-y: auto; list-style: none; }
  .palette-results li { list-style: none; }
  .palette-results button { width: 100%; display: grid; grid-template-columns: 4.5rem minmax(0,1fr); align-items: center; gap: .6rem; min-height: 3rem; padding: .45rem .6rem; border: 0; border-radius: 10px; background: transparent; color: var(--text-primary); text-align: left; cursor: pointer; }
  .palette-results button.active, .palette-results button:focus-visible { background: rgba(255,255,255,.07); outline: none; box-shadow: inset 0 0 0 1px rgba(255,255,255,.14); }
  .palette-group { overflow: hidden; color: var(--text-muted); font: 650 .62rem/1 var(--font-mono); letter-spacing: .08em; text-overflow: ellipsis; white-space: nowrap; }
  .palette-label { min-width: 0; display: grid; gap: .15rem; }
  .palette-label strong { overflow: hidden; font-size: .92rem; text-overflow: ellipsis; white-space: nowrap; }
  .palette-label small { overflow: hidden; color: var(--text-muted); font-size: .72rem; text-overflow: ellipsis; white-space: nowrap; }
  .palette-empty { margin: 0; padding: 1.2rem; color: var(--text-muted); font-size: .84rem; text-align: center; }
</style>
