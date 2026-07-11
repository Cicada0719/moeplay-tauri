<script lang="ts">
  import { onMount } from "svelte";
  import Button from "../../components/ui/Button.svelte";
  import Icon from "../../components/Icon.svelte";
  import {
    SOURCE_CAPABILITIES,
    SOURCE_MEDIA_TYPES,
    SOURCE_PRIORITY_MAX,
    SOURCE_PRIORITY_MIN,
    SOURCE_RUNTIME_STATES,
    createSourceCenterStore,
    type SourceCenterStore,
    type SourceDescriptor,
    type SourceFilters,
  } from ".";

  export let store: SourceCenterStore = createSourceCenterStore();
  export let compact = false;

  let sources: SourceDescriptor[] = [];
  let filters: Readonly<SourceFilters> = store.getSnapshot().filters;
  let allSources: SourceDescriptor[] = [];
  let index = store.getSnapshot().extensionIndex;
  let configuredExtensionEndpoint = store.getSnapshot().extensionIndexEndpoint;
  let extensionEndpoint = configuredExtensionEndpoint ?? "";
  let actionKeys: readonly string[] = [];
  let loading = false;
  let refreshing = false;
  let error: string | null = null;
  let announcement = "";

  const unsubscribe = store.subscribe((snapshot) => {
    sources = snapshot.sources;
    allSources = snapshot.allSources;
    filters = snapshot.filters;
    index = snapshot.extensionIndex;
    actionKeys = snapshot.actionKeys;
    loading = snapshot.loading;
    refreshing = snapshot.refreshing;
    error = snapshot.error;
    if (configuredExtensionEndpoint !== snapshot.extensionIndexEndpoint) {
      configuredExtensionEndpoint = snapshot.extensionIndexEndpoint;
      extensionEndpoint = snapshot.extensionIndexEndpoint ?? "";
    }
  });

  onMount(() => {
    void store.load();
    return unsubscribe;
  });

  $: languages = [...new Set(allSources.flatMap((source) => source.languages))].sort();
  $: enabledSources = sources.filter((source) => source.enabled);

  const mediaLabel: Record<string, string> = { all: "全部媒体", anime: "动画", comic: "漫画", external_runtime: "外部运行时" };
  const capabilityLabel: Record<string, string> = { all: "全部能力", probe: "探测", search: "搜索", detail: "详情", children: "子项", resolve: "解析", progress_read: "读取进度", progress_write: "写入进度", download: "下载", verify: "验证" };
  const healthLabel: Record<string, string> = { healthy: "健康", degraded: "降级", open_circuit: "已熔断", disabled: "已禁用", unknown: "未验证" };
  const runtimeLabel: Record<string, string> = { all: "全部运行时", available: "可用", unavailable: "不可用", deferred: "延后连接", unknown: "未知" };
  const authLabel: Record<string, string> = { not_required: "无需认证", configured: "已认证", missing: "缺少认证配置", unknown: "认证状态未知" };

  function formatTime(value: string | null | undefined): string {
    if (!value || !Number.isFinite(Date.parse(value))) return "尚未检查";
    return new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }).format(new Date(value));
  }

  function isRunning(source: SourceDescriptor, action: string): boolean {
    return actionKeys.includes(`${action}:${source.mediaType}:${source.providerId}`);
  }

  function setFilter<K extends keyof SourceFilters>(key: K, value: SourceFilters[K]) {
    void store.setFilters({ [key]: value } as Partial<SourceFilters>);
  }

  async function run(label: string, action: () => Promise<void>) {
    announcement = `${label}：正在处理。`;
    try {
      await action();
      announcement = `${label}：操作已提交。`;
    } catch {
      announcement = `${label}：操作失败，请查看错误提示。`;
    }
  }

  async function saveExtensionEndpoint() {
    await run("保存扩展目录端点", async () => {
      await store.setExtensionIndexEndpoint(extensionEndpoint);
      extensionEndpoint = store.getSnapshot().extensionIndexEndpoint ?? "";
    });
  }
</script>

