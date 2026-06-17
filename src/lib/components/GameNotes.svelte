<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    gameId,
  }: {
    gameId: string;
  } = $props();

  let notes = $state("");
  let saving = $state(false);
  let lastSaved = $state("");
  let saveTimer = $state<ReturnType<typeof setTimeout>>();

  const STORAGE_KEY = "moeplay-game-notes";

  function loadNotes() {
    try {
      const all: Record<string, string> = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
      notes = all[gameId] || "";
      lastSaved = notes;
    } catch { notes = ""; }
  }

  function saveNotes() {
    if (notes === lastSaved) return;
    saving = true;
    try {
      const all: Record<string, string> = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
      if (notes.trim()) {
        all[gameId] = notes;
      } else {
        delete all[gameId];
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
      lastSaved = notes;
    } catch {}
    setTimeout(() => saving = false, 500);
  }

  function onInput() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(saveNotes, 800);
  }

  $effect(() => {
    if (gameId) loadNotes();
  });
</script>

<div class="game-notes">
  <div class="gn-header">
    <div class="gn-title">
      <Icon name="paperclip" size={14} />
      <h4>个人笔记</h4>
    </div>
    {#if saving}
      <span class="gn-saving">保存中...</span>
    {/if}
  </div>
  <textarea
    class="gn-textarea"
    bind:value={notes}
    oninput={onInput}
    onblur={saveNotes}
    placeholder="记录你的游戏感想、攻略备忘..."
    rows="4"
  ></textarea>
</div>

<style>
  .game-notes {
    background: var(--bg-elev, rgba(255,255,255,0.03));
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 14px;
  }
  .gn-header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 10px;
  }
  .gn-title { display: flex; align-items: center; gap: 8px; }
  .gn-title h4 { margin: 0; font-size: 14px; font-weight: 700; color: var(--text-primary); }
  .gn-saving { font-size: 11px; color: var(--text-muted); }
  .gn-textarea {
    width: 100%; min-height: 80px; resize: vertical;
    padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--bg-card, #12151e); color: var(--text-primary);
    font-size: 13px; line-height: 1.6; font-family: inherit;
    transition: border-color 0.15s;
  }
  .gn-textarea:focus { outline: none; border-color: var(--accent); }
  .gn-textarea::placeholder { color: var(--text-muted); }
</style>
