<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import Icon from "../../components/Icon.svelte";
  import { attachGamepad, type GamepadAttachment } from "../../components/switch/useGamepad.svelte";
  import { closeOverlay, openOverlay } from "../../stores/router.svelte";

  export type HandheldRouteId = "home" | "game-library" | "anime" | "comic" | "novel" | "settings" | "more";

  interface RouteNavItem {
    id: Exclude<HandheldRouteId, "more">;
    label: string;
    icon: string;
    view: string;
  }

  interface QuickItem {
    id: string;
    label: string;
    description: string;
    icon: string;
    view: string;
  }

  interface Props {
    active: HandheldRouteId;
    title: string;
    subtitle?: string;
    onBack: () => void;
    onNavigate: (view: string) => void;
    children: Snippet;
  }

  let { active, title, subtitle = "横屏掌机 · 内容与功能已统一收纳", onBack, onNavigate, children }: Props = $props();

  const routeNav: RouteNavItem[] = [
    { id: "home", label: "主屏幕", icon: "home", view: "home" },
    { id: "game-library", label: "游戏库", icon: "gamepad", view: "game-library" },
    { id: "anime", label: "番剧", icon: "tv", view: "anime" },
    { id: "comic", label: "漫画", icon: "image", view: "comic" },
    { id: "novel", label: "小说", icon: "book", view: "novel" },
  ];

  const quickItems: QuickItem[] = [
    { id: "import", label: "模拟器导入", description: "扫描 ROM 并自动匹配平台", icon: "database", view: "handheld-import" },
    { id: "continue", label: "继续内容", description: "统一打开最近游玩、观看与阅读", icon: "play", view: "continue" },
    { id: "sources", label: "来源中心", description: "管理番剧、漫画和小说来源", icon: "layers", view: "sources" },
    { id: "downloads", label: "下载任务", description: "查看下载、缓存和后台任务", icon: "download", view: "downloads" },
    { id: "records", label: "历史记录", description: "查看全部游玩与媒体历史", icon: "chart", view: "records" },
    { id: "settings", label: "掌机设置", description: "主题、方向、沉浸式与手柄", icon: "settings", view: "settings" },
    { id: "diagnostics", label: "诊断工具", description: "检查设备、规则和连接状态", icon: "toolbox", view: "diagnostics" },
  ];

  let menuOpen = $state(false);
  let menuIndex = $state(0);
  let menuEl = $state<HTMLElement>();
  let pad: GamepadAttachment | null = null;
  const MENU_OVERLAY_ID = "handheld-route-menu";

  function focusNav(index: number) {
    requestAnimationFrame(() => {
      document.querySelector<HTMLElement>(`[data-hh-route-nav-index="${index}"]`)?.focus({ preventScroll: true });
    });
  }

  function moveNav(delta: number) {
    const current = Math.max(0, routeNav.findIndex((item) => item.id === active));
    const next = (current + delta + routeNav.length) % routeNav.length;
    focusNav(next);
    if (routeNav[next]) onNavigate(routeNav[next].view);
  }

  function focusMenu() {
    requestAnimationFrame(() => {
      menuEl?.querySelector<HTMLElement>(`[data-hh-quick-index="${menuIndex}"]`)?.focus({ preventScroll: true });
    });
  }

  function openMenu() {
    menuIndex = 0;
    menuOpen = true;
    openOverlay({ id: MENU_OVERLAY_ID, kind: "drawer" }, () => { menuOpen = false; });
    focusMenu();
  }

  function closeMenu() {
    menuOpen = false;
    closeOverlay(MENU_OVERLAY_ID);
  }

  function moveMenu(delta: number) {
    menuIndex = (menuIndex + delta + quickItems.length) % quickItems.length;
    focusMenu();
  }

  function activateMenu() {
    const item = quickItems[menuIndex];
    if (!item) return;
    closeMenu();
    onNavigate(item.view);
  }

  function handleBack() {
    if (menuOpen) closeMenu();
    else onBack();
  }

  function routeHandlers() {
    return {
      categoryLeft: () => menuOpen ? moveMenu(-1) : moveNav(-1),
      categoryRight: () => menuOpen ? moveMenu(1) : moveNav(1),
      launch: () => menuOpen ? activateMenu() : undefined,
      favorite: () => menuOpen ? moveMenu(-1) : openMenu(),
      filter: () => menuOpen ? moveMenu(1) : openMenu(),
      back: handleBack,
      start: () => menuOpen ? closeMenu() : onNavigate("handheld-import"),
    };
  }

  onMount(() => {
    pad = attachGamepad(routeHandlers(), { id: "handheld-route-shell", zone: "content", priority: 70 });
    return () => {
      pad?.();
      pad = null;
      closeOverlay(MENU_OVERLAY_ID);
    };
  });
