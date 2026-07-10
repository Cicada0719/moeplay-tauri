<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { onDestroy, onMount } from "svelte";
  import { focusTrap } from "../../../actions/a11y/focusTrap";
  import { openPath, openUrl } from "../../../api";
  import { computePrefetchWindow, pageRetryDelayMs, planPageRetry } from "../../../features/comic/logic";
  import { decideComicTarget, localPathFromFileUrl } from "../../../features/comic/reader";
  import type { ComicProviderDescriptor, ComicResolvedTarget } from "../../../features/comic/types";
  import Icon from "../../Icon.svelte";
  import { Button, EmptyState, Tag } from "../../ui";

  let {
    provider,
    target,
    title,
    chapterTitle,
    onclose,
    onretry,
    returnFocusKey,
  }: {
    provider: ComicProviderDescriptor;
    target: ComicResolvedTarget;
    title: string;
    chapterTitle: string;
    onclose: () => void;
    onretry?: () => Promise<void> | void;
    returnFocusKey?: string;
  } = $props();

  const decision = $derived(decideComicTarget(target, provider));
  let pageIndex = $state(0);
  let pageSources = $state<Record<number, string>>({});
  let pageAttempts = $state<Record<number, number>>({});
  let loadingPage = $state(true);
  let pageError = $state("");
  let retryingTarget = $state(false);
  let generation = 0;
  const controllers = new Map<number, AbortController>();
  const objectUrls = new Set<string>();
  const retryTimers = new Set<ReturnType<typeof setTimeout>>();

  function displayFileUrl(value: string): string {
    const path = localPathFromFileUrl(value);
    return path ? convertFileSrc(path) : value;
  }

  function canRenderLocalImage(path: string): boolean {
    return /\.(avif|bmp|gif|jpe?g|png|webp)$/i.test(path);
  }

  async function loadPage(index: number, attempt = pageAttempts[index] ?? 0, prefetch = false) {
    if (decision.kind !== "images" || !decision.pages[index] || pageSources[index]) return;
    const token = generation;
    const page = decision.pages[index];
    try {
      let source: string;
      if (page.startsWith("file:")) {
        source = displayFileUrl(page);
      } else if (decision.headers.length === 0) {
        source = page;
      } else {
        controllers.get(index)?.abort();
        const controller = new AbortController();
        controllers.set(index, controller);
        const response = await fetch(page, {
          headers: new Headers(decision.headers),
          signal: controller.signal,
          cache: attempt > 0 ? "reload" : "default",
        });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const objectUrl = URL.createObjectURL(await response.blob());
        objectUrls.add(objectUrl);
        source = objectUrl;
      }
      if (token !== generation) return;
      pageSources = { ...pageSources, [index]: source };
      if (!prefetch && index === pageIndex) {
        loadingPage = false;
        pageError = "";
      }
    } catch (error) {
      if (token !== generation || (error instanceof DOMException && error.name === "AbortError")) return;
      if (!prefetch && index === pageIndex) handlePageFailure(index, error);
    }
  }

  function prefetchNeighbors(index: number) {
    if (decision.kind !== "images") return;
    for (const neighbor of computePrefetchWindow(index, decision.pages.length, 2)) {
      void loadPage(neighbor, pageAttempts[neighbor] ?? 0, true);
      const source = pageSources[neighbor];
      if (source && decision.headers.length === 0 && typeof Image !== "undefined") {
        const image = new Image();
        image.src = source;
      }
    }
  }

  function handlePageFailure(index: number, error: unknown = new Error("图片加载失败")) {
    const attempt = pageAttempts[index] ?? 0;
    const plan = planPageRetry(index, attempt, 3, true);
    if (!plan) {
      loadingPage = false;
      pageError = error instanceof Error ? error.message : "当前页加载失败";
      return;
    }
    pageAttempts = { ...pageAttempts, [index]: plan.attempt };
    const currentSource = pageSources[index];
    if (currentSource?.startsWith("blob:")) {
      URL.revokeObjectURL(currentSource);
      objectUrls.delete(currentSource);
    }
    const nextSources = { ...pageSources };
    delete nextSources[index];
    pageSources = nextSources;
    loadingPage = true;
    const timer = setTimeout(() => {
      retryTimers.delete(timer);
      void loadPage(index, plan.attempt);
    }, pageRetryDelayMs(plan.attempt));
    retryTimers.add(timer);
  }

  function movePage(delta: number) {
    if (decision.kind !== "images") return;
    const next = Math.min(decision.pages.length - 1, Math.max(0, pageIndex + delta));
    if (next === pageIndex) return;
    pageIndex = next;
    loadingPage = !pageSources[next];
    pageError = "";
    void loadPage(next);
    prefetchNeighbors(next);
  }

  function closeReader() {
    onclose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.target instanceof HTMLInputElement) return;
    if (event.key === "Escape") closeReader();
    if (event.key === "ArrowLeft") movePage(-1);
    if (event.key === "ArrowRight") movePage(1);
  }

  async function retryTarget() {
    if (!onretry) return;
    retryingTarget = true;
    try { await onretry(); } finally { retryingTarget = false; }
  }

  onMount(() => {
    if (decision.kind === "images") {
      void loadPage(0);
      prefetchNeighbors(0);
    } else {
      loadingPage = false;
    }
  });

  onDestroy(() => {
    generation += 1;
    controllers.forEach((controller) => controller.abort());
    retryTimers.forEach((timer) => clearTimeout(timer));
    objectUrls.forEach((url) => URL.revokeObjectURL(url));
  });
