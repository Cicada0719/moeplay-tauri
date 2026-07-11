<script module lang="ts">
  import type {
    ConceptTemplate,
    ContentMode,
    ContentModule,
    MotionQuality,
  } from "../contracts";

  export type ReviewViewport = "responsive" | "compact" | "desktop" | "couch";

  export interface ReviewPanelProps {
    open?: boolean;
    template: ConceptTemplate;
    module: ContentModule;
    mode: ContentMode;
    quality: MotionQuality;
    muted: boolean;
    viewport?: ReviewViewport;
    onTemplateChange: (template: ConceptTemplate) => void;
    onModuleChange: (module: ContentModule) => void;
    onModeChange: (mode: ContentMode) => void;
    onQualityChange: (quality: MotionQuality) => void;
    onMutedChange: (muted: boolean) => void;
    onViewportChange: (viewport: ReviewViewport) => void;
    onClose?: () => void;
  }
</script>

<script lang="ts">
  let {
    open = true,
    template,
    module,
    mode,
    quality,
    muted,
    viewport = "responsive",
    onTemplateChange,
    onModuleChange,
    onModeChange,
    onQualityChange,
    onMutedChange,
    onViewportChange,
    onClose = () => undefined,
  }: ReviewPanelProps = $props();

  const templates: ReadonlyArray<{ id: ConceptTemplate; number: string; label: string }> = [
    { id: "cinematic", number: "01", label: "电影" },
    { id: "editorial", number: "02", label: "画报" },
    { id: "kinetic", number: "03", label: "动态" },
  ];

  const modules: ReadonlyArray<{ id: ContentModule; label: string }> = [
    { id: "games", label: "游戏" },
    { id: "anime", label: "动画" },
    { id: "comics", label: "漫画" },
  ];

  const modes: ReadonlyArray<{ id: ContentMode; label: string; key: string }> = [
    { id: "visual", label: "视觉", key: "1" },
    { id: "index", label: "索引", key: "2" },
    { id: "scene", label: "场景", key: "3" },
  ];

  const qualities: ReadonlyArray<{ id: MotionQuality; label: string }> = [
    { id: "full", label: "完整" },
    { id: "balanced", label: "平衡" },
    { id: "reduced", label: "精简" },
  ];

  const viewports: ReadonlyArray<{ id: ReviewViewport; label: string; meta: string }> = [
    { id: "responsive", label: "自适应", meta: "AUTO" },
    { id: "compact", label: "紧凑", meta: "960" },
    { id: "desktop", label: "桌面", meta: "1440" },
    { id: "couch", label: "客厅", meta: "1920" },
  ];
</script>

