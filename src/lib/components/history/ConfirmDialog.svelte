<script lang="ts">
  // 通用确认弹窗（spec §4 步骤 5：单条删除 Popover / 批量删除 Modal / 按类型清空 Modal）
  //
  // 采用 Svelte 5 回调 props（`onconfirm` / `oncancel`）而非 `createEventDispatcher`，
  // 与仓库其余组件（`Dialog.onClose`、`BPSearch.onselect`）一致，便于单测断言。
  import type { Snippet } from 'svelte';
  import { Button, Dialog } from '../ui';

  let {
    open = false,
    title = '确认操作',
    message = '',
    confirmLabel = '删除',
    danger = true,
    onconfirm,
    oncancel,
    children,
  }: {
    open?: boolean;
    title?: string;
    message?: string;
    confirmLabel?: string;
    danger?: boolean;
    onconfirm?: () => void;
    oncancel?: () => void;
    children?: Snippet;
  } = $props();
</script>

<Dialog {open} title={title} onClose={() => oncancel?.()}>
  <div class="confirm-dialog">
    {#if message}
      <p class="confirm-message" data-testid="confirm-message">{message}</p>
    {/if}
    {@render children?.()}
    <div class="confirm-actions">
      <Button variant="ghost" size="sm" press={() => oncancel?.()}>取消</Button>
      <Button
        variant={danger ? 'secondary' : 'primary'}
        size="sm"
        press={() => onconfirm?.()}
        >{confirmLabel}</Button
      >
    </div>
  </div>
</Dialog>

<style>
  .confirm-dialog {
    display: grid;
    min-width: min(20rem, 72vw);
    gap: var(--v2-space-4, 1rem);
    padding: var(--v2-space-4, 1rem);
    color: var(--v2-color-text, var(--text-primary));
  }

  .confirm-message {
    margin: 0;
    color: var(--v2-color-text-secondary, var(--text-muted));
    font-size: var(--v2-text-sm, 0.875rem);
    line-height: 1.6;
    white-space: pre-wrap;
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--v2-space-2, 0.5rem);
  }
</style>
