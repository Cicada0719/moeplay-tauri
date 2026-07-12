import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const source = (path: string) => readFileSync(resolve(process.cwd(), path), "utf8");

describe("desktop window lifecycle contract", () => {
  it("makes the close button deterministic and exposes tray restore", () => {
    const app = source("src/App.svelte");
    const backend = source("src-tauri/src/lib.rs");
    const settings = source("src/lib/components/SettingsPage.svelte");
    expect(app).toContain("getCurrentWindow");
    expect(backend).toContain("WindowEvent::CloseRequested");
    expect(backend).toContain("get_settings().minimize_to_tray");
    expect(backend).toContain("window.app_handle().exit(0)");
    expect(backend).toContain('TrayIconBuilder::with_id("moeplay-main")');
    expect(backend).toContain('"tray-quit" => app.exit(0)');
    expect(settings).toContain("点击右上角关闭");
    expect(settings).toContain("驻留托盘");
    expect(settings).toContain('scrollToSettings("settings-appearance")');
    expect(settings).not.toContain('href="#settings-appearance"');
    expect(settings).toContain('setBooleanSetting("ai_enabled"');
  });

  it("keeps the game stage media-led and the detail document full width", () => {
    const visual = source("src/lib/features/media-workspace/styles/game-visual.css");
    const detail = source("src/lib/styles/scheme-c.css");
    const navigation = source("src/lib/shell/GlobalTopNavigation.svelte");
    expect(visual).toContain("grid-template-columns:minmax(0,3fr) minmax(360px,2fr)");
    expect(visual).toContain(".nd-cover-window");
    expect(detail).toContain(".v2-detail-panel.game-detail-panel { width: 100vw !important; }");
    expect(navigation).toContain('<span class="utility-label">全屏</span>');
    expect(navigation).toContain('class:active={windowFullscreen}');
  });
});
