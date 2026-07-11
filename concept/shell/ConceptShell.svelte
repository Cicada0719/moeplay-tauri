<script lang="ts">
  import type { ConceptTemplate, ContentMode, ContentModule, NavigationIntent } from "../contracts";
  import { navigationController } from "../input";
  import TopNavigation from "./TopNavigation.svelte";
  import ModeController from "./ModeController.svelte";
  import TemplateSelector from "./TemplateSelector.svelte";
  import MediaCursor from "./MediaCursor.svelte";

  export let template: ConceptTemplate;
  export let module: ContentModule;
  export let mode: ContentMode;
  export let toneClass = "concept-tone--dark concept-nav--light-ink";
  export let cursorEnabled = true;
  export let cursorLabel = "VIEW";
  export let onTemplateChange: (template: ConceptTemplate) => void;
  export let onModuleChange: (module: ContentModule) => void;
  export let onModeChange: (mode: ContentMode) => void;
  export let onIntent: (intent: NavigationIntent, event: Event) => void = () => undefined;
  export let onSearch: () => void = () => undefined;
  export let onStatus: () => void = () => undefined;
  export let onSettings: () => void = () => undefined;
</script>

<div
  class={`concept-shell ${toneClass}`}
  data-template={template}
  data-module={module}
  data-mode={mode}
  tabindex="-1"
  use:navigationController={{ getMode: () => mode, onIntent }}
>
  <TopNavigation {module} {toneClass} {onModuleChange} {onSearch} {onStatus} {onSettings} />
  <main class="concept-shell__stage"><slot /></main>
  <TemplateSelector {template} onChange={onTemplateChange} />
  <ModeController {mode} onChange={onModeChange} />
  <MediaCursor enabled={cursorEnabled} label={cursorLabel} />
</div>

<style>
  .concept-shell{position:relative;min-height:100dvh;background:#090909;color:#f4f1eb;isolation:isolate;--concept-nav-ink:#f4f1eb}.concept-shell.concept-tone--light{background:#ece9e1;color:#101010;--concept-nav-ink:#101010}.concept-shell__stage{height:100dvh;min-height:100dvh;overflow:auto;overscroll-behavior:contain}.concept-shell:focus{outline:none}
</style>

