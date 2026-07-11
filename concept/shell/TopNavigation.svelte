<script lang="ts">
  import type { ContentModule } from "../contracts";

  export let module: ContentModule;
  export let toneClass = "concept-nav--light-ink";
  export let title = "MOEPLAY";
  export let statusLabel = "CONCEPT / ONLINE";
  export let onModuleChange: (module: ContentModule) => void;
  export let onSearch: () => void = () => undefined;
  export let onStatus: () => void = () => undefined;
  export let onSettings: () => void = () => undefined;

  const modules: Array<{ id: ContentModule; label: string; zh: string }> = [
    { id: "games", label: "GAMES", zh: "游戏" },
    { id: "anime", label: "ANIME", zh: "番剧" },
    { id: "comics", label: "COMICS", zh: "漫画" },
  ];
</script>

<header class={`concept-top-nav ${toneClass}`} data-concept-shell="top-navigation">
  <button class="brand" type="button" on:click={() => onModuleChange(module)} aria-label="返回当前模块顶部">
    <span>{title}</span><small>{statusLabel}</small>
  </button>

  <nav class="module-nav" aria-label="媒体模块">
    {#each modules as item}
      <button
        type="button"
        class:active={module === item.id}
        aria-current={module === item.id ? "page" : undefined}
        on:click={() => onModuleChange(item.id)}
      >
        <span>{item.label}</span><small>{item.zh}</small>
      </button>
    {/each}
  </nav>

  <div class="utility-nav">
    <button type="button" on:click={onSearch}><span>SEARCH</span><small>搜索</small></button>
    <button type="button" on:click={onStatus}><span>STATUS</span><small>状态</small></button>
    <button type="button" on:click={onSettings}><span>SETTINGS</span><small>设置</small></button>
  </div>
</header>

<style>
  .concept-top-nav{position:fixed;z-index:80;inset:0 0 auto;display:grid;grid-template-columns:minmax(12rem,1fr) auto minmax(12rem,1fr);align-items:start;padding:1.25rem 1.5rem;pointer-events:none;color:var(--concept-nav-ink,#f6f3ed);mix-blend-mode:normal}
  .concept-top-nav.concept-nav--adaptive{mix-blend-mode:difference;--concept-nav-ink:#fff}.concept-top-nav.concept-nav--dark-ink{--concept-nav-ink:#111;}.concept-top-nav.concept-nav--light-ink{--concept-nav-ink:#f6f3ed}
  button{pointer-events:auto;color:inherit;background:none;border:0;padding:0;text-align:left;cursor:pointer;text-transform:uppercase;letter-spacing:.08em}
  button:focus-visible{outline:1px solid currentColor;outline-offset:.45rem}
  button span{display:block;font-size:.72rem;font-weight:650}button small{display:block;margin-top:.2rem;font-size:.58rem;opacity:.58;letter-spacing:.04em}
  .brand{justify-self:start}.brand span{font-size:.86rem;letter-spacing:.16em}.module-nav{display:flex;gap:clamp(1.25rem,3vw,3.75rem)}.module-nav button{position:relative;opacity:.48;transition:opacity 180ms ease}.module-nav button::after{content:"";position:absolute;left:0;right:100%;bottom:-.55rem;height:1px;background:currentColor;transition:right 240ms ease}.module-nav button:hover,.module-nav button.active{opacity:1}.module-nav button.active::after{right:0}.utility-nav{justify-self:end;display:flex;gap:1.3rem}.utility-nav button{opacity:.62}.utility-nav button:hover{opacity:1}
  @media(max-width:760px){.concept-top-nav{grid-template-columns:1fr auto;padding:1rem}.module-nav{position:fixed;left:1rem;right:1rem;top:4rem;justify-content:space-between;border-top:1px solid color-mix(in srgb,currentColor 26%,transparent);padding-top:.65rem}.utility-nav button:not(:last-child){display:none}.utility-nav small,.brand small{display:none}}
</style>
