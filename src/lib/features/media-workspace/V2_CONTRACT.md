# Media Workspace V2 frozen integration contract

- `model/` is the shared data contract. Do not edit it in parallel work.
- `chroma/` owns browser-side palette loading, caching and CSS variable application.
- `shell/` owns the SHIFTBRAIN product frame and input/navigation chrome.
- `v2/` owns Visual/Scene renderers. The existing production Index renderer remains the source of truth for search/filter/grid until integration.
- `SwitchHome.svelte`, `App.svelte`, and `SystemDock.svelte` are integration-only files owned by the main agent.
- All modes consume `MediaPresentationItem`; no template may read `gameStore` directly.
- Text and controls stay DOM-rendered. Image processing is bounded and never runs per frame.
- Reduced motion must disable inertial/continuous movement while keeping all navigation paths available.