</script>

<section class="hh-route-shell hh-route-shell--{active}" data-testid="handheld-route-shell" data-route-active={active}>
  <header class="hh-route-topbar">
    <div class="hh-route-brand">
      <button class="hh-route-back" type="button" aria-label="返回掌机主屏幕" onclick={onBack}>
        <Icon name="arrowLeft" size={17} />
      </button>
      <button class="hh-route-logo" type="button" aria-label="返回掌机主屏幕" onclick={() => onNavigate("home")}>
        <strong>萌游</strong><small>PORTABLE / XMB</small>
      </button>
      <div class="hh-route-context"><span>掌机工作区</span><b>{title}</b></div>
    </div>

    <nav class="hh-route-nav" aria-label="掌机主导航">
      {#each routeNav as item, index (item.id)}
        <button
          type="button"
          class:active={active === item.id || (active === "game-library" && item.id === "game-library")}
          aria-current={active === item.id ? "page" : undefined}
          data-hh-route-nav-index={index}
          onclick={() => onNavigate(item.view)}
        >
          <Icon name={item.icon} size={16} /><span>{item.label}</span>
        </button>
      {/each}
    </nav>

    <div class="hh-route-actions">
      <button type="button" class="hh-route-import" aria-label="打开模拟器导入" onclick={() => onNavigate("handheld-import")}><Icon name="database" size={15} /><span>导入</span></button>
      <button type="button" class="hh-route-more" aria-label="打开全部功能" aria-expanded={menuOpen} onclick={() => menuOpen ? closeMenu() : openMenu()}><Icon name="grid" size={17} /><span>功能</span></button>
    </div>
  </header>

  <div class="hh-route-subbar">
    <span class="hh-route-status"><i></i> {subtitle}</span>
    <span class="hh-route-hint"><b>LT</b><b>RT</b> 切换栏目</span>
  </div>

  <main class="hh-route-content">{@render children()}</main>

  <footer class="hh-route-footer" aria-label="掌机操作提示">
    <span><b>A</b>打开</span><span><b>B</b>返回</span><span><b>LT / RT</b>切换栏目</span><span><b>START</b>导入</span><span><b>Y</b>功能</span>
  </footer>

  {#if menuOpen}
    <button class="hh-route-scrim" type="button" aria-label="关闭全部功能" onclick={closeMenu}></button>
    <div class="hh-route-menu" bind:this={menuEl} role="dialog" aria-modal="true" aria-label="全部功能">
      <header><div><span>MOEPLAY / UTILITY DECK</span><h2>全部功能</h2></div><button type="button" aria-label="关闭全部功能" onclick={closeMenu}><Icon name="x" size={18} /></button></header>
      <p class="hh-route-menu-intro">常用入口集中在这里，内容页面保持干净，B 键逐层返回。</p>
      <div class="hh-route-menu-grid">
        {#each quickItems as item, index (item.id)}
          <button type="button" class:active={index === menuIndex} data-hh-quick-index={index} onclick={() => { closeMenu(); onNavigate(item.view); }}>
            <span class="hh-route-menu-icon"><Icon name={item.icon} size={18} /></span>
            <span><strong>{item.label}</strong><small>{item.description}</small></span>
          </button>
        {/each}
      </div>
      <footer><span><b>LT / RT</b>选择</span><span><b>A</b>打开</span><span><b>B</b>关闭</span></footer>
    </div>
  {/if}
</section>

<style>
  .hh-route-shell {
    --hh-route-line: color-mix(in srgb, var(--text-primary) 14%, transparent);
    --hh-route-panel: color-mix(in srgb, var(--bg-void) 94%, var(--bg-card));
    position: relative;
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    isolation: isolate;
    color: var(--text-primary);
    background: radial-gradient(circle at 84% 0%, color-mix(in srgb, var(--accent) 11%, transparent), transparent 34%), #07090d;
  }
  .hh-route-topbar { position: relative; z-index: 2; display: grid; grid-template-columns: max-content minmax(0, 1fr) max-content; align-items: center; gap: clamp(8px, 1.4vw, 20px); min-height: 58px; padding: max(8px, env(safe-area-inset-top)) max(14px, env(safe-area-inset-right)) 7px max(14px, env(safe-area-inset-left)); border-bottom: 1px solid var(--hh-route-line); background: linear-gradient(180deg, rgb(5 7 12 / .95), rgb(5 7 12 / .78)); }
  .hh-route-brand { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .hh-route-back { display: grid; width: 36px; height: 36px; flex: 0 0 auto; place-items: center; border: 1px solid var(--hh-route-line); border-radius: 8px; background: rgb(255 255 255 / .04); color: var(--text-secondary); cursor: pointer; }
  .hh-route-back:hover, .hh-route-back:focus-visible { border-color: var(--accent-ring); color: var(--accent-hi); }
  .hh-route-logo { display: grid; gap: 3px; min-width: 92px; padding: 0; border: 0; background: transparent; text-align: left; cursor: pointer; }
  .hh-route-logo strong { color: var(--accent); font: 850 1.18rem/1 var(--font-display); letter-spacing: .05em; text-shadow: 0 0 18px color-mix(in srgb, var(--accent) 38%, transparent); }
  .hh-route-logo small, .hh-route-context span, .hh-route-subbar, .hh-route-footer { color: var(--text-muted); font: 700 .54rem/1 var(--font-mono); letter-spacing: .1em; }
  .hh-route-context { display: grid; gap: 4px; min-width: 92px; padding-left: 12px; border-left: 1px solid var(--hh-route-line); }
  .hh-route-context b { overflow: hidden; color: var(--text-primary); font: 750 .78rem/1 var(--font-ui); letter-spacing: 0; text-overflow: ellipsis; white-space: nowrap; }
  .hh-route-nav { display: flex; align-items: center; justify-content: center; gap: 3px; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  .hh-route-nav::-webkit-scrollbar { display: none; }
  .hh-route-nav button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; min-width: 64px; min-height: 40px; padding: 0 8px; border: 1px solid transparent; border-radius: 7px; background: transparent; color: var(--text-muted); font: 700 .68rem/1 var(--font-ui); white-space: nowrap; cursor: pointer; transition: color 160ms ease, background 160ms ease, border-color 160ms ease, transform 160ms ease; }
  .hh-route-nav button:hover { color: var(--text-primary); background: rgb(255 255 255 / .04); }
  .hh-route-nav button.active { border-color: color-mix(in srgb, var(--accent) 68%, transparent); background: color-mix(in srgb, var(--accent) 14%, transparent); color: var(--text-primary); transform: translateY(-1px); }
  .hh-route-nav button:focus-visible, .hh-route-actions button:focus-visible, .hh-route-menu button:focus-visible { outline: 2px solid var(--accent-hi); outline-offset: 2px; }
  .hh-route-actions { display: flex; gap: 6px; min-width: max-content; }
  .hh-route-actions button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; min-height: 36px; padding: 0 9px; border: 1px solid var(--hh-route-line); border-radius: 7px; background: rgb(255 255 255 / .04); color: var(--text-secondary); font: 700 .64rem/1 var(--font-ui); cursor: pointer; }
  .hh-route-actions .hh-route-import { border-color: color-mix(in srgb, var(--accent) 50%, transparent); color: var(--accent-hi); }
  .hh-route-subbar { position: relative; z-index: 2; display: flex; align-items: center; justify-content: space-between; gap: 10px; min-height: 28px; padding: 0 max(14px, env(safe-area-inset-right)) 0 max(14px, env(safe-area-inset-left)); border-bottom: 1px solid color-mix(in srgb, var(--hh-route-line) 80%, transparent); background: rgb(5 7 12 / .56); letter-spacing: .06em; }
  .hh-route-status { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hh-route-status i { display: inline-block; width: 6px; height: 6px; margin-right: 4px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 10px color-mix(in srgb, var(--accent) 60%, transparent); }
  .hh-route-hint { display: inline-flex; align-items: center; gap: 4px; flex: 0 0 auto; }
  .hh-route-hint b, .hh-route-footer b, .hh-route-menu footer b { color: var(--accent-hi); font-weight: 800; }
  .hh-route-content { position: relative; z-index: 1; min-width: 0; min-height: 0; overflow: hidden; }
  .hh-route-footer { position: relative; z-index: 2; display: flex; align-items: center; justify-content: center; flex-wrap: wrap; gap: 8px 18px; min-height: 30px; padding: 4px max(14px, env(safe-area-inset-right)) max(5px, env(safe-area-inset-bottom)) max(14px, env(safe-area-inset-left)); border-top: 1px solid var(--hh-route-line); background: rgb(5 7 12 / .86); }
  .hh-route-footer span { white-space: nowrap; }
  .hh-route-scrim { position: fixed; inset: 0; z-index: 20; border: 0; background: rgb(0 0 0 / .62); cursor: pointer; }
  .hh-route-menu { position: absolute; z-index: 21; top: max(8px, env(safe-area-inset-top)); right: max(10px, env(safe-area-inset-right)); bottom: max(8px, env(safe-area-inset-bottom)); width: min(520px, calc(100% - 20px)); overflow: auto; border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border)); background: var(--hh-route-panel); box-shadow: -18px 16px 60px rgb(0 0 0 / .44); }
  .hh-route-menu header { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 15px 18px 12px; border-bottom: 1px solid var(--hh-route-line); }
  .hh-route-menu header span { color: var(--accent); font: 700 .55rem/1 var(--font-mono); letter-spacing: .14em; }
  .hh-route-menu h2 { margin: 6px 0 0; font: 850 1.28rem/1 var(--font-display); }
  .hh-route-menu header button { display: grid; width: 36px; height: 36px; place-items: center; border: 1px solid var(--hh-route-line); background: transparent; color: var(--text-secondary); cursor: pointer; }
  .hh-route-menu-intro { margin: 0; padding: 12px 18px 4px; color: var(--text-muted); font-size: .7rem; line-height: 1.5; }
  .hh-route-menu-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 7px; padding: 12px 18px; }
  .hh-route-menu-grid > button { display: grid; grid-template-columns: 32px minmax(0, 1fr); align-items: center; gap: 8px; min-height: 58px; padding: 8px; border: 1px solid var(--hh-route-line); background: rgb(255 255 255 / .025); color: var(--text-secondary); text-align: left; cursor: pointer; }
  .hh-route-menu-grid > button.active, .hh-route-menu-grid > button:hover { border-color: var(--accent-ring); background: color-mix(in srgb, var(--accent) 10%, transparent); color: var(--text-primary); }
  .hh-route-menu-icon { display: grid; width: 30px; height: 30px; place-items: center; border: 1px solid var(--hh-route-line); color: var(--accent-hi); }
  .hh-route-menu-grid > button > span:last-child { display: grid; gap: 4px; min-width: 0; }
  .hh-route-menu-grid strong, .hh-route-menu-grid small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hh-route-menu-grid strong { font-size: .72rem; }
  .hh-route-menu-grid small { color: var(--text-muted); font-size: .58rem; }
  .hh-route-menu footer { display: flex; justify-content: flex-end; gap: 14px; padding: 8px 18px 12px; color: var(--text-muted); font-size: .62rem; }
  @media (orientation: landscape) and (max-height: 620px) {
    .hh-route-topbar { min-height: 52px; padding-block: max(5px, env(safe-area-inset-top)) 4px; }
    .hh-route-back { width: 32px; height: 32px; }
    .hh-route-logo { min-width: 76px; }
    .hh-route-logo strong { font-size: 1rem; }
    .hh-route-context { min-width: 72px; }
    .hh-route-nav button { min-width: 58px; min-height: 36px; padding-inline: 6px; font-size: .62rem; }
    .hh-route-nav button :global(svg) { width: 14px; height: 14px; }
    .hh-route-actions button { min-height: 32px; padding-inline: 7px; }
    .hh-route-subbar { min-height: 24px; }
    .hh-route-footer { min-height: 26px; gap: 5px 12px; font-size: .5rem; }
    .hh-route-menu-grid { gap: 5px; padding-block: 8px; }
    .hh-route-menu-grid > button { min-height: 48px; }
  }
  @media (max-width: 660px) {
    .hh-route-topbar { grid-template-columns: max-content minmax(0, 1fr) max-content; }
    .hh-route-context { display: none; }
    .hh-route-nav { justify-content: flex-start; }
    .hh-route-nav button { min-width: 58px; }
    .hh-route-actions button span { display: none; }
    .hh-route-menu-grid { grid-template-columns: 1fr; }
  }
  @media (prefers-reduced-motion: reduce) { .hh-route-nav button { transition: none; } }

  /* 统一外壳后，子页面只负责内容，不重复绘制桌面/移动端标题栏。 */
  .hh-route-shell--handheld-import :global(.ip-topbar) { display: none; }
  .hh-route-shell--handheld-import :global(.import-page) { height: 100%; min-height: 0; }
  .hh-route-shell--settings :global(.settings-v2-shell) { height: 100%; min-height: 0; }
  .hh-route-shell--settings :global(.settings-v2-shell > .v2-page-shell__inner > .stg > .stg-header) { display: none; }
  .hh-route-shell--settings :global(.settings-v2-shell > .v2-page-shell__inner > .stg) { height: 100%; min-height: 0; }
  .hh-route-shell--settings :global(.hh-settings) { height: 100%; max-width: none; padding-top: 8px; }
  .hh-route-shell--game-library :global(.library-page-shell), .hh-route-shell--game-detail :global(.game-detail-panel) { height: 100%; min-height: 0; }
</style>
