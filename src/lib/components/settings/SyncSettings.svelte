<script lang="ts">
  import { onMount } from "svelte";
  import Card from "../ui/Card.svelte";
  import Button from "../ui/Button.svelte";
  import Input from "../ui/Input.svelte";
  import SegmentControl from "../ui/SegmentControl.svelte";
  import Icon from "../Icon.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import {
    clearWebdavConfig,
    getSyncConfig,
    getSyncStatus,
    saveWebdavConfig,
    syncErrorMessage,
    syncNow,
    testWebdavConnection,
  } from "../../api/sync";
  import type { SyncMode, SyncResult, SyncStatus, WebDavConfig } from "../../types/sync";
  import "./settings-shared.css";

  // ── 表单状态（与后端 WebDavConfig 字段一一对应） ──
  let baseUrl = $state("");
  let username = $state("");
  let mode = $state<SyncMode>("manual");
  let remoteDir = $state("moeplay-sync");
  let configured = $state(false);

  // ── 交互状态 ──
  let passwordInput = $state("");
  let status = $state<SyncStatus>({ configured: false, last_result: null, syncing: false });
  let testing = $state(false);
  let saving = $state(false);
  let syncing = $state(false);
  let clearing = $state(false);
  let errorMessage = $state("");
  let lastResult = $state<SyncResult | null>(null);

  // 旧版本后端或升级中的 WebView 可能返回 null / 缺字段；设置页必须把
  // 它归一化为可渲染状态，不能把原始 TypeError 暴露给用户。
  function normalizeStatus(value: unknown): SyncStatus {
    if (!value || typeof value !== "object") {
      return { configured: false, last_result: null, syncing: false };
    }
    const raw = value as Partial<SyncStatus>;
    const result = raw.last_result;
    const last = result && typeof result === "object"
      ? {
          uploaded: Number((result as SyncResult).uploaded) || 0,
          downloaded: Number((result as SyncResult).downloaded) || 0,
          conflicts: Number((result as SyncResult).conflicts) || 0,
          tombstones_purged: Number((result as SyncResult).tombstones_purged) || 0,
          synced_at: Number((result as SyncResult).synced_at) || 0,
        }
      : null;
    return {
      configured: raw.configured === true,
      last_result: last,
      syncing: raw.syncing === true,
    };
  }

  function currentConfig(): WebDavConfig {
    return {
      base_url: baseUrl.trim(),
      username: username.trim(),
      mode,
      remote_dir: remoteDir.trim() || "moeplay-sync",
    };
  }

  function showError(err: unknown) {
    errorMessage = syncErrorMessage(err);
  }

  async function load() {
    try {
      const [cfg, st] = await Promise.all([getSyncConfig(), getSyncStatus()]);
      if (cfg) {
        baseUrl = cfg.base_url;
        username = cfg.username;
        mode = cfg.mode;
        remoteDir = cfg.remote_dir || "moeplay-sync";
        configured = true;
      }
      status = normalizeStatus(st);
      lastResult = status.last_result;
    } catch (err) {
      showError(err);
    }
  }

  onMount(() => {
    void load();
  });

  async function handleSave() {
    saving = true;
    errorMessage = "";
    try {
      await saveWebdavConfig(currentConfig(), passwordInput || null);
      passwordInput = "";
      configured = true;
      const st = await getSyncStatus();
      status = normalizeStatus(st);
      lastResult = status.last_result;
      uiStore.notify("WebDAV 配置已保存", "success");
    } catch (err) {
      showError(err);
    } finally {
      saving = false;
    }
  }

  async function handleTest() {
    testing = true;
    errorMessage = "";
    try {
      // 密码为空时后端回退到已保存的 keyring 凭据。
      await testWebdavConnection(currentConfig(), passwordInput);
      uiStore.notify("连接成功", "success");
    } catch (err) {
      showError(err);
    } finally {
      testing = false;
    }
  }

  async function handleSyncNow() {
    syncing = true;
    errorMessage = "";
    try {
      const result = await syncNow();
      lastResult = result;
      status = { ...status, configured: true, syncing: false, last_result: result };
      uiStore.notify(
        `上传 ${result.uploaded} 条 / 下载 ${result.downloaded} 条 / 冲突 ${result.conflicts} 条`,
        "success",
      );
    } catch (err) {
      showError(err);
    } finally {
      syncing = false;
    }
  }

  async function handleClear() {
    if (!window.confirm("确定清除 WebDAV 同步配置与已保存凭据？本地历史记录不会被删除。")) return;
    clearing = true;
    errorMessage = "";
    try {
      await clearWebdavConfig();
      baseUrl = "";
      username = "";
      passwordInput = "";
      configured = false;
      lastResult = null;
      status = { configured: false, last_result: null, syncing: false };
      uiStore.notify("已清除 WebDAV 配置", "success");
    } catch (err) {
      showError(err);
    } finally {
      clearing = false;
    }
  }

  async function handleModeChange(next: string) {
    mode = next === "auto" ? "auto" : "manual";
    errorMessage = "";
    try {
      // 同步模式切换即时保存（不带密码字段则后端保留旧密码）。
      await saveWebdavConfig(currentConfig(), null);
      configured = true;
      uiStore.notify(
        next === "auto" ? "已开启启动时自动 + 每 30 分钟同步" : "已切换为仅手动同步",
        "success",
      );
    } catch (err) {
      showError(err);
    }
  }

  const modeOptions = [
    { value: "manual", label: "仅手动" },
    { value: "auto", label: "启动时自动 + 每 30 分钟" },
  ];

  const busy = $derived(syncing || testing || saving || clearing || status.syncing);
