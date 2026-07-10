<script lang="ts">
  import type {
    AnimeLocalMediaScanResult,
    AnimeProviderApi,
    AnimeProviderFeatureState,
    AnimeProviderFeatureStore,
  } from "../../../features/anime";
  import Icon from "../../Icon.svelte";
  import { Drawer } from "../../ui-v2";
  import { focusRovingItem, nextRovingIndex } from "../a11y";

  let {
    api,
    store,
    snapshot,
    onClose,
  }: {
    api: AnimeProviderApi;
    store: AnimeProviderFeatureStore;
    snapshot: AnimeProviderFeatureState;
    onClose: () => void;
  } = $props();

  let mode = $state<"local_media" | "jellyfin">("local_media");
  let scan = $state<AnimeLocalMediaScanResult | null>(null);
  let scanning = $state(false);
  let scanError = $state("");
  let jellyfinBaseUrl = $state("");
  let jellyfinToken = $state("");
  let modeTabRefs: Array<HTMLButtonElement | null> = [];


  function selectMode(nextMode: "local_media" | "jellyfin", index: number) {
    mode = nextMode;
    focusRovingItem(modeTabRefs, index);
  }

  function handleModeKeydown(event: KeyboardEvent, index: number) {
    const next = nextRovingIndex(event.key, index, 2, "horizontal");
    if (next === null) return;
    event.preventDefault();
    selectMode(next === 0 ? "local_media" : "jellyfin", next);
  }

  async function pickAndScan() {
    scanning = true;
    scanError = "";
    try {
      scan = await api.pickLocalDirectory();
    } catch {
      scanError = "目录选择或扫描失败，请确认目录可访问后重试。";
    } finally {
      scanning = false;
    }
  }

  async function configureLocalMedia() {
    if (!scan || scan.fileCount === 0) return;
    const configured = await store.configure({
      kind: "local_media",
      allowedPaths: scan.allowedPaths,
      library: scan.library,
    });
    if (configured) {
      scan = null;
      onClose();
    }
  }

  async function configureJellyfin(event: SubmitEvent) {
    event.preventDefault();
    const baseUrl = jellyfinBaseUrl.trim();
    const token = jellyfinToken;
    if (!baseUrl || !token.trim()) return;
    try {
      const configured = await store.configure({ kind: "jellyfin", baseUrl, token });
      if (configured) onClose();
    } finally {
      // Credential input is one-shot and is never copied into the feature store.
      jellyfinToken = "";
    }
  }

  async function removeProvider(providerId: string, providerName: string) {
    if (!window.confirm(`移除来源“${providerName}”？关联的安全凭据也会一并删除。`)) return;
    await store.removeProvider(providerId);
  }
</script>

<Drawer
  open
  title="来源配置"
  description="添加本地媒体或 Jellyfin。凭据只会交给系统安全存储。"
  side="right"
  size="lg"
  onClose={onClose}
  initialFocus="[data-provider-config-primary]"
  returnFocus
  class="provider-config-drawer"
