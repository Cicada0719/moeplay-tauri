<script lang="ts">
  import { pickDirectory } from "../../../api";
  import type { ComicProviderAuthMode, ComicProviderConfigureRequest, ComicProviderKind } from "../../../features/comic/types";
  import Icon from "../../Icon.svelte";
  import { Button, Input, Tag } from "../../ui";

  let {
    busy = false,
    onconfigure,
    oncancel,
    pickRoot = pickDirectory,
  }: {
    busy?: boolean;
    onconfigure: (request: ComicProviderConfigureRequest) => Promise<void> | void;
    oncancel?: () => void;
    pickRoot?: () => Promise<string>;
  } = $props();

  let kind = $state<ComicProviderKind>("local");
  let root = $state("");
  let baseUrl = $state("");
  let authMode = $state<ComicProviderAuthMode>("none");
  let username = $state("");
  let secret = $state("");
  let localError = $state("");
  let picking = $state(false);

  const authOptions = $derived(kind === "komga"
    ? [
        { value: "none", label: "无需认证" },
        { value: "basic", label: "Basic" },
        { value: "bearer", label: "Bearer" },
      ] as const
    : [{ value: "api_key", label: "API Key" }] as const);

  function selectKind(next: ComicProviderKind) {
    kind = next;
    localError = "";
    if (next === "komga") authMode = "none";
    if (next === "kavita") authMode = "api_key";
  }

  async function chooseRoot() {
    picking = true;
    localError = "";
    try {
      const selected = await pickRoot();
      if (selected) root = selected;
    } catch (error) {
      localError = error instanceof Error ? error.message : "无法选择漫画目录";
    } finally {
      picking = false;
    }
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    localError = "";
    if (kind === "local") {
      if (!root.trim()) {
        localError = "请选择包含 moeplay-comics.json 的漫画根目录";
        return;
      }
      await onconfigure({ kind: "local", root: root.trim() });
      return;
    }

    if (!baseUrl.trim()) {
      localError = "请输入服务器地址";
      return;
    }
    if (kind === "komga" && authMode === "basic" && !username.trim()) {
      localError = "Basic 认证需要用户名";
      return;
    }

    const oneTimeSecret = secret;
    secret = "";
    const request: ComicProviderConfigureRequest = {
      kind,
      baseUrl: baseUrl.trim(),
      authMode: kind === "kavita" ? "api_key" : authMode,
      ...(username.trim() ? { username: username.trim() } : {}),
      ...(oneTimeSecret ? { secret: oneTimeSecret } : {}),
    };
    try {
      await onconfigure(request);
    } finally {
      secret = "";
    }
  }
</script>

