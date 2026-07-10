<script lang="ts">
  import { getAiClient, type AiClient } from "../../features/ai";
  import AiStatusCenter from "./AiStatusCenter.svelte";
  import NaturalLanguageFilterCompiler from "./NaturalLanguageFilterCompiler.svelte";
  import LibraryCleanupPreview from "./LibraryCleanupPreview.svelte";
  import type { NaturalLanguageFilterDsl } from "../../features/ai/types";

  let {
    client = getAiClient(),
    onApplyFilter,
  }: {
    client?: AiClient;
    onApplyFilter?: (dsl: NaturalLanguageFilterDsl) => void;
  } = $props();
</script>

<div class="workbench">
  <section class="workbench-panel"><AiStatusCenter {client} /></section>
  <div class="experience-grid">
    <section class="workbench-panel"><NaturalLanguageFilterCompiler {client} onApply={onApplyFilter} /></section>
    <section class="workbench-panel"><LibraryCleanupPreview {client} /></section>
  </div>
</div>

<style>
  .workbench { display: grid; gap: 14px; }
  .experience-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; align-items: start; }
  .workbench-panel { min-width: 0; padding: 18px; border: 1px solid var(--border); border-radius: 9px; background: var(--bg-card); box-shadow: var(--shadow-xs); }
  @media (max-width: 1080px) { .experience-grid { grid-template-columns: 1fr; } }
  @media (max-width: 560px) { .workbench-panel { padding: 14px; } }
</style>