>
  <div class="config-panel">
    <div class="panel-body">
      <section class="configured-section" aria-labelledby="configured-title">
        <div class="section-heading">
          <div>
            <span class="section-index">01</span>
            <h3 id="configured-title">已配置来源</h3>
          </div>
          <span>{snapshot.providers.length} 个</span>
        </div>

        {#if snapshot.isLoadingProviders}
          <div class="compact-state"><span class="spinner"></span>正在读取来源</div>
        {:else if snapshot.providers.length === 0}
          <div class="compact-state muted">还没有 Provider v2 来源，请在下方添加。</div>
        {:else}
          <div class="provider-list">
            {#each snapshot.providers as provider (provider.id)}
              <article class="provider-row" class:selected={snapshot.selectedProviderId === provider.id}>
                <button class="provider-select" type="button" onclick={() => store.selectProvider(provider.id)}>
                  <span class="provider-mark"><Icon name={provider.kind === "local_media" ? "folder" : "database"} size={17} /></span>
                  <span class="provider-copy">
                    <strong>{provider.name}</strong>
                    <small>
                      {provider.kind === "local_media"
                        ? `${provider.localFileCount ?? 0} 个媒体文件`
                        : provider.baseUrl ?? "Jellyfin"}
                    </small>
                  </span>
                  {#if snapshot.selectedProviderId === provider.id}<span class="active-label">当前</span>{/if}
                </button>
                <button
                  class="remove-button"
                  type="button"
                  aria-label={`移除 ${provider.name}`}
                  disabled={snapshot.isRemovingProvider}
                  onclick={() => removeProvider(provider.id, provider.name)}
                >
                  <Icon name="trash" size={15} />
                </button>
              </article>
            {/each}
          </div>
        {/if}
      </section>

      <section class="add-section" aria-labelledby="add-title">
        <div class="section-heading">
          <div>
            <span class="section-index">02</span>
            <h3 id="add-title">添加来源</h3>
          </div>
        </div>

        <div class="mode-switch" role="tablist" aria-label="来源类型">
          <button bind:this={modeTabRefs[0]} data-provider-config-primary type="button" role="tab" aria-selected={mode === "local_media"} tabindex={mode === "local_media" ? 0 : -1} class:active={mode === "local_media"} onclick={() => selectMode("local_media", 0)} onkeydown={(event) => handleModeKeydown(event, 0)}>
            <Icon name="folder" size={15} />本地媒体
          </button>
          <button bind:this={modeTabRefs[1]} type="button" role="tab" aria-selected={mode === "jellyfin"} tabindex={mode === "jellyfin" ? 0 : -1} class:active={mode === "jellyfin"} onclick={() => selectMode("jellyfin", 1)} onkeydown={(event) => handleModeKeydown(event, 1)}>
            <Icon name="database" size={15} />Jellyfin
          </button>
        </div>

        {#if mode === "local_media"}
          <div class="config-form">
            <div class="form-intro">
              <h4>选择番剧目录</h4>
              <p>会递归扫描常见视频格式，按所在文件夹分组为番剧，并从文件名识别集数。</p>
            </div>
            <button class="outline-button" type="button" onclick={pickAndScan} disabled={scanning || snapshot.isConfiguring}>
              <Icon name="folder" size={16} />
              {scanning ? "正在扫描" : "选择并扫描目录"}
            </button>

            {#if scanError}<p class="form-error" role="alert">{scanError}</p>{/if}
            {#if scan}
              <div class="scan-result">
                <div class="scan-path"><Icon name="folder" size={14} /><span title={scan.directory}>{scan.directory}</span></div>
                <div class="scan-stats">
                  <span><strong>{scan.seriesCount}</strong> 部番剧</span>
                  <span><strong>{scan.fileCount}</strong> 个视频</span>
                  {#if scan.skippedCount > 0}<span><strong>{scan.skippedCount}</strong> 项跳过</span>{/if}
                </div>
                {#each scan.warnings as warning}<p class="scan-warning">{warning}</p>{/each}
                <button class="primary-button" type="button" onclick={configureLocalMedia} disabled={scan.fileCount === 0 || snapshot.isConfiguring}>
                  {snapshot.isConfiguring ? "正在保存" : "添加本地媒体"}
                </button>
              </div>
            {/if}
          </div>
        {:else}
          <form class="config-form" onsubmit={configureJellyfin}>
            <div class="form-intro">
              <h4>连接 Jellyfin</h4>
              <p>支持 HTTPS 服务和本机 localhost HTTP。令牌提交后立即从表单清除，不会进入前端来源状态。</p>
            </div>
            <label>
              <span>服务器地址</span>
              <input type="url" bind:value={jellyfinBaseUrl} placeholder="https://jellyfin.example.com" required autocomplete="url" />
            </label>
            <label>
              <span>访问令牌</span>
              <input type="password" bind:value={jellyfinToken} placeholder="输入一次性访问令牌" required autocomplete="off" spellcheck="false" />
            </label>
            <p class="security-note"><Icon name="shield" size={14} />令牌由 Rust 后端写入操作系统凭据存储，来源列表和播放界面不会返回令牌。</p>
            <button class="primary-button" type="submit" disabled={!jellyfinBaseUrl.trim() || !jellyfinToken.trim() || snapshot.isConfiguring}>
              {snapshot.isConfiguring ? "正在连接" : "保存并连接"}
            </button>
          </form>
        {/if}
      </section>
    </div>
  </div>
</Drawer>

<style>
  :global(.v2-drawer.provider-config-drawer) { width: min(48rem, calc(100vw - 1rem)); }
  :global(.v2-drawer.provider-config-drawer .v2-drawer__body) { padding: 0; }
  .config-panel { position: static; width: 100%; max-width: none; height: 100%; border: 0; border-radius: 0; box-shadow: none; animation: none; }

  .config-panel {
    width: min(560px, 100%);
    height: 100%;
    display: flex;
    flex-direction: column;
    border-left: 1px solid rgba(255,255,255,0.09);
    background: #11151c;
    box-shadow: -24px 0 60px rgba(0,0,0,0.36);
  }
  .section-index {
    color: #69b8a5;
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 750;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  h3, h4, p { margin: 0; }
  .remove-button {
    display: grid;
    place-items: center;
    border: 1px solid rgba(255,255,255,0.1);
    background: rgba(255,255,255,0.035);
    color: #aeb5c0;
    cursor: pointer;
  }
  .panel-body { min-height: 0; flex: 1; overflow: auto; padding: 22px 24px 34px; }
  .configured-section, .add-section { display: flex; flex-direction: column; gap: 14px; }
  .add-section { margin-top: 30px; padding-top: 24px; border-top: 1px solid rgba(255,255,255,0.08); }
  .section-heading { display: flex; align-items: center; justify-content: space-between; color: #7f8998; font-size: 11px; }
  .section-heading > div { display: flex; align-items: center; gap: 9px; }
  .section-heading h3 { color: #e9edf2; font-size: 14px; }
  .provider-list { display: flex; flex-direction: column; gap: 8px; }
  .provider-row { display: grid; grid-template-columns: 1fr auto; border: 1px solid rgba(255,255,255,0.08); border-radius: 11px; background: rgba(255,255,255,0.025); overflow: hidden; }
  .provider-row.selected { border-color: rgba(105,184,165,0.36); background: rgba(105,184,165,0.055); }
  .provider-select { min-width: 0; display: flex; align-items: center; gap: 11px; padding: 11px 12px; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .provider-mark { width: 34px; height: 34px; display: grid; place-items: center; flex: 0 0 auto; border-radius: 9px; background: rgba(255,255,255,0.055); color: #69b8a5; }
  .provider-copy { min-width: 0; flex: 1; display: flex; flex-direction: column; gap: 3px; }
  .provider-copy strong { color: #edf0f4; font-size: 13px; }
  .provider-copy small { overflow: hidden; color: #7f8998; font-size: 10.5px; text-overflow: ellipsis; white-space: nowrap; }
  .active-label { color: #69b8a5; font-family: var(--font-mono); font-size: 9px; font-weight: 750; letter-spacing: 0.08em; }
  .remove-button { width: 39px; margin: 8px 8px 8px 0; border-radius: 8px; }
  .remove-button:hover { border-color: rgba(248,113,113,0.28); color: #f08d95; }
  .compact-state { display: flex; align-items: center; gap: 8px; min-height: 62px; padding: 14px; border: 1px dashed rgba(255,255,255,0.1); border-radius: 10px; color: #a8b0bc; font-size: 12px; }
  .compact-state.muted { color: #737d8c; }
  .spinner { width: 14px; height: 14px; border: 2px solid rgba(255,255,255,0.12); border-top-color: #69b8a5; border-radius: 50%; animation: spin .7s linear infinite; }
  .mode-switch { display: grid; grid-template-columns: 1fr 1fr; padding: 3px; border: 1px solid rgba(255,255,255,0.08); border-radius: 10px; background: rgba(0,0,0,0.18); }
  .mode-switch button { min-height: 34px; display: inline-flex; align-items: center; justify-content: center; gap: 7px; border: 0; border-radius: 7px; background: transparent; color: #7f8998; font-size: 12px; font-weight: 650; cursor: pointer; }
  .mode-switch button.active { background: rgba(255,255,255,0.075); color: #f0f3f6; }
  .config-form { display: flex; flex-direction: column; gap: 13px; padding: 17px; border: 1px solid rgba(255,255,255,0.08); border-radius: 12px; background: rgba(255,255,255,0.022); }
  .form-intro h4 { color: #e9edf2; font-size: 14px; }
  .form-intro p { margin-top: 5px; color: #838d9c; font-size: 11.5px; line-height: 1.6; }
  label { display: flex; flex-direction: column; gap: 6px; color: #9ca5b2; font-size: 11px; font-weight: 650; }
  input { width: 100%; height: 38px; padding: 0 11px; border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; outline: 0; background: rgba(0,0,0,0.22); color: #eef1f4; font: inherit; font-size: 12px; }
  input:focus { border-color: rgba(105,184,165,0.6); box-shadow: 0 0 0 3px rgba(105,184,165,0.08); }
  .outline-button, .primary-button { min-height: 38px; display: inline-flex; align-items: center; justify-content: center; gap: 8px; border-radius: 9px; font: inherit; font-size: 12px; font-weight: 750; cursor: pointer; }
  .outline-button { border: 1px dashed rgba(105,184,165,0.35); background: rgba(105,184,165,0.05); color: #83c8b7; }
  .primary-button { border: 0; background: #64b49f; color: #07120f; }
  button:disabled { opacity: .5; cursor: wait; }
  .scan-result { display: flex; flex-direction: column; gap: 11px; padding-top: 13px; border-top: 1px solid rgba(255,255,255,0.07); }
  .scan-path { min-width: 0; display: flex; align-items: center; gap: 7px; color: #aeb6c1; font-size: 10.5px; }
  .scan-path span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .scan-stats { display: flex; gap: 7px; flex-wrap: wrap; }
  .scan-stats span { padding: 6px 8px; border-radius: 7px; background: rgba(255,255,255,0.045); color: #8e98a6; font-size: 10px; }
  .scan-stats strong { color: #e6eaef; font-size: 12px; }
  .scan-warning, .form-error { color: #e9aa72; font-size: 10.5px; line-height: 1.5; }
  .form-error { color: #ef969e; }
  .security-note { display: flex; align-items: flex-start; gap: 7px; color: #7f8998; font-size: 10.5px; line-height: 1.55; }
  @keyframes spin { to { transform: rotate(360deg); } }

  @media (prefers-reduced-motion: reduce) {
    .config-panel, .config-panel * { animation: none !important; transition: none !important; }
  }
  :global([data-motion="reduce"]) .config-panel,
  :global([data-motion="reduce"]) .config-panel * { animation: none !important; transition: none !important; }
</style>
