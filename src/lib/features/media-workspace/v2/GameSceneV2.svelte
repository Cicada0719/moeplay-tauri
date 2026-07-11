<script lang="ts">
  import "../styles/media-workspace.css";
  import MediaArtwork from "../components/MediaArtwork.svelte";
  import { findAction, runAction, sceneEntries, statusLabel } from "../components/viewHelpers";
  import type { MediaPresentationItem, MediaWorkspaceViewActions } from "../components/types";

  interface Props extends MediaWorkspaceViewActions { items?: readonly MediaPresentationItem[]; selectedId?: string | null; }
  let { items = [], selectedId = null, onAction, onImport }: Props = $props();
  const entries = $derived(sceneEntries(items, selectedId, 9));
  const selected = $derived(items.find((item) => item.id === selectedId) ?? entries[0]?.item ?? null);
  const launch = $derived(selected ? findAction(selected, "launch") : undefined);
</script>

<section class="mw-v2-scene" aria-labelledby="mw-v2-scene-title">
  <header class="mw-v2-scene__header">
    <div><p class="mw-v2-kicker">SCENE DIRECTORY / LIVE MEMORY</p><h1 id="mw-v2-scene-title">场景流</h1></div>
    {#if selected}<div><span>FOCUS</span><strong>{selected.title}</strong><small>{statusLabel(selected.metadata.completionStatus)}</small></div>{/if}
  </header>

  {#if entries.length}
    <div class="mw-v2-scene__stream">
      {#each entries as entry, index (entry.id)}
        <article class={`mw-v2-scene__item mw-v2-scene__item--${(index % 6) + 1}`} class:active={entry.item.id === selected?.id}>
          <button onclick={() => runAction(entry.item, "select", onAction)} ondblclick={() => runAction(entry.item, "open", onAction)} aria-label={`选择 ${entry.item.title}`}>
            <MediaArtwork src={entry.asset?.src} alt={entry.asset?.alt || ""} title={entry.item.title} eager={index < 3} />
            <span class="mw-v2-scene__index">{String(index + 1).padStart(2, "0")}</span>
            <span class="mw-v2-scene__caption"><strong>{entry.item.title}</strong><small>{entry.asset?.role || "archive"}</small></span>
          </button>
        </article>
      {/each}
    </div>
    {#if selected}
      <footer class="mw-v2-scene__footer"><p>{selected.description || "从媒体切片、游玩状态与档案信息重新进入这部作品。"}</p>{#if launch}<button class="mw-v2-action mw-v2-action--accent" onclick={() => runAction(selected, "launch", onAction)}><span>{launch.label}</span><i aria-hidden="true"></i></button>{/if}<button class="mw-v2-action" onclick={() => runAction(selected, "open", onAction)}>查看详情</button></footer>
    {/if}
  {:else}
    <div class="mw-v2-empty"><span>SCENE 000</span><h1 id="mw-v2-scene-title">还没有可编排的场景</h1><p>补充封面与截图后，这里会形成连续、可滚动的媒体目录。</p>{#if onImport}<button class="mw-v2-action mw-v2-action--accent" onclick={() => void onImport?.()}><span>导入游戏</span><i aria-hidden="true"></i></button>{/if}</div>
  {/if}
</section>
