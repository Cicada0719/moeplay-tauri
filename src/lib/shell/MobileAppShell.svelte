<script lang="ts">
  import { onDestroy } from "svelte";
  import Icon from "../components/Icon.svelte";
  import { getViewLabel } from "../nav";
  import { closeOverlay, openOverlay } from "../stores/router.svelte";

  interface MobileNavItem {
    id: string;
    label: string;
    icon: string;
    view: string;
  }

  let {
    currentView,
    taskActiveCount = 0,
    taskFailedCount = 0,
    onNavigate,
    onSearch,
  }: {
    currentView: string;
    taskActiveCount?: number;
    taskFailedCount?: number;
    onNavigate: (view: string) => void;
    onSearch: () => void;
  } = $props();

  const primaryItems: readonly MobileNavItem[] = [
    { id: "home", label: "首页", icon: "home", view: "home" },
    { id: "anime", label: "番剧", icon: "film", view: "anime" },
    { id: "comic", label: "漫画", icon: "book", view: "comic" },
    { id: "novel", label: "小说", icon: "collection", view: "novel" },
  ];

  const moreItems: readonly MobileNavItem[] = [
    { id: "continue", label: "继续", icon: "play", view: "continue" },
    { id: "discovery", label: "发现", icon: "compass", view: "discovery" },
    { id: "downloads", label: "下载", icon: "download", view: "downloads" },
    { id: "sources", label: "来源", icon: "layers", view: "sources" },
    { id: "tasks", label: "任务", icon: "list", view: "tasks" },
    { id: "settings", label: "设置", icon: "settings", view: "settings" },
  ];

  let moreOpen = $state(false);
  const MORE_OVERLAY_ID = "mobile-more-sheet";
  const currentLabel = $derived(getViewLabel(currentView));
  const activeInMore = $derived(moreItems.some((item) => item.view === currentView));

  $effect(() => {
    if (moreOpen) {
      openOverlay({ id: MORE_OVERLAY_ID, kind: "drawer", returnFocusKey: null }, () => (moreOpen = false));
    } else {
      closeOverlay(MORE_OVERLAY_ID);
    }
  });

  onDestroy(() => closeOverlay(MORE_OVERLAY_ID));

  function navigate(view: string) {
    moreOpen = false;
    onNavigate(view);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && moreOpen) {
      event.preventDefault();
      moreOpen = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="mobile-shell" data-testid="mobile-app-shell">
  <header class="mobile-topbar">
    <div class="mobile-brand" aria-label="萌游 MoePlay">
      <span class="brand-mark"></span>
      <span class="brand-copy"><small>MOEPLAY</small><strong>{currentLabel}</strong></span>
    </div>
    <div class="top-actions">
      <button type="button" aria-label="搜索当前模块" onclick={onSearch}><Icon name="search" size={20} /></button>
      <button type="button" aria-label="打开更多功能" aria-expanded={moreOpen} onclick={() => (moreOpen = !moreOpen)}>
        <Icon name="toolbox" size={20} />
        {#if taskFailedCount > 0}<span class="alert-dot" aria-label={`${taskFailedCount} 个失败任务`}></span>{/if}
      </button>
    </div>
  </header>

  <nav class="mobile-rail" aria-label="移动端主导航">
    {#each primaryItems as item (item.id)}
      <button type="button" class:active={currentView === item.view} aria-current={currentView === item.view ? "page" : undefined} onclick={() => navigate(item.view)}>
        <Icon name={item.icon} size={21} />
        <span>{item.label}</span>
      </button>
    {/each}
    <button type="button" class:active={activeInMore} aria-expanded={moreOpen} onclick={() => (moreOpen = !moreOpen)}>
      <Icon name="toolbox" size={21} /><span>更多</span>
      {#if taskActiveCount > 0}<b>{taskActiveCount}</b>{/if}
    </button>
  </nav>

  <nav class="mobile-bottom-nav" aria-label="移动端主导航">
    {#each primaryItems as item (item.id)}
      <button type="button" class:active={currentView === item.view} aria-current={currentView === item.view ? "page" : undefined} onclick={() => navigate(item.view)}>
        <Icon name={item.icon} size={21} /><span>{item.label}</span>
      </button>
    {/each}
    <button type="button" class:active={activeInMore} aria-expanded={moreOpen} onclick={() => (moreOpen = !moreOpen)}>
      <span class="more-icon"><Icon name="toolbox" size={21} />{#if taskActiveCount > 0}<b>{taskActiveCount}</b>{/if}</span>
      <span>更多</span>
    </button>
  </nav>

  {#if moreOpen}
    <button class="mobile-scrim" type="button" aria-label="关闭更多功能" onclick={() => (moreOpen = false)}></button>
    <section class="mobile-more-sheet" aria-label="更多功能">
      <div class="sheet-handle"></div>
      <header><div><small>UTILITY DECK</small><h2>更多功能</h2></div><button type="button" aria-label="关闭" onclick={() => (moreOpen = false)}><Icon name="x" size={20} /></button></header>
      <div class="more-grid">
        {#each moreItems as item (item.id)}
          <button type="button" class:active={currentView === item.view} onclick={() => navigate(item.view)}>
            <span class="more-glyph"><Icon name={item.icon} size={23} /></span>
            <span>{item.label}</span>
            {#if item.view === "tasks" && (taskActiveCount > 0 || taskFailedCount > 0)}
              <small class:failed={taskFailedCount > 0}>{taskFailedCount > 0 ? `${taskFailedCount} 失败` : `${taskActiveCount} 进行中`}</small>
            {/if}
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .mobile-shell { color: var(--text-primary); }
  .mobile-topbar, .mobile-bottom-nav, .mobile-rail { position: fixed; z-index: 96; background: rgba(5, 5, 5, .94); backdrop-filter: blur(18px); -webkit-backdrop-filter: blur(18px); }
  .mobile-topbar { top: 0; left: 0; right: 0; height: calc(56px + env(safe-area-inset-top)); padding: env(safe-area-inset-top) max(12px, env(safe-area-inset-right)) 0 max(14px, env(safe-area-inset-left)); display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border); }
  .mobile-brand { min-width: 0; display: flex; align-items: center; gap: 10px; }
  .brand-mark { width: 9px; height: 28px; background: var(--accent); box-shadow: 0 0 22px color-mix(in srgb, var(--accent) 55%, transparent); }
  .brand-copy { min-width: 0; display: grid; gap: 3px; }
  .brand-copy small { color: var(--text-muted); font: 650 7px/1 var(--font-mono); letter-spacing: .18em; }
  .brand-copy strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font: 700 14px/1 var(--font-ui); letter-spacing: .08em; }
  .top-actions { display: flex; align-items: center; }
  .top-actions button, .mobile-more-sheet header button { position: relative; width: 48px; height: 48px; display: grid; place-items: center; border: 0; background: transparent; color: var(--text-secondary); }
  .alert-dot { position: absolute; top: 10px; right: 9px; width: 7px; height: 7px; border-radius: 50%; background: var(--danger, #ff5f6d); }

  .mobile-bottom-nav { left: 0; right: 0; bottom: 0; min-height: calc(64px + env(safe-area-inset-bottom)); padding: 0 max(4px, env(safe-area-inset-right)) env(safe-area-inset-bottom) max(4px, env(safe-area-inset-left)); display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); border-top: 1px solid var(--border); }
  .mobile-bottom-nav button, .mobile-rail button { position: relative; min-width: 0; min-height: 56px; display: grid; place-items: center; align-content: center; gap: 5px; border: 0; background: transparent; color: var(--text-muted); font: 650 9px/1 var(--font-ui); letter-spacing: .05em; }
  .mobile-bottom-nav button.active, .mobile-rail button.active { color: var(--text-primary); }
  .mobile-bottom-nav button.active::before { content: ""; position: absolute; top: 0; left: 24%; right: 24%; height: 2px; background: var(--accent); }
  .more-icon { position: relative; display: grid; place-items: center; }
  .more-icon b, .mobile-rail b { position: absolute; top: -7px; right: -13px; min-width: 17px; height: 17px; padding: 0 4px; display: grid; place-items: center; border-radius: 10px; background: var(--accent); color: #050505; font: 800 9px/1 var(--font-mono); }

  .mobile-rail { display: none; }
  .mobile-scrim { position: fixed; inset: 0; z-index: 97; width: 100%; border: 0; background: rgba(0, 0, 0, .64); }
  .mobile-more-sheet { position: fixed; z-index: 98; left: max(8px, env(safe-area-inset-left)); right: max(8px, env(safe-area-inset-right)); bottom: calc(70px + env(safe-area-inset-bottom)); max-height: min(70dvh, 520px); overflow: auto; border: 1px solid var(--border-hover); background: rgba(10, 10, 12, .98); box-shadow: 0 -24px 80px rgba(0,0,0,.55); }
  .sheet-handle { width: 42px; height: 4px; margin: 8px auto 2px; border-radius: 4px; background: var(--border-hover); }
  .mobile-more-sheet header { min-height: 64px; display: flex; align-items: center; justify-content: space-between; padding: 8px 10px 8px 18px; border-bottom: 1px solid var(--border); }
  .mobile-more-sheet header small { color: var(--accent); font: 650 7px/1 var(--font-mono); letter-spacing: .15em; }
  .mobile-more-sheet h2 { margin: 4px 0 0; font-size: 17px; }
  .more-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .more-grid > button { min-height: 86px; display: grid; grid-template-columns: 38px minmax(0, 1fr); align-items: center; gap: 10px; padding: 12px; border: 0; border-right: 1px solid var(--border); border-bottom: 1px solid var(--border); background: transparent; color: var(--text-secondary); text-align: left; }
  .more-grid > button.active { color: var(--text-primary); box-shadow: inset 2px 0 var(--accent); background: color-mix(in srgb, var(--accent) 7%, transparent); }
  .more-glyph { width: 38px; height: 38px; display: grid; place-items: center; border: 1px solid var(--border); }
  .more-grid small { grid-column: 2; margin-top: -18px; color: var(--text-muted); font-size: 9px; }
  .more-grid small.failed { color: var(--danger, #ff5f6d); }

  @media (orientation: landscape) and (max-height: 600px) {
    .mobile-topbar, .mobile-bottom-nav { display: none; }
    .mobile-rail { top: 0; bottom: 0; left: 0; width: calc(72px + env(safe-area-inset-left)); padding: max(8px, env(safe-area-inset-top)) 0 max(8px, env(safe-area-inset-bottom)) env(safe-area-inset-left); display: grid; grid-template-rows: repeat(5, minmax(52px, 1fr)); border-right: 1px solid var(--border); }
    .mobile-rail button.active::before { content: ""; position: absolute; left: 0; top: 25%; bottom: 25%; width: 2px; background: var(--accent); }
    .mobile-more-sheet { left: calc(80px + env(safe-area-inset-left)); bottom: max(8px, env(safe-area-inset-bottom)); max-width: 540px; }
  }

  @media (max-width: 360px) { .mobile-bottom-nav button { font-size: 8px; } }
  @media (prefers-reduced-motion: reduce) { .mobile-more-sheet { scroll-behavior: auto; } }
</style>