<form class="config-panel" aria-label="配置 Comic Provider v2" onsubmit={submit}>
  <div class="config-head">
    <div>
      <span class="eyebrow">Provider v2</span>
      <h2>添加漫画源</h2>
      <p>配置只保存非敏感元数据；密码、令牌和 API Key 仅作为一次性输入交给系统安全存储。</p>
    </div>
    {#if oncancel}
      <Button variant="quiet" size="sm" press={oncancel} ariaLabel="关闭漫画源配置"><Icon name="x" size={15} /></Button>
    {/if}
  </div>

  <div class="kind-grid" role="tablist" aria-label="漫画源类型">
    {#each [
      { value: "local", label: "本地目录", hint: "manifest + 图片" },
      { value: "komga", label: "Komga", hint: "none / basic / bearer" },
      { value: "kavita", label: "Kavita", hint: "API Key" },
    ] as option}
      <button type="button" class:active={kind === option.value} onclick={() => selectKind(option.value as ComicProviderKind)}>
        <strong>{option.label}</strong>
        <span>{option.hint}</span>
      </button>
    {/each}
  </div>

  {#if kind === "local"}
    <div class="field">
      <span>漫画根目录</span>
      <div class="path-row">
        <Input bind:value={root} placeholder="请选择本地漫画目录" readonly ariaLabel="本地漫画根目录" />
        <Button type="button" variant="secondary" press={chooseRoot} loading={picking} disabled={busy}>
          <Icon name="folder" size={15} />选择目录
        </Button>
      </div>
      <small>目录中需要存在 Provider v2 本地清单 <code>moeplay-comics.json</code>。</small>
    </div>
  {:else}
    <label class="field">
      <span>服务器地址</span>
      <Input type="url" bind:value={baseUrl} placeholder="https://comics.example.com" autocomplete="url" disabled={busy} ariaLabel="漫画服务器地址" />
    </label>

    <div class="field">
      <span>认证方式</span>
      <div class="auth-row">
        {#each authOptions as option}
          <Tag active={authMode === option.value} onclick={() => authMode = option.value}>{option.label}</Tag>
        {/each}
      </div>
    </div>

    {#if kind === "komga" && authMode === "basic"}
      <label class="field">
        <span>用户名</span>
        <Input bind:value={username} autocomplete="username" placeholder="Komga 用户名" disabled={busy} ariaLabel="Komga 用户名" />
      </label>
    {/if}

    {#if authMode !== "none" || kind === "kavita"}
      <label class="field">
        <span>{kind === "kavita" ? "API Key" : authMode === "basic" ? "密码" : "Bearer Token"}</span>
        <Input
          type="password"
          bind:value={secret}
          autocomplete="new-password"
          placeholder="仅本次提交使用；已配置凭据可留空"
          disabled={busy}
          ariaLabel="一次性漫画源凭据"
        />
        <small>提交后此输入会立即清空，页面不会回显或持久化凭据。</small>
      </label>
    {/if}
  {/if}

  {#if localError}<p class="form-error" role="alert">{localError}</p>{/if}

  <div class="config-actions">
    {#if oncancel}<Button type="button" variant="ghost" press={oncancel} disabled={busy}>取消</Button>{/if}
    <Button type="submit" loading={busy}>保存并切换</Button>
  </div>
</form>

<style>
  .config-panel { display: flex; flex-direction: column; gap: 18px; padding: 22px; border: 1px solid var(--border); border-radius: var(--radius-lg); background: color-mix(in srgb, var(--bg-surface) 94%, transparent); box-shadow: 0 20px 55px rgba(0,0,0,.18); }
  .config-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 18px; }
  .eyebrow { color: var(--accent); font-family: var(--font-mono); font-size: 10px; font-weight: 700; letter-spacing: .14em; text-transform: uppercase; }
  h2 { margin: 5px 0 4px; font-family: var(--font-display); font-size: 21px; }
  p { margin: 0; color: var(--text-muted); font-size: 12px; line-height: 1.6; max-width: 64ch; }
  .kind-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
  .kind-grid button { min-width: 0; padding: 13px; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--bg-elev); color: var(--text-secondary); text-align: left; cursor: pointer; transition: border-color .18s ease, background .18s ease, transform .18s ease; }
  .kind-grid button:hover { transform: translateY(-1px); border-color: var(--border-hover); }
  .kind-grid button.active { border-color: color-mix(in srgb, var(--accent) 55%, var(--border)); background: var(--accent-lo); color: var(--text-primary); }
  .kind-grid strong, .kind-grid span { display: block; overflow: hidden; text-overflow: ellipsis; }
  .kind-grid strong { font-size: 13px; }
  .kind-grid span { margin-top: 4px; color: var(--text-muted); font-family: var(--font-mono); font-size: 9px; }
  .field { display: flex; flex-direction: column; gap: 7px; color: var(--text-secondary); font-size: 12px; font-weight: 650; }
  .field small { color: var(--text-muted); font-weight: 400; line-height: 1.5; }
  code { font-family: var(--font-mono); color: var(--text-secondary); }
  .path-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; }
  .auth-row { display: flex; flex-wrap: wrap; gap: 8px; }
  .form-error { padding: 10px 12px; border: 1px solid rgba(248,113,113,.28); border-radius: var(--radius-md); background: rgba(248,113,113,.08); color: #f87171; }
  .config-actions { display: flex; justify-content: flex-end; gap: 8px; }
  @media (max-width: 720px) { .kind-grid { grid-template-columns: 1fr; } .path-row { grid-template-columns: 1fr; } }
  @media (prefers-reduced-motion: reduce) { .kind-grid button { transition: none; } }
</style>