</script>

<div class="reader" role="dialog" aria-modal="true" aria-label={`阅读 ${title}`} tabindex="-1" data-return-focus-key={returnFocusKey} use:focusTrap={{ initialFocus: ".provider-reader-close", returnFocus: false, onEscape: closeReader }} onkeydown={handleKeydown}>
  <header class="reader-bar">
    <Button class="provider-reader-close" variant="quiet" size="sm" press={closeReader}><Icon name="chevronLeft" size={16} />返回章节</Button>
    <div class="reader-title">
      <strong>{title}</strong>
      <span>{chapterTitle}</span>
    </div>
    {#if decision.kind === "images"}
      <Tag variant="muted">{pageIndex + 1} / {decision.pages.length}</Tag>
    {:else}
      <span></span>
    {/if}
  </header>

  {#if decision.kind === "images"}
    <main class="reader-stage">
      <button class="page-hit previous" type="button" aria-label="上一页" disabled={pageIndex === 0} onclick={() => movePage(-1)}></button>
      <div class="page-frame" aria-live="polite">
        {#if loadingPage}
          <div class="page-loading"><span></span><p>正在准备第 {pageIndex + 1} 页</p></div>
        {/if}
        {#if pageError}
          <EmptyState
            icon="refresh"
            title="这一页没有加载成功"
            description={pageError}
            action={{ label: "重试当前页", onclick: () => { pageError = ""; loadingPage = true; pageAttempts = { ...pageAttempts, [pageIndex]: 0 }; void loadPage(pageIndex, 0); } }}
          />
        {:else if pageSources[pageIndex]}
          {#key `${pageIndex}-${pageAttempts[pageIndex] ?? 0}`}
            <img
              src={pageSources[pageIndex]}
              alt={`${chapterTitle} 第 ${pageIndex + 1} 页`}
              onload={() => { loadingPage = false; pageError = ""; prefetchNeighbors(pageIndex); }}
              onerror={() => handlePageFailure(pageIndex)}
            />
          {/key}
        {/if}
      </div>
      <button class="page-hit next" type="button" aria-label="下一页" disabled={pageIndex >= decision.pages.length - 1} onclick={() => movePage(1)}></button>
    </main>
    <footer class="reader-controls">
      <Button variant="secondary" size="sm" press={() => movePage(-1)} disabled={pageIndex === 0}><Icon name="chevronLeft" size={15} />上一页</Button>
      <input aria-label="阅读页码" type="range" min="1" max={decision.pages.length} value={pageIndex + 1} oninput={(event) => {
        const next = Number(event.currentTarget.value) - 1;
        pageIndex = next;
        loadingPage = !pageSources[next];
        pageError = "";
        void loadPage(next);
        prefetchNeighbors(next);
      }} />
      <Button variant="secondary" size="sm" press={() => movePage(1)} disabled={pageIndex >= decision.pages.length - 1}>下一页<Icon name="chevronRight" size={15} /></Button>
    </footer>
  {:else if decision.kind === "local_file"}
    <div class="reader-fallback">
      {#if canRenderLocalImage(decision.path)}
        <img class="single-local-image" src={convertFileSrc(decision.path)} alt={chapterTitle} />
      {:else}
        <EmptyState icon="folder" title="使用系统应用打开本地章节" description="该文件类型不适合在内置图片阅读器中展示。" action={{ label: "打开文件", onclick: () => openPath(decision.path) }} />
      {/if}
    </div>
  {:else if decision.kind === "external"}
    <div class="reader-fallback">
      <EmptyState icon="externalLink" title="需要外部网页阅读器" description={decision.reason} action={{ label: "在系统浏览器中打开", onclick: () => openUrl(decision.url) }} />
    </div>
  {:else}
    <div class="reader-fallback">
      <EmptyState icon="shield" title="无法安全打开此章节" description={decision.reason} action={onretry ? { label: retryingTarget ? "重试中..." : "重新解析", onclick: retryTarget } : undefined} />
    </div>
  {/if}
</div>

<style>
  .reader { position: absolute; inset: 0; z-index: 50; display: grid; grid-template-rows: auto minmax(0,1fr) auto; background: rgba(8,10,14,.985); color: var(--text-primary); }
  .reader-bar { min-height: 58px; padding: 8px 14px; border-bottom: 1px solid var(--border); display: grid; grid-template-columns: minmax(120px,1fr) minmax(0,2fr) minmax(120px,1fr); align-items: center; gap: 12px; background: rgba(13,15,21,.94); }
  .reader-bar > :last-child { justify-self: end; }
  .reader-title { min-width: 0; text-align: center; }
  .reader-title strong, .reader-title span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .reader-title strong { font-size: 13px; }
  .reader-title span { margin-top: 2px; color: var(--text-muted); font-size: 11px; }
  .reader-stage { min-height: 0; display: grid; grid-template-columns: minmax(44px,1fr) minmax(0,900px) minmax(44px,1fr); align-items: stretch; overflow: hidden; }
  .page-frame { position: relative; min-height: 0; display: flex; align-items: center; justify-content: center; overflow: auto; }
  .page-frame img, .single-local-image { display: block; max-width: 100%; max-height: 100%; object-fit: contain; }
  .page-hit { border: 0; background: transparent; cursor: pointer; }
  .page-hit:hover:not(:disabled) { background: linear-gradient(90deg, transparent, rgba(255,255,255,.035)); }
  .page-hit.next:hover:not(:disabled) { background: linear-gradient(270deg, transparent, rgba(255,255,255,.035)); }
  .page-hit:disabled { cursor: default; }
  .page-loading { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; color: var(--text-muted); font-size: 12px; }
  .page-loading span { width: 24px; height: 24px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: spin .8s linear infinite; }
  .reader-controls { padding: 10px 16px; border-top: 1px solid var(--border); display: grid; grid-template-columns: auto minmax(120px,520px) auto; justify-content: center; align-items: center; gap: 16px; background: rgba(13,15,21,.94); }
  .reader-controls input { accent-color: var(--accent); }
  .reader-fallback { min-height: 0; display: flex; align-items: center; justify-content: center; padding: 28px; grid-row: 2 / 4; }
  .single-local-image { max-height: calc(100dvh - 90px); }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 700px) { .reader-bar { grid-template-columns: auto minmax(0,1fr) auto; } .reader-stage { grid-template-columns: 24px minmax(0,1fr) 24px; } .reader-controls { grid-template-columns: auto 1fr auto; gap: 8px; padding-inline: 8px; } }
  @media (prefers-reduced-motion: reduce) { .page-loading span { animation: none; } }
</style>
