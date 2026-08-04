<script lang="ts">
  import Icon from "../Icon.svelte";
  import { sortSourcesByHealth, sourcesFor, type SourceHealth } from "../../services/sourceSwitch";

  interface Props {
    /** 内容类型：按此从适配层源健康 store 读取列表（如 'anime'） */
    contentType: string;
    contentId: string;
    chapterId?: string;
    positionSec?: number;
    onSelect: (sourceId: string) => void;
    onClose: () => void;
  }

  let { contentType, contentId, chapterId, positionSec, onSelect, onClose }: Props = $props();

  // spec §3.5：消费适配层暴露的源健康可读 store，响应式订阅（替代父组件注入的临时 sources）
  const sourceStore = $derived(sourcesFor(contentType));
  const ordered = $derived(sortSourcesByHealth($sourceStore));
  const healthLabel: Record<SourceHealth, string> = { ok: "可用", unknown: "未知", degraded: "异常" };
</script>

<div
  class="source-suggest"
  role="dialog"
  aria-modal="true"
  aria-labelledby="source-suggest-title"
  data-testid="source-suggest-sheet"
>
  <div class="source-suggest__card">
    <header class="source-suggest__header">
      <h2 id="source-suggest-title">推荐切换源</h2>
      <button type="button" class="source-suggest__close" aria-label="关闭源推荐" onclick={onClose}>
        <Icon name="x" size={16} />
      </button>
    </header>
    <p class="source-suggest__hint">当前源连续播放失败，以下源按健康度排序（可用优先）：</p>
    {#if ordered.length === 0}
      <p class="source-suggest__empty">暂无可推荐源，可稍后重试或检查规则更新。</p>
    {:else}
      <ul
        class="source-suggest__list"
        data-content-id={contentId}
        data-chapter-id={chapterId ?? ""}
        data-position-sec={positionSec ?? ""}
      >
        {#each ordered as source}
          <li>
            <button
              type="button"
              class="source-suggest__item"
              data-source-id={source.id}
              onclick={() => onSelect(source.id)}
            >
              <span class="source-suggest__name">{source.name}</span>
              <span class="source-suggest__health source-suggest__health--{source.health}">
                {healthLabel[source.health]}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .source-suggest {
    position: absolute;
    inset: 0;
    z-index: 71;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 6, 12, 0.78);
    backdrop-filter: blur(10px);
  }
  .source-suggest__card {
    width: min(460px, 100%);
    max-height: min(68vh, 30rem);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    background: rgba(10, 12, 18, 0.96);
    color: var(--text-primary, #f5f7fb);
  }
  .source-suggest__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }
  .source-suggest__header h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
  }
  .source-suggest__close {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-muted, #9aa3b2);
    cursor: pointer;
  }
  .source-suggest__close:hover {
    color: var(--text-primary, #f5f7fb);
  }
  .source-suggest__hint {
    margin: 0;
    padding: 10px 16px 4px;
    color: var(--text-muted, #9aa3b2);
    font-size: 12px;
    line-height: 1.5;
  }
  .source-suggest__list {
    margin: 0;
    padding: 8px 16px 14px;
    list-style: none;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .source-suggest__item {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-primary, #f5f7fb);
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .source-suggest__item:hover {
    border-color: rgba(232, 85, 127, 0.5);
    background: rgba(232, 85, 127, 0.1);
  }
  .source-suggest__name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .source-suggest__health {
    flex: 0 0 auto;
    padding: 3px 8px;
    border-radius: 12px;
    font: 600 10px/1 var(--font-ui, sans-serif);
  }
  .source-suggest__health--ok {
    background: rgba(74, 222, 128, 0.16);
    color: #4ade80;
  }
  .source-suggest__health--unknown {
    background: rgba(148, 163, 184, 0.16);
    color: #94a3b8;
  }
  .source-suggest__health--degraded {
    background: rgba(248, 113, 113, 0.16);
    color: #f87171;
  }
  .source-suggest__empty {
    margin: 0;
    padding: 20px 16px;
    color: var(--text-muted, #9aa3b2);
    font-size: 12px;
    text-align: center;
  }
</style>