</script>

<span class="section-anchor" id="settings-sync" aria-hidden="true"></span>
<Card class="s-section" padding="lg" ariaLabel="settings-sync">
  <div class="s-head">
    <h2 class="s-title">
      <Icon name="refresh" size={17} className="s-title-ic" />
      历史记录同步
      <span class="s-title-sub">同期 / SYNC</span>
    </h2>
  </div>

  <p class="s-note">
    当前仅同步已接入 SQLite 的历史记录；漫画 / 小说阅读器的 IndexedDB 历史尚未支持跨设备接力。
    密码仅保存在系统安全存储，不会写入配置文件。
  </p>

  <div class="s-divider"></div>

  <div class="ai-form">
    <label class="ai-field">
      <span class="ai-label">WebDAV 地址</span>
      <Input
        type="url"
        bind:value={baseUrl}
        placeholder="https://dav.jianguoyun.com/dav"
        ariaLabel="WebDAV 地址"
      />
    </label>

    <label class="ai-field">
      <span class="ai-label">用户名</span>
      <Input
        bind:value={username}
        placeholder="账号 / 应用专用用户名"
        ariaLabel="用户名"
      />
    </label>

    <label class="ai-field">
      <span class="ai-label">密码</span>
      <Input
        type="password"
        bind:value={passwordInput}
        autocomplete="off"
        placeholder={configured ? "已保存（不显示）" : "应用专用密码 / 授权密码"}
        ariaLabel="密码"
      />
    </label>

    <div class="ai-field">
      <span class="ai-label">同步模式</span>
      <SegmentControl options={modeOptions} value={mode} onChange={handleModeChange} size="sm" />
    </div>
  </div>

  {#if errorMessage}
    <div class="sync-error" role="alert">{errorMessage}</div>
  {/if}

  {#if lastResult}
    <div class="sync-result">
      上传 {lastResult.uploaded} 条 / 下载 {lastResult.downloaded} 条 / 冲突 {lastResult.conflicts}
      条
    </div>
  {/if}

  <div class="s-divider"></div>

  <div class="sync-actions">
    <Button variant="secondary" size="sm" press={handleTest} loading={testing} disabled={busy}>
      <Icon name="refresh" size={14} /> 测试连接
    </Button>
    <Button variant="secondary" size="sm" press={handleSave} loading={saving} disabled={busy}>
      <Icon name="save" size={14} /> 保存配置
    </Button>
    <Button
      variant="primary"
      size="sm"
      press={handleSyncNow}
      loading={syncing}
      disabled={busy || !configured}
    >
      <Icon name="download" size={14} /> 立即同步
    </Button>
    <Button
      variant="ghost"
      size="sm"
      class="sync-clear"
      press={handleClear}
      loading={clearing}
      disabled={busy || !configured}
    >
      <Icon name="trash" size={14} /> 清除配置
    </Button>
  </div>
</Card>

<style>
  .sync-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    padding-top: 4px;
  }
  .sync-error {
    margin: 12px 0 0;
    padding: 9px 12px;
    border: 1px solid color-mix(in srgb, var(--color-error) 45%, var(--border));
    border-radius: 8px;
    background: color-mix(in srgb, var(--color-error) 8%, transparent);
    color: var(--color-error);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .sync-result {
    margin: 12px 0 0;
    padding: 8px 12px;
    border-left: 2px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 6%, transparent);
    color: var(--text-secondary);
    font-size: 12.5px;
  }
  :global(.sync-clear) {
    margin-left: auto;
  }
  @media (max-width: 720px) {
    :global(.sync-clear) {
      margin-left: 0;
    }
  }</style>
