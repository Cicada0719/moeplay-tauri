<script lang="ts">
  import type { ConceptTemplate } from "../contracts";
  import { TEMPLATE_IDS, TEMPLATE_META } from "../templates/registry";

  export let template: ConceptTemplate;
  export let onChange: (template: ConceptTemplate) => void;
  export let compact = false;
</script>

<div class:compact class="template-selector" role="group" aria-label="视觉模板" data-concept-shell="template-selector">
  {#each TEMPLATE_IDS as id}
    {@const meta = TEMPLATE_META[id]}
    <button type="button" class:active={template === id} aria-pressed={template === id} on:click={() => onChange(id)}>
      <span class="number">{meta.number}</span>
      <span class="name">{meta.label}</span>
      <small>{meta.zh}</small>
    </button>
  {/each}
</div>

<style>
  .template-selector{position:fixed;z-index:85;left:1.5rem;bottom:1.35rem;display:flex;align-items:flex-end;gap:.25rem;color:#f5f2ec}.template-selector button{display:grid;grid-template-columns:auto 1fr;column-gap:.55rem;min-width:9rem;padding:.65rem .75rem;background:rgba(7,7,7,.72);border:1px solid rgba(255,255,255,.16);color:inherit;cursor:pointer;text-align:left;backdrop-filter:blur(12px);opacity:.52;transition:opacity 180ms ease,background 180ms ease,color 180ms ease}.template-selector button:hover,.template-selector button.active{opacity:1}.template-selector button.active{background:#f2efe8;color:#111}.number{grid-row:1/3;font-size:.58rem;opacity:.55}.name{font-size:.63rem;font-weight:700;letter-spacing:.09em}small{font-size:.55rem;opacity:.58}.template-selector button:focus-visible{outline:1px solid currentColor;outline-offset:.2rem}
  .template-selector.compact button{min-width:auto}.compact .name,.compact small{display:none}.compact .number{grid-row:auto}
  @media(max-width:900px){.template-selector{left:1rem;bottom:5.25rem}.template-selector button{min-width:auto}.name,small{display:none}.number{grid-row:auto}}
</style>