<section class:compact class="sources" aria-label="来源中心" data-testid="source-center-list">
  <div class="toolbar">
    <div class="filters" aria-label="来源筛选">
      <label>媒体类型<select value={filters.mediaType} onchange={(event) => setFilter("mediaType", (event.currentTarget as HTMLSelectElement).value as SourceFilters["mediaType"])}><option value="all">全部媒体</option>{#each SOURCE_MEDIA_TYPES as value}<option value={value}>{mediaLabel[value]}</option>{/each}</select></label>
      <label>能力<select value={filters.capability} onchange={(event) => setFilter("capability", (event.currentTarget as HTMLSelectElement).value as SourceFilters["capability"])}><option value="all">全部能力</option>{#each SOURCE_CAPABILITIES as value}<option value={value}>{capabilityLabel[value]}</option>{/each}</select></label>
      <label>语言<select value={filters.language} onchange={(event) => setFilter("language", (event.currentTarget as HTMLSelectElement).value)}><option value="all">全部语言</option>{#each languages as language}<option value={language}>{language.toUpperCase()}</option>{/each}</select></label>
      <label>NSFW<select value={filters.nsfw} onchange={(event) => setFilter("nsfw", (event.currentTarget as HTMLSelectElement).value as SourceFilters["nsfw"])}><option value="all">全部策略</option><option value="allow">允许 NSFW</option><option value="exclude">排除 NSFW</option></select></label>
      <label>运行时<select value={filters.runtime} onchange={(event) => setFilter("runtime", (event.currentTarget as HTMLSelectElement).value as SourceFilters["runtime"])}><option value="all">全部运行时</option>{#each SOURCE_RUNTIME_STATES as value}<option value={value}>{runtimeLabel[value]}</option>{/each}</select></label>
    </div>
    <div class="toolbar-actions">
      <Button variant="quiet" size="sm" loading={refreshing} press={() => run("刷新来源", () => store.refresh())} ariaLabel="刷新来源列表"><Icon name="refresh" size={15} />刷新</Button>
      <Button variant="secondary" size="sm" disabled={enabledSources.length === 0 || actionKeys.includes("verify-batch")} press={() => run("批量验证来源", () => store.verifyVisible())}>验证已筛选 {enabledSources.length}</Button>
    </div>
  </div>

  <p class="sr-only" role="status" aria-live="polite">{announcement}</p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <aside class:offline={index?.isOfflineSnapshot === true || Boolean(index?.lastError)} class="extension-index" aria-label="远程扩展目录状态" data-testid="extension-index-state">
    <div>
      <strong>远程扩展目录</strong>
      {#if configuredExtensionEndpoint}
        <span>{index ? `${index.isOfflineSnapshot ? "离线快照" : index.lastError ? "过期快照" : "已缓存"} · ${index.entries.length} 条元数据 · ${formatTime(index.fetchedAt)}` : "已配置端点，尚无目录快照。"}</span>
        {#if index?.isOfflineSnapshot || Boolean(index?.lastError)}<small>仅展示最后成功快照；不下载或执行第三方扩展代码。</small>{/if}
        {#if index?.lastError}<small>{index.lastError}</small>{/if}
      {:else}
        <span>同步已禁用：请先提供并保存受控目录端点。不会使用默认目录，也不会存储凭据。</span>
      {/if}
    </div>
    <div class="extension-index-controls">
      <label>目录端点<input aria-label="扩展目录端点" bind:value={extensionEndpoint} type="url" inputmode="url" autocomplete="off" spellcheck={false} placeholder="https://example.com/extensions.json" /></label>
      <div class="extension-index-actions">
        <Button variant="secondary" size="sm" press={saveExtensionEndpoint}>保存端点</Button>
        {#if configuredExtensionEndpoint}<Button variant="quiet" size="sm" press={() => run("移除扩展目录端点", () => store.clearExtensionIndexEndpoint())}>移除端点</Button>{/if}
        <Button variant="quiet" size="sm" disabled={!configuredExtensionEndpoint || actionKeys.includes("refresh-extension-index")} press={() => run("同步扩展目录", () => store.refreshExtensionIndex())}>同步目录</Button>
      </div>
    </div>
  </aside>

  {#if loading}<div class="empty" role="status">正在加载来源中心…</div>
  {:else if sources.length === 0}<div class="empty"><strong>没有匹配的来源</strong><p>调整筛选条件，或先完成来源配置。</p></div>
  {:else}<div class="source-list" role="list" aria-label="来源列表">
    {#each sources as source (source.mediaType + source.providerId)}
      <article class:disabled={!source.enabled} class:circuit={source.health.state === "open_circuit"} class="source" role="listitem" data-testid={`source-${source.mediaType}-${source.providerId}`}>
        <header><div class="title"><div><h2>{source.displayName}</h2><p>{source.providerId} · {mediaLabel[source.mediaType]} · {source.kind}</p></div><span class:warning={source.health.state === "degraded"} class:bad={source.health.state === "disabled" || source.health.state === "open_circuit"} class="health">{healthLabel[source.health.state]}</span></div>
          <button class="switch" type="button" role="switch" aria-checked={source.enabled} aria-label={`${source.displayName} 已${source.enabled ? "启用" : "禁用"}`} disabled={isRunning(source, "preference")} onclick={() => run(`${source.displayName}启用状态`, () => store.toggleEnabled(source))}><span aria-hidden="true"></span></button>
        </header>
        <div class="metrics"><span>优先级 <b>{source.priority}</b></span><span>延迟 <b>{source.latencyMs === null ? "—" : `${Math.round(source.latencyMs)} ms`}</b></span><span>成功率 <b>{source.health.successRate === null ? "—" : `${Math.round(source.health.successRate * 100)}%`}</b></span><span>连续失败 <b>{source.health.consecutiveFailures}</b></span><span>检查于 <b>{formatTime(source.lastCheckedAt)}</b></span></div>
        <div class="chips">{#each source.capabilities as capability}<span>{capabilityLabel[capability]}</span>{/each}{#each source.languages as language}<span>{language.toUpperCase()}</span>{/each}<span>{source.nsfw === "allow" ? "允许 NSFW" : source.nsfw === "only" ? "仅 NSFW" : source.nsfw === "exclude" ? "排除 NSFW" : "NSFW 未知"}</span><span>{runtimeLabel[source.runtimeState]}</span><span>{authLabel[source.authState]}</span></div>
        <div class="actions" aria-label={`${source.displayName}操作`}><Button variant="quiet" size="sm" disabled={source.priority <= SOURCE_PRIORITY_MIN || isRunning(source, "preference")} press={() => run(`${source.displayName}降低优先级`, () => store.adjustPriority(source, -1))}>− 优先级</Button><Button variant="quiet" size="sm" disabled={source.priority >= SOURCE_PRIORITY_MAX || isRunning(source, "preference")} press={() => run(`${source.displayName}提高优先级`, () => store.adjustPriority(source, 1))}>+ 优先级</Button><Button variant="secondary" size="sm" disabled={isRunning(source, "verify")} press={() => run(`${source.displayName}验证`, () => store.verifySource(source))}>验证</Button><Button variant="quiet" size="sm" disabled={isRunning(source, "reset-health")} press={() => run(`${source.displayName}清除健康状态`, () => store.resetHealth(source))}>清除健康状态</Button></div>
        <details><summary>健康详情与最近失败</summary>{#if source.recentFailures.length === 0}<p>暂无记录的失败事件。</p>{:else}<ul>{#each source.recentFailures as failure}<li><code>{failure.code}</code><span>{failure.message}</span>{#if failure.occurredAt}<time>{formatTime(failure.occurredAt)}</time>{/if}</li>{/each}</ul>{/if}</details>
      </article>
    {/each}
  </div>{/if}
</section>

<style>
  .sources { display: grid; gap: 14px; min-width: 0; color: var(--text-secondary); }
  .toolbar { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; }
  .filters, .toolbar-actions, .actions, .chips, .metrics, .extension-index-actions { display: flex; flex-wrap: wrap; gap: 7px; }
  .filters { flex: 1; }
  .filters label { display: grid; gap: 4px; color: var(--text-muted); font: 700 10px/1 var(--font-mono, monospace); text-transform: uppercase; letter-spacing: .04em; }
  select, .extension-index-controls input { min-width: 100px; min-height: 31px; padding: 0 8px; border: 1px solid var(--border); border-radius: 7px; background: var(--bg-elevated); color: var(--text-primary); font-size: 12px; }
  .extension-index-controls input { width: 100%; min-width: 0; background: var(--bg-card); text-transform: none; letter-spacing: normal; }
  select:focus-visible, .switch:focus-visible, .extension-index-controls input:focus-visible { outline: 2px solid var(--accent-ring); outline-offset: 2px; }
  .error { margin: 0; padding: 9px 11px; border: 1px solid var(--color-error, #f87171); border-radius: 8px; color: var(--color-error, #f87171); font-size: 12px; }
  .extension-index { display: flex; justify-content: space-between; gap: 12px; padding: 11px 12px; border: 1px solid var(--border); border-radius: 9px; background: var(--bg-elevated); }
  .extension-index.offline { border-color: var(--color-warning, #fbbf24); }
  .extension-index > div:first-child, .extension-index-controls, .extension-index-controls label { display: grid; gap: 3px; }
  .extension-index-controls { min-width: min(100%, 330px); }
  .extension-index-controls label { color: var(--text-muted); font: 700 10px/1 var(--font-mono, monospace); letter-spacing: .04em; text-transform: uppercase; }
  .extension-index strong { color: var(--text-primary); font-size: 12px; }
  .extension-index span, .extension-index small { color: var(--text-muted); font-size: 11px; }
  .source-list { display: grid; gap: 10px; }
  .source { padding: 14px; border: 1px solid var(--border); border-radius: 10px; background: var(--bg-card); }
  .source.disabled { opacity: .62; }
  .source.circuit { border-color: var(--color-error, #f87171); }
  header, .title { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; }
  .title { justify-content: flex-start; min-width: 0; }
  h2 { margin: 0; color: var(--text-primary); font-size: 15px; }
  p { margin: 4px 0 0; color: var(--text-muted); font-size: 11px; }
  .health { padding: 3px 7px; border: 1px solid var(--color-success, #4ade80); border-radius: 999px; color: var(--color-success, #4ade80); font: 700 10px/1 var(--font-mono, monospace); white-space: nowrap; }
  .health.warning { border-color: var(--color-warning, #fbbf24); color: var(--color-warning, #fbbf24); }
  .health.bad { border-color: var(--color-error, #f87171); color: var(--color-error, #f87171); }
  .switch { position: relative; flex: 0 0 auto; width: 42px; height: 24px; border: 1px solid var(--border); border-radius: 999px; background: var(--bg-hover); cursor: pointer; }
  .switch span { position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: var(--text-muted); transition: transform .18s ease; }
  .switch[aria-checked="true"] { border-color: var(--accent-ring); background: var(--accent-lo); }
  .switch[aria-checked="true"] span { transform: translateX(18px); background: var(--accent); }
  .switch:disabled { opacity: .5; cursor: not-allowed; }
  .metrics { margin: 13px 0 8px; color: var(--text-muted); font-size: 11px; }
  .metrics b { color: var(--text-secondary); font-family: var(--font-mono, monospace); }
  .chips span { padding: 3px 6px; border: 1px solid var(--border); border-radius: 5px; color: var(--text-muted); background: var(--bg-elevated); font: 700 10px/1 var(--font-mono, monospace); }
  .actions { margin-top: 13px; }
  details { margin-top: 13px; padding-top: 10px; border-top: 1px solid var(--border); }
  summary { cursor: pointer; font-size: 11px; }
  ul { display: grid; gap: 6px; padding: 0; margin: 9px 0 0; list-style: none; }
  li { display: grid; grid-template-columns: max-content minmax(0, 1fr) max-content; gap: 8px; font-size: 11px; color: var(--text-muted); }
  code { color: var(--color-error, #f87171); }
  time { font-family: var(--font-mono, monospace); }
  .empty { min-height: 180px; display: grid; place-content: center; justify-items: center; gap: 7px; border: 1px dashed var(--border); border-radius: 9px; color: var(--text-muted); text-align: center; font-size: 12px; }
  .empty strong { color: var(--text-primary); }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0, 0, 0, 0); }
  @media (max-width: 840px) { .toolbar, .extension-index { flex-direction: column; } .extension-index-controls { width: 100%; } }
  @media (max-width: 560px) { li { grid-template-columns: 1fr; gap: 3px; } }
  @media (prefers-reduced-motion: reduce) { .switch span { transition: none; } }
</style>
