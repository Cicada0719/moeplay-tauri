<script lang="ts">
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import Icon from "./Icon.svelte";

  let { open = $bindable(false) }: { open: boolean } = $props();

  type UpdateState = 'idle' | 'checking' | 'available' | 'not-available' | 'downloading' | 'ready' | 'error';

  let dialogState = $state<UpdateState>('idle');
  let updateVersion = $state('');
  let updateNotes = $state('');
  let downloadProgress = $state(0);
  let downloadTotal = $state(0);
  let errorMsg = $state('');
  let updateInfo = $state<any>(null);

  $effect(() => {
    if (open && dialogState === 'idle') {
      doCheck();
    }
  });

  async function doCheck() {
    dialogState = 'checking';
    errorMsg = '';
    try {
      const update = await check();
      if (update) {
        updateInfo = update;
        updateVersion = update.version;
        updateNotes = update.body || '无更新说明';
        dialogState = 'available';
      } else {
        dialogState = 'not-available';
      }
    } catch (e) {
      errorMsg = String(e);
      dialogState = 'error';
    }
  }

  async function doDownload() {
    if (!updateInfo) return;
    dialogState = 'downloading';
    downloadProgress = 0;
    downloadTotal = 0;

    try {
      await updateInfo.downloadAndInstall((event: any) => {
        switch (event.event) {
          case 'Started':
            if (event.data.contentLength) {
              downloadTotal = event.data.contentLength;
            }
            break;
          case 'Progress':
            downloadProgress += event.data.chunkLength;
            break;
          case 'Finished':
            dialogState = 'ready';
            break;
        }
      });
    } catch (e) {
      errorMsg = String(e);
      dialogState = 'error';
    }
  }

  async function doRelaunch() {
    try {
      await relaunch();
    } catch (e) {
      errorMsg = String(e);
    }
  }

  function close() {
    open = false;
    dialogState = 'idle';
    errorMsg = '';
  }

  const progressPercent = $derived(
    downloadTotal > 0 ? Math.min(100, Math.round(downloadProgress / downloadTotal * 100)) : 0
  );
  const progressLabel = $derived(
    downloadTotal > 0
      ? `${(downloadProgress / 1024 / 1024).toFixed(1)} / ${(downloadTotal / 1024 / 1024).toFixed(1)} MB`
      : `${(downloadProgress / 1024 / 1024).toFixed(1)} MB`
  );
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="update-backdrop" role="none" onclick={close}></div>
  <div class="update-dialog" role="dialog">
    <div class="update-header">
      <Icon name="download" size={20} />
      <span class="update-title">应用更新</span>
      <button class="update-close" onclick={close}>
        <Icon name="x" size={16} />
      </button>
    </div>

    <div class="update-body">
      {#if dialogState === 'checking'}
        <div class="update-state">
          <div class="update-spinner"></div>
          <span>正在检查更新…</span>
        </div>

      {:else if dialogState === 'not-available'}
        <div class="update-state">
          <Icon name="check" size={32} />
          <span class="state-title">已是最新版本</span>
          <small>当前版本无需更新</small>
        </div>

      {:else if dialogState === 'available'}
        <div class="update-state">
          <Icon name="download" size={32} />
          <span class="state-title">发现新版本 v{updateVersion}</span>
          <div class="update-notes">{updateNotes}</div>
          <button class="update-btn primary" onclick={doDownload}>
            <Icon name="download" size={16} /> 下载并安装
          </button>
        </div>

      {:else if dialogState === 'downloading'}
        <div class="update-state">
          <div class="update-spinner"></div>
          <span class="state-title">正在下载更新…</span>
          <div class="download-progress">
            <div class="progress-bar">
              <div class="progress-fill" style="width: {progressPercent}%"></div>
            </div>
            <span class="progress-text">{progressLabel} ({progressPercent}%)</span>
          </div>
        </div>

      {:else if dialogState === 'ready'}
        <div class="update-state">
          <Icon name="check" size={32} />
          <span class="state-title">下载完成</span>
          <small>重启应用以完成更新</small>
          <button class="update-btn primary" onclick={doRelaunch}>
            <Icon name="refresh" size={16} /> 立即重启
          </button>
        </div>

      {:else if dialogState === 'error'}
        <div class="update-state">
          <Icon name="x" size={32} />
          <span class="state-title">更新检查失败</span>
          <small>{errorMsg}</small>
          <button class="update-btn" onclick={doCheck}>重试</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .update-backdrop {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 50; animation: fade-in 0.2s ease;
  }
  .update-dialog {
    position: fixed; top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    width: min(440px, 90vw);
    background: #161b22;
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 14px;
    z-index: 51;
    overflow: hidden;
    animation: fade-in 0.2s ease;
    box-shadow: 0 16px 48px rgba(0,0,0,0.5);
  }
  .update-header {
    display: flex; align-items: center; gap: 10px;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(255,255,255,0.06);
    color: var(--text-primary);
  }
  .update-title { flex: 1; font-size: 15px; font-weight: 650; }
  .update-close {
    width: 28px; height: 28px; border: none; border-radius: 50%;
    background: rgba(255,255,255,0.06); color: var(--text-muted);
    cursor: pointer; display: flex; align-items: center; justify-content: center;
  }
  .update-close:hover { background: rgba(255,255,255,0.1); color: #fff; }
  .update-body { padding: 24px 18px; }
  .update-state {
    display: flex; flex-direction: column; align-items: center; gap: 12px;
    text-align: center; color: var(--text-muted); font-size: 13px;
  }
  .state-title { font-size: 16px; font-weight: 650; color: var(--text-primary); }
  .update-notes {
    width: 100%; max-height: 120px; overflow-y: auto;
    padding: 10px; border-radius: 8px;
    background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06);
    font-size: 12px; line-height: 1.6; color: var(--text-secondary);
    text-align: left; white-space: pre-wrap;
  }
  .update-btn {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 8px 20px; border: 1px solid rgba(255,255,255,0.12);
    border-radius: 8px; background: transparent;
    color: var(--text-secondary); font-size: 13px; cursor: pointer;
  }
  .update-btn:hover { border-color: var(--accent); color: var(--accent); }
  .update-btn.primary {
    border-color: var(--accent-ring); background: var(--accent); color: #fff;
  }
  .update-btn.primary:hover { background: var(--accent-hi); }
  .update-spinner {
    width: 28px; height: 28px;
    border: 3px solid rgba(255,255,255,0.08);
    border-top-color: var(--accent); border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  .download-progress { width: 100%; display: flex; flex-direction: column; gap: 6px; }
  .progress-bar {
    width: 100%; height: 6px; border-radius: 3px;
    background: rgba(255,255,255,0.08); overflow: hidden;
  }
  .progress-fill {
    height: 100%; border-radius: 3px;
    background: var(--accent); transition: width 0.2s ease;
  }
  .progress-text {
    font-size: 11px; color: var(--text-muted);
    font-family: var(--font-mono);
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }

  @media (prefers-reduced-motion: reduce) {
    .update-backdrop, .update-dialog, .update-spinner { animation: none; }
    .progress-fill { transition: none; }
  }
  :global([data-motion="reduce"]) .update-backdrop,
  :global([data-motion="reduce"]) .update-dialog,
  :global([data-motion="reduce"]) .update-spinner { animation: none; }
  :global([data-motion="reduce"]) .progress-fill { transition: none; }
</style>
