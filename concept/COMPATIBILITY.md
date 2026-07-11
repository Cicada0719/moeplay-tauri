# Concept compatibility and review report

## Verified paths

- Templates: Cinematic / Editorial / Kinetic
- Modules: Games / Anime / Comics
- Modes: Visual / Index / Scene
- Detail: open, back, focus restoration
- Input: keyboard, wheel intent policy, touch, gamepad polling
- Rendering: WebGL2 media stage, context-loss hook, CSS fallback, reduced motion
- State: per-module mode/selection/focus/scroll memory, template and quality persistence

## Viewports

Automated checks cover:

- 900 × 600
- 1200 × 800
- 1920 × 1080
- 2560 × 1080 (21:9)

Each viewport exercises all three templates and all three modes while keeping the global module navigation and bottom mode controls reachable.

## Performance strategy

- Three.js is isolated into a dedicated concept chunk.
- WebGL is limited to media; text, navigation and controls remain semantic DOM.
- Renderer, textures, geometry, listeners, ResizeObserver and animation frames are disposed on unmount.
- Reduced quality and `prefers-reduced-motion` use the CSS image stage.
- Non-active videos are controlled by their template lifecycle; no runtime remote media requests are used.
- Motion only changes transforms, opacity and shader uniforms.

## Accessibility

- Complete keyboard path for module, mode, selection and detail.
- Focus returns to the originating content control after closing detail.
- `prefers-reduced-motion` disables nonessential animation.
- Axe automated checks block serious and critical violations.
- WebGL canvas is decorative; all meaningful content remains in DOM.

## Known concept limitations

- Audio is a state control only; the demo does not ship soundtrack files.
- Handheld gamepad behavior is implemented through the standard Gamepad API and represented in the review panel; CI validates keyboard-equivalent intent paths because virtual gamepad injection is browser-dependent.
- Asset rights are recorded for design review, but this concept package is not a redistribution release.
