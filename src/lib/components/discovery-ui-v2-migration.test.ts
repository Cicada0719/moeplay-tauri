import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function source(path: string): string {
  return readFileSync(resolve(process.cwd(), path), "utf8");
}

describe("discovery UI-v2 migration contract", () => {
  it("rebuilds the page skeleton on shared ui-v2 primitives with the halftone grain", () => {
    const page = source("src/lib/components/DiscoveryPage.svelte");
    expect(page).toContain('<PageShell as="div"');
    expect(page).toContain("<PageHeader");
    expect(page).toContain("<FilterBar");
    expect(page).toContain("<StateBoundary");
    expect(page).toContain("<ContentGrid");
    expect(page).toContain('class="v2-grain dc-grain"');
    expect(page).toContain("発見 / DISCOVERY");
    expect(page).toContain('id="discovery-page-title"');
    // Legacy skeleton is gone.
    expect(page).not.toContain("SakuraParticles");
    expect(page).not.toContain('from "./ui/Rail.svelte"');
  });

  it("keeps all seven discovery tabs wired through an i18n-backed segment control", () => {
    const page = source("src/lib/components/DiscoveryPage.svelte");
    for (const key of [
      "discovery.tab_search",
      "discovery.tab_recommend",
      "discovery.tab_ai",
      "discovery.tab_developers",
      "discovery.tab_tags",
      "discovery.tab_years",
      "discovery.tab_ratings",
    ]) {
      expect(page).toContain(key);
    }
    expect(page).toContain("<SegmentControl");
    for (const branch of ['active === "search"', 'active === "recommend"', 'active === "ai"', 'active === "developers"', 'active === "tags"', 'active === "years" || active === "ratings"']) {
      expect(page).toContain(branch);
    }
  });

  it("routes section loading/empty/error states through StateBoundary instead of hand-rolled banners", () => {
    const page = source("src/lib/components/DiscoveryPage.svelte");
    expect(page).toContain("searchViewState");
    expect(page).toContain("recommendViewState");
    expect(page).toContain("devsViewState");
    expect(page).toContain("tagsViewState");
    expect(page).toContain("yearsViewState");
    expect(page).toContain("ratingsViewState");
    expect(page).toContain("onRetry={loadRecommendations}");
    expect(page).toContain("onRetry={search}");
    expect(page).not.toContain("error-banner");
  });

  it("wires the i18n store into the page and ships discovery.* keys in both dictionaries", () => {
    const page = source("src/lib/components/DiscoveryPage.svelte");
    const i18nStore = source("src/lib/stores/i18n.svelte.ts");
    expect(page).toContain('from "../stores/i18n.svelte"');
    expect(page).toContain('i18n.t("discovery.title")');
    for (const key of [
      '"discovery.title"',
      '"discovery.tab_search"',
      '"discovery.tab_ai"',
      '"discovery.search_idle_title"',
      '"discovery.rec_empty_title"',
      '"discovery.facet_empty_title"',
    ]) {
      expect(i18nStore).toContain(key);
    }
    // zh + en dictionaries both carry the keys.
    expect(i18nStore).toContain('"discovery.title": "资源发现"');
    expect(i18nStore).toContain('"discovery.title": "Discovery"');
  });

  it("preserves the AI recommendation guard pipeline and the detail/workbench mount points", () => {
    const page = source("src/lib/components/DiscoveryPage.svelte");
    expect(page).toContain("GenerationGuard");
    expect(page).toContain("validateRecommendationExplanations");
    expect(page).toContain("isAiUnavailableError");
    expect(page).toContain("isAbortError");
    expect(page).toContain("<DiscoveryDetail");
    expect(page).toContain("<AiExperienceWorkbench");
    expect(page).toContain("<RecommendationExplanation");
    expect(page).toContain("appliedFilterDsl");
    expect(page).toContain("settingsStore.settings.getchu_enabled");
  });

  it("keeps reduced-motion fallbacks for page-level scrolling", () => {
    const page = source("src/lib/components/DiscoveryPage.svelte");
    expect(page).toContain("prefers-reduced-motion: reduce");
    expect(page).toContain('data-motion="reduce"');
  });
});