{#if open}
  <aside class="review-panel" aria-label="概念评审控制台" data-testid="review-panel">
    <header class="panel-header">
      <div>
        <span class="eyebrow">REVIEW / LIVE</span>
        <h2>概念评审台</h2>
      </div>
      <button class="close-button" type="button" aria-label="关闭评审控制台" onclick={onClose}>
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <path d="M5 5l10 10M15 5L5 15" />
        </svg>
      </button>
    </header>

    <section class="control-section" aria-labelledby="review-template-label">
      <div class="section-heading">
        <h3 id="review-template-label">模板</h3>
        <output data-testid="review-template-state">{template}</output>
      </div>
      <div class="template-grid" role="radiogroup" aria-label="模板">
        {#each templates as item}
          <button
            type="button"
            role="radio"
            aria-checked={template === item.id}
            class:active={template === item.id}
            data-testid={`template-${item.id}`}
            onclick={() => onTemplateChange(item.id)}
          >
            <span>{item.number}</span>
            <strong>{item.label}</strong>
          </button>
        {/each}
      </div>
    </section>

    <section class="control-section split" aria-label="内容与视图">
      <div>
        <div class="section-heading">
          <h3>模块</h3>
          <output data-testid="review-module-state">{module}</output>
        </div>
        <div class="segmented" role="radiogroup" aria-label="内容模块">
          {#each modules as item}
            <button
              type="button"
              role="radio"
              aria-checked={module === item.id}
              class:active={module === item.id}
              data-testid={`module-${item.id}`}
              onclick={() => onModuleChange(item.id)}
            >{item.label}</button>
          {/each}
        </div>
      </div>

      <div>
        <div class="section-heading">
          <h3>模式</h3>
          <output data-testid="review-mode-state">{mode}</output>
        </div>
        <div class="mode-list" role="radiogroup" aria-label="内容模式">
          {#each modes as item}
            <button
              type="button"
              role="radio"
              aria-checked={mode === item.id}
              class:active={mode === item.id}
              data-testid={`mode-${item.id}`}
              onclick={() => onModeChange(item.id)}
            >
              <kbd>{item.key}</kbd>
              <span>{item.label}</span>
            </button>
          {/each}
        </div>
      </div>
    </section>

    <section class="control-section split" aria-label="播放质量与声音">
      <div>
        <div class="section-heading">
          <h3>动态质量</h3>
          <output data-testid="review-quality-state">{quality}</output>
        </div>
        <div class="quality-list" role="radiogroup" aria-label="动态质量">
          {#each qualities as item}
            <button
              type="button"
              role="radio"
              aria-checked={quality === item.id}
              class:active={quality === item.id}
              data-testid={`quality-${item.id}`}
              onclick={() => onQualityChange(item.id)}
            >{item.label}</button>
          {/each}
        </div>
      </div>

      <div>
        <div class="section-heading">
          <h3>声音</h3>
          <output data-testid="review-sound-state">{muted ? "muted" : "on"}</output>
        </div>
        <button
          class="sound-toggle"
          class:active={!muted}
          type="button"
          role="switch"
          aria-checked={!muted}
          aria-label={muted ? "开启声音" : "静音"}
          data-testid="sound-toggle"
          onclick={() => onMutedChange(!muted)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 10v4h4l5 4V6L8 10H4Z" />
            {#if muted}<path d="m17 9 4 6m0-6-4 6" />{:else}<path d="M17 9c1.4 1.6 1.4 4.4 0 6m2.5-8.5c3 3 3 8 0 11" />{/if}
          </svg>
          <span>{muted ? "已静音" : "声音开启"}</span>
        </button>
      </div>
    </section>

    <section class="control-section" aria-labelledby="review-viewport-label">
      <div class="section-heading">
        <h3 id="review-viewport-label">Viewport</h3>
        <output data-testid="review-viewport-state">{viewport}</output>
      </div>
      <div class="viewport-grid" role="radiogroup" aria-label="预览尺寸">
        {#each viewports as item}
          <button
            type="button"
            role="radio"
            aria-checked={viewport === item.id}
            class:active={viewport === item.id}
            data-testid={`viewport-${item.id}`}
            onclick={() => onViewportChange(item.id)}
          >
            <span>{item.label}</span>
            <small>{item.meta}</small>
          </button>
        {/each}
      </div>
    </section>

    <footer class="input-guide" data-testid="input-guide" aria-label="键盘与手柄操作提示">
      <div class="guide-title">
        <span>INPUT MAP</span>
        <strong>键盘 / 手柄</strong>
      </div>
      <ul>
        <li><span><kbd>←</kbd><kbd>→</kbd></span><span class="gamepad-key">D-PAD</span><em>移动</em></li>
        <li><span><kbd>↵</kbd></span><span class="gamepad-key">A</span><em>确认</em></li>
        <li><span><kbd>ESC</kbd></span><span class="gamepad-key">B</span><em>返回</em></li>
        <li><span><kbd>⇧</kbd><kbd>←</kbd><kbd>→</kbd></span><span class="gamepad-key">LB / RB</span><em>切换模式</em></li>
      </ul>
    </footer>
  </aside>
{/if}

<style>
  .review-panel {
    --accent: #d7ff45;
    --line: rgba(255, 255, 255, 0.16);
    position: fixed;
    z-index: 120;
    top: 1rem;
    right: 1rem;
    width: min(28rem, calc(100vw - 2rem));
    max-height: calc(100dvh - 2rem);
    overflow: auto;
    color: #f3f3ef;
    background: rgba(13, 14, 13, 0.94);
    border: 1px solid var(--line);
    box-shadow: 0 1.5rem 5rem rgba(0, 0, 0, 0.42);
    font-family: "Outfit", "Microsoft YaHei", sans-serif;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.28) transparent;
  }

  .panel-header,
  .section-heading,
  .guide-title,
  .input-guide li,
  .sound-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-header { padding: 1.25rem 1.25rem 1rem; }
  .eyebrow, .section-heading output, .guide-title span, small, kbd, .gamepad-key {
    font-family: "JetBrains Mono", monospace;
    letter-spacing: 0.08em;
  }
  .eyebrow { color: var(--accent); font-size: 0.65rem; }
  h2 { margin: 0.25rem 0 0; font-size: 1.35rem; letter-spacing: -0.03em; }
  h3 { margin: 0; color: rgba(255,255,255,.67); font-size: .72rem; font-weight: 500; letter-spacing: .1em; text-transform: uppercase; }

  button { color: inherit; background: transparent; border: 0; cursor: pointer; }
  button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .close-button { width: 2.25rem; height: 2.25rem; padding: .55rem; border: 1px solid var(--line); }
  .close-button svg, .sound-toggle svg { width: 100%; fill: none; stroke: currentColor; stroke-width: 1.5; }

  .control-section { padding: 1rem 1.25rem 1.15rem; border-top: 1px solid var(--line); }
  .control-section.split { display: grid; grid-template-columns: 1fr 1fr; gap: 1.25rem; }
  .section-heading { margin-bottom: .7rem; }
  .section-heading output { color: rgba(255,255,255,.4); font-size: .58rem; text-transform: uppercase; }

  .template-grid, .viewport-grid { display: grid; gap: .4rem; }
  .template-grid { grid-template-columns: repeat(3, 1fr); }
  .template-grid button { min-height: 4rem; padding: .65rem; text-align: left; border: 1px solid var(--line); }
  .template-grid button span { display: block; margin-bottom: .6rem; color: rgba(255,255,255,.42); font: .62rem "JetBrains Mono", monospace; }
  .template-grid button strong { font-size: .83rem; font-weight: 500; }

  .segmented, .quality-list { display: grid; gap: .3rem; }
  .segmented { grid-template-columns: repeat(3, 1fr); }
  .segmented button, .quality-list button { min-height: 2.15rem; padding: .35rem .5rem; color: rgba(255,255,255,.58); border: 1px solid var(--line); font-size: .72rem; }
  .quality-list { grid-template-columns: 1fr; }

  .mode-list { display: flex; gap: .3rem; }
  .mode-list button { flex: 1; min-height: 3rem; padding: .35rem; color: rgba(255,255,255,.58); border: 1px solid var(--line); }
  .mode-list kbd { display: block; margin-bottom: .2rem; font-size: .58rem; }
  .mode-list span { font-size: .7rem; }

  button.active { color: #11120e; background: var(--accent); border-color: var(--accent); }
  button.active span, button.active kbd { color: inherit; }

  .sound-toggle { width: 100%; min-height: 3rem; padding: .55rem .7rem; border: 1px solid var(--line); font-size: .72rem; }
  .sound-toggle svg { width: 1.25rem; height: 1.25rem; }

  .viewport-grid { grid-template-columns: repeat(4, 1fr); }
  .viewport-grid button { min-height: 3.25rem; padding: .5rem .25rem; border: 1px solid var(--line); }
  .viewport-grid span, .viewport-grid small { display: block; }
  .viewport-grid span { font-size: .68rem; }
  .viewport-grid small { margin-top: .3rem; color: rgba(255,255,255,.38); font-size: .55rem; }

  .input-guide { padding: 1rem 1.25rem 1.2rem; background: #d7ff45; color: #10110e; }
  .guide-title { margin-bottom: .75rem; }
  .guide-title span { font-size: .56rem; }
  .guide-title strong { font-size: .72rem; }
  .input-guide ul { display: grid; gap: .45rem; margin: 0; padding: 0; list-style: none; }
  .input-guide li { display: grid; grid-template-columns: 5.7rem 4rem 1fr; gap: .5rem; font-size: .66rem; }
  .input-guide li > span:first-child { display: flex; gap: .18rem; }
  kbd, .gamepad-key { min-width: 1.25rem; padding: .12rem .22rem; text-align: center; border: 1px solid rgba(16,17,14,.5); font-size: .54rem; }
  .gamepad-key { justify-self: start; }
  em { justify-self: end; font-style: normal; }

  @media (max-width: 560px) {
    .review-panel { width: calc(100vw - 1rem); max-height: calc(100dvh - 1rem); }
    .control-section.split { grid-template-columns: 1fr; }
  }

  @media (prefers-reduced-motion: no-preference) {
    button { transition: color 140ms ease, background-color 140ms ease, border-color 140ms ease, transform 140ms ease; }
    button:hover { transform: translateY(-1px); }
  }
</style>



