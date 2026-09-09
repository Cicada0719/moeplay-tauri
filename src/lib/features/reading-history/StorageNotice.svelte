<script lang="ts">
  import { onMount } from "svelte";
  import { readingRepository } from "./repository";
  let error = $state(readingRepository.error);
  let retrying = $state(false);
  onMount(() => readingRepository.subscribe(() => { error = readingRepository.error; }));
  async function retry() {
    retrying = true;
    try { await readingRepository.retry(); } catch (reason) { error = String(reason); }
    finally { retrying = false; }
  }
</script>
{#if error}
  <aside role="alert" class="storage-notice">
    <span>{error}。旧版记录仍保留，可在统一历史中导出备份。</span>
    <button type="button" disabled={retrying} onclick={retry}>{retrying ? "正在重试…" : "重试存储"}</button>
  </aside>
{/if}
<style>
  .storage-notice { position:fixed; z-index:1200; bottom:max(5rem, env(safe-area-inset-bottom)); left:50%; transform:translateX(-50%); width:min(90vw, 42rem); display:flex; align-items:center; gap:1rem; padding:1rem; background:var(--bg-surface); color:var(--text-primary); border:1px solid var(--accent); border-radius:.75rem; box-shadow:0 8px 32px #0008; font-size:.875rem; }
  button { flex:none; padding:.65rem; background:var(--accent); color:var(--bg-deep); border:0; border-radius:.5rem; font:inherit; }
</style>
