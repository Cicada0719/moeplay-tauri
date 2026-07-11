<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "../../../components/Icon.svelte";
  import MediaModeSwitcher from "../components/MediaModeSwitcher.svelte";
  import type { ContentMode } from "../model";

  interface Props {
    mode: ContentMode;
    searching?: boolean;
    count?: number;
    searchValue?: string;
    searchInput?: HTMLInputElement;
    healthLegacy?: boolean;
    healthOpen?: boolean;
    content?: Snippet;
    onModeChange?: (mode: ContentMode) => void;
    onSearchInput?: (value: string) => void;
    onClearSearch?: () => void;
    onOpenHealth?: () => void;
    onImport?: () => void;
  }

  let {
    mode,
    searching = false,
    count = 0,
    searchValue = "",
    searchInput = $bindable<HTMLInputElement>(),
    healthLegacy = false,
    healthOpen = false,
    content,
    onModeChange,
    onSearchInput,
    onClearSearch,
    onOpenHealth,
    onImport,
  }: Props = $props();
</script>

<div class="mw-shell" data-mode={mode} data-module-style="cinematic" data-testid="switch-home">
  <header class="mw-shell__topline">
    <div class="mw-shell__module"><span>01 / CINEMATIC</span><strong>游戏档案</strong><small>{count} TITLES</small></div>
    <label class="mw-shell__search" for="library-search">
      <Icon name="search" size={15} />
      <input id="library-search" bind:this={searchInput} type="search" value={searchValue} oninput={(event) => onSearchInput?.(event.currentTarget.value)} placeholder="在游戏档案中搜索" aria-label="搜索游戏库" data-search-scope="home" data-route-search />
      {#if searching}<button type="button" onclick={onClearSearch} aria-label="清空搜索"><Icon name="x" size={13} /></button>{/if}
    </label>
    <div class="mw-shell__utilities">
      <button type="button" class:legacy={healthLegacy} aria-haspopup="dialog" aria-expanded={healthOpen} onclick={onOpenHealth}><Icon name="database" size={15}/><span>库健康</span></button>
      <button type="button" onclick={onImport} aria-label="添加游戏"><Icon name="plus" size={16}/><span>导入</span></button>
    </div>
  </header>

  <main class="mw-shell__stage">{@render content?.()}</main>

  <footer class="mw-shell__director">
    <div class="mw-shell__style-id" aria-label="游戏模块风格">
      <span>01 / CINEMATIC</span><strong>电影档案</strong><small>游戏模块固定导演语言</small>
    </div>
    <MediaModeSwitcher {mode} onModeChange={onModeChange} label="游戏库显示模式" />
    <div class="mw-shell__hint"><span>CINEMATIC / {mode.toUpperCase()}</span><strong>{mode === "index" ? "原生浏览" : "滚轮切换作品"}</strong></div>
  </footer>
</div>

<style>
  .mw-shell { position: relative; width: 100%; height: 100%; min-height: 0; display: grid; grid-template-rows: 58px minmax(0, 1fr) 78px; overflow: hidden; isolation: isolate; background: rgb(5 5 5 / .68); color: var(--text-primary); }
  .mw-shell::before { content: ""; position: absolute; inset: 0; z-index: -1; pointer-events: none; background: linear-gradient(115deg, rgb(var(--media-primary-rgb, 70 76 88) / .12), transparent 46%), radial-gradient(circle at 72% 34%, rgb(var(--media-secondary-rgb, 70 76 88) / .1), transparent 44%); }
  .mw-shell__topline { display: grid; grid-template-columns: minmax(210px, .45fr) minmax(260px, 1fr) auto; align-items: stretch; border-top: 1px solid rgb(255 255 255 / .12); border-bottom: 1px solid rgb(255 255 255 / .13); background: rgb(5 5 5 / .88); backdrop-filter: blur(18px); }
  .mw-shell__module { display: flex; align-items: center; gap: 11px; padding: 0 16px; border-right: 1px solid rgb(255 255 255 / .12); }
  .mw-shell__module { min-width: 0; }
  .mw-shell__module span, .mw-shell__module small { display: block; font-family: "JetBrains Mono", monospace; }
  .mw-shell__module span { color: rgb(var(--media-accent-rgb, 199 71 47)); font-size: 9px; }
  .mw-shell__module strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .mw-shell__module small { margin-left: auto; color: var(--text-muted); font-size: 8px; }
  .mw-shell__search { min-width: 0; display: flex; align-items: center; gap: 10px; margin: 9px 14px; padding: 0 13px; border: 1px solid rgb(255 255 255 / .12); background: rgb(255 255 255 / .025); }
  .mw-shell__search:focus-within { border-color: rgb(var(--media-accent-rgb, 199 71 47) / .65); box-shadow: inset 0 -1px rgb(var(--media-accent-rgb, 199 71 47)); }
  .mw-shell__search input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: var(--text-primary); font: 12px/1 var(--font-ui); }
  .mw-shell__search button { display: grid; border: 0; background: transparent; color: var(--text-muted); cursor: pointer; }
  .mw-shell__utilities { display: flex; border-left: 1px solid rgb(255 255 255 / .12); }
  .mw-shell__utilities button { min-width: 48px; display: inline-flex; align-items: center; justify-content: center; gap: 7px; padding: 0 13px; border: 0; border-left: 1px solid rgb(255 255 255 / .1); background: transparent; color: var(--text-secondary); font: 600 10px/1 var(--font-ui); cursor: pointer; }
  .mw-shell__utilities button:hover { color: var(--text-primary); background: rgb(255 255 255 / .04); }
  .mw-shell__utilities button.legacy { color: var(--text-muted); }
  .mw-shell__stage { min-height: 0; overflow: hidden; }
  .mw-shell__director { z-index: 8; display: grid; grid-template-columns: minmax(250px, 1fr) minmax(420px, 580px) minmax(170px, 1fr); align-items: stretch; border-top: 1px solid rgb(255 255 255 / .14); background: rgb(5 5 5 / .93); backdrop-filter: blur(20px); }
  .mw-shell__style-id { display: grid; grid-template-columns: auto 1fr; grid-template-rows: auto auto; align-content: center; gap: 4px 11px; padding: 0 20px; border-right: 1px solid rgb(255 255 255 / .12); }
  .mw-shell__style-id span { grid-row: 1 / 3; align-self: center; color: var(--c-accent, #c7472f); font: 600 9px/1 var(--font-mono); letter-spacing: .1em; }
  .mw-shell__style-id strong { font-size: 11px; letter-spacing: .08em; }
  .mw-shell__style-id small { color: var(--text-muted); font: 500 7px/1 var(--font-mono); letter-spacing: .08em; }
  .mw-shell__director :global(.mw-mode-switcher) { width: 100%; max-width: none; height: 100%; border-top: 0; border-bottom: 0; }
  .mw-shell__hint { display: grid; align-content: center; gap: 6px; padding: 0 20px; }
  .mw-shell__hint span { color: var(--text-muted); font: 600 8px/1 "JetBrains Mono", monospace; letter-spacing: .16em; }
  .mw-shell__hint strong { font: 600 10px/1 "JetBrains Mono", monospace; letter-spacing: .07em; }
  .mw-shell__hint { text-align: right; }
  @media (max-width: 980px) { .mw-shell__topline { grid-template-columns: minmax(180px, .45fr) minmax(180px, 1fr) auto; } .mw-shell__director { grid-template-columns: minmax(230px, .8fr) minmax(390px, 1.4fr); } .mw-shell__hint { display: none; } }
  @media (max-width: 700px) { .mw-shell { grid-template-rows: auto minmax(0, 1fr) 68px; } .mw-shell__topline { grid-template-columns: 1fr auto; min-height: 58px; } .mw-shell__module { border-right: 0; } .mw-shell__search { grid-column: 1 / -1; grid-row: 2; min-height: 38px; } .mw-shell__utilities button span { display: none; } .mw-shell__director { grid-template-columns: 1fr; } .mw-shell__style-id, .mw-shell__hint { display: none; } }
</style>
