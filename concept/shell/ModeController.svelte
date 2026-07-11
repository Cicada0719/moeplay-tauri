<script lang="ts">
  import type { ContentMode } from "../contracts";

  export let mode: ContentMode;
  export let onChange: (mode: ContentMode) => void;

  const modes: Array<{ id: ContentMode; label: string; zh: string }> = [
    { id: "visual", label: "VISUAL", zh: "视觉" },
    { id: "index", label: "INDEX", zh: "索引" },
    { id: "scene", label: "SCENE", zh: "场景" },
  ];
</script>

<nav class="mode-controller" aria-label="内容模式" data-concept-shell="mode-controller" style={`--mode-index:${modes.findIndex((item) => item.id === mode)}`}>
  <span class="selection" aria-hidden="true"></span>
  {#each modes as item, index}
    <button type="button" class:active={mode === item.id} aria-pressed={mode === item.id} on:click={() => onChange(item.id)}>
      <span class="number">0{index + 1}</span><span>{item.label}</span><small>{item.zh}</small>
    </button>
  {/each}
</nav>

<style>
  .mode-controller{position:fixed;z-index:90;left:50%;bottom:1.35rem;transform:translateX(-50%);display:grid;grid-template-columns:repeat(3,minmax(7rem,1fr));width:min(31rem,calc(100vw - 2rem));padding:.28rem;background:color-mix(in srgb,#090909 88%,transparent);border:1px solid rgba(255,255,255,.18);backdrop-filter:blur(14px);color:#f4f1eb;isolation:isolate}
  .selection{position:absolute;z-index:-1;top:.28rem;bottom:.28rem;left:.28rem;width:calc((100% - .56rem)/3);background:#f1eee7;transform:translateX(calc(var(--mode-index)*100%));transition:transform 420ms cubic-bezier(.22,.86,.25,1),width 180ms ease}
  button{display:grid;grid-template-columns:auto 1fr;column-gap:.6rem;align-items:center;min-height:3.15rem;padding:.5rem .75rem;border:0;background:none;color:inherit;cursor:pointer;text-align:left;letter-spacing:.08em;transition:color 160ms ease}button.active{color:#111}.number{font-size:.57rem;opacity:.55;grid-row:1/3}button>span:not(.number){font-size:.68rem;font-weight:700}small{font-size:.56rem;opacity:.55}button:focus-visible{outline:1px solid currentColor;outline-offset:-.35rem}
  @media(prefers-reduced-motion:reduce){.selection{transition:none}}
  @media(max-width:560px){.mode-controller{bottom:.75rem;grid-template-columns:repeat(3,1fr)}button{grid-template-columns:1fr;text-align:center;padding:.45rem .2rem}.number,small{display:none}}
</style>
