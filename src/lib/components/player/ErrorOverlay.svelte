<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import { messageForErrorKind } from "../../player/errorMap";
  import type { PlayerError } from "../../stores/player";

  interface Props {
    error: PlayerError;
    retryCount: number;
    onRetry: () => void;
    onSwitchSource: () => void;
    onCopyLog: () => void;
  }

  let { error, retryCount, onRetry, onSwitchSource, onCopyLog }: Props = $props();

  let switchBtn = $state<HTMLButtonElement | null>(null);

  // PARSE_EMPTY / HTTP_FORBIDDEN 这类「建议切换源」的错误，默认聚焦「切换源」主按钮
  onMount(() => {
    if (error.kind === "PARSE_EMPTY" || error.kind === "HTTP_FORBIDDEN") {
      switchBtn?.focus({ preventScroll: true });
    }
  });
</script>

<div
  class="player-error-overlay"
  role="alertdialog"
  aria-modal="true"
  aria-labelledby="player-error-title"
  data-testid="player-error-overlay"
>
  <div class="player-error-overlay__card">
    <Icon name="info" size={34} className="player-error-overlay__icon" />
    <h2 id="player-error-title">{messageForErrorKind(error.kind)}</h2>
    <p class="player-error-overlay__kind">
      {error.kind}
      {#if error.httpStatus !== undefined} · HTTP {error.httpStatus}{/if}
    </p>
    {#if error.detail}
      <p class="player-error-overlay__detail">{error.detail}</p>
    {/if}
    <div class="player-error-overlay__actions">
      <button type="button" class="player-error-overlay__btn" onclick={onRetry}>
        <Icon name="refresh" size={14} /> 重试{retryCount > 0 ? `（${retryCount}）` : ""}
      </button>
      <button
        type="button"
        class="player-error-overlay__btn player-error-overlay__btn--primary"
        bind:this={switchBtn}
        onclick={onSwitchSource}
      >
        <Icon name="externalLink" size={14} /> 切换源
      </button>
      <button type="button" class="player-error-overlay__btn" onclick={onCopyLog}>
        <Icon name="paperclip" size={14} /> 复制日志
      </button>
    </div>
  </div>
</div>

<style>
  .player-error-overlay {
    position: absolute;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 6, 12, 0.82);
    backdrop-filter: blur(10px);
  }
  .player-error-overlay__card {
    width: min(440px, 100%);
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 28px 24px;
    border: 1px solid rgba(248, 113, 113, 0.24);
    border-radius: 16px;
    background: rgba(23, 9, 12, 0.92);
    color: var(--text-primary, #f5f7fb);
    text-align: center;
  }
  :global(.player-error-overlay__icon) {
    color: #f87171;
  }
  .player-error-overlay__card h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
  }
  .player-error-overlay__kind {
    margin: 0;
    color: var(--text-muted, #9aa3b2);
    font: 600 11px/1 var(--font-mono, monospace);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .player-error-overlay__detail {
    max-width: 34rem;
    margin: 0;
    color: rgba(247, 248, 251, 0.72);
    font-size: 12px;
    line-height: 1.5;
    word-break: break-word;
  }
  .player-error-overlay__actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 6px;
  }
  .player-error-overlay__btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-secondary, #c7cdd8);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .player-error-overlay__btn:hover {
    border-color: var(--text-muted, #9aa3b2);
    color: var(--text-primary, #f5f7fb);
  }
  .player-error-overlay__btn--primary {
    border-color: rgba(232, 85, 127, 0.5);
    background: rgba(232, 85, 127, 0.14);
    color: #ffd6e4;
  }
  .player-error-overlay__btn--primary:hover {
    border-color: var(--accent, #e8557f);
    background: var(--accent, #e8557f);
    color: #fff;
  }
  @media (prefers-reduced-motion: reduce) {
    .player-error-overlay * {
      transition: none !important;
    }
  }
</style>
