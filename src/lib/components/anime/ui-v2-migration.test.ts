import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function source(path: string): string {
  return readFileSync(resolve(process.cwd(), path), "utf8");
}

describe("anime UI-v2 migration contract", () => {
  it("uses the shared page primitives inside an editorial content stage", () => {
    const page = source("src/lib/components/AnimePage.svelte");
    const editorial = source("src/lib/components/anime/editorial/AnimeEditorialHome.svelte");
    expect(page).toContain('<PageShell as="div"');
    expect(page).toContain("<AnimeEditorialHome");
    expect(page).toContain("<AsyncSection");
    expect(page).toContain("<MediaCard");
    expect(page).toContain('data-search-scope="anime"');
    expect(editorial).toContain("editorial");
    expect(editorial).toContain("onResumeHistory");
    expect(editorial).toContain("onOpenSubject");
  });

  it("locks tab semantics and roving keyboard navigation into classic and provider views", () => {
    const files = [
      "src/lib/components/AnimePage.svelte",
      "src/lib/components/anime/AnimeDetail.svelte",
      "src/lib/components/anime/SearchDrawer.svelte",
      "src/lib/components/anime/SourceSheet.svelte",
      "src/lib/components/anime/provider-v2/ProviderConfigPanel.svelte",
    ];
    for (const file of files) {
      const text = source(file);
      expect(text).toContain('role="tablist"');
      expect(text).toContain('role="tab"');
      expect(text).toContain("nextRovingIndex");
      expect(text).toContain("tabindex=");
    }
  });

  it("uses modal focus management and reduced-motion fallbacks for details, drawers and players", () => {
    const detail = source("src/lib/components/anime/AnimeDetail.svelte");
    const search = source("src/lib/components/anime/SearchDrawer.svelte");
    const sourceSheet = source("src/lib/components/anime/SourceSheet.svelte");
    const player = source("src/lib/components/anime/AnimePlayer.svelte");
    const providerPlayer = source("src/lib/components/anime/provider-v2/ProviderV2Player.svelte");

    expect(detail).toContain("<DetailPanel");
    expect(detail).toContain("{returnFocus}");
    expect(search).toContain("<Drawer");
    expect(sourceSheet).toContain("<Drawer");
    expect(player).toContain("use:focusTrap");
    expect(player).toContain("data-episode-key");
    expect(providerPlayer).toContain("use:focusTrap");
    for (const text of [detail, search, sourceSheet, player, providerPlayer]) {
      expect(text).toContain("prefers-reduced-motion: reduce");
      expect(text).toContain('data-motion="reduce"');
    }
  });
});
