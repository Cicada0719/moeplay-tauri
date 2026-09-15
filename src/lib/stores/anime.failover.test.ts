import { beforeEach, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (value: string) => value, invoke: vi.fn() }));

const rule = (name: string) => ({
  name, version: "1", baseUrl: `https://${name}.test`, searchURL: "", searchList: "", searchName: "",
  searchResult: "", chapterRoads: "", chapterResult: "", muliSources: false, useWebview: true,
  useNativePlayer: true, usePost: false, useLegacyParser: false, adBlocker: false, userAgent: "",
  referer: "", api: "", type: "anime",
});

beforeEach(() => {
  localStorage.clear();
  vi.resetModules();
});

it("starts extracting a fast candidate before a slower source finishes discovery", async () => {
  localStorage.setItem("anime-rules", JSON.stringify([rule("slow"), rule("fast")]));
  const core = await import("../api/core");
  const extracted: string[] = [];
  core.setMockInvokeHandler((command, args) => {
    const input = args as Record<string, unknown> | undefined;
    if (command === "anime_build_url") return input?.url;
    if (command === "get_video_proxy_port") return 43123;
    if (command === "anime_get_proxy_url") return "http://localhost:43123/fast.mp4";
    if (command === "anime_record_source_health" || command === "frontend_log") return null;
    if (command === "anime_search") {
      const source = String(input?.ruleName);
      if (source === "slow") return new Promise(resolve => setTimeout(() => resolve([{ name: "Show", url: "slow-show" }]), 180));
      return [{ name: "Show", url: "fast-show" }];
    }
    if (command === "anime_fetch_roads") {
      const pageUrl = String(input?.pageUrl);
      return [{ name: "main", episodes: [{ name: "第1集", url: `${pageUrl}/ep1` }] }];
    }
    if (command === "anime_extract_video_url") {
      const pageUrl = String(input?.episodeUrl);
      extracted.push(pageUrl);
      if (pageUrl === "initial") return Promise.reject(new Error("解析为空"));
      return { url: "https://cdn.test/fast.mp4", tab_url: pageUrl };
    }
    return null;
  });

  const { animeStore } = await import("./anime.svelte");
  animeStore.setRoadsForPlayback([{ name: "main", episodes: [{ name: "第1集", url: "initial" }] }], "initial", "show");
  await animeStore.playEpisode(0, 0);
  await vi.waitFor(() => expect(animeStore.playerVideoSrc).toBe("http://localhost:43123/fast.mp4"), { timeout: 2_000 });

  expect(extracted.some(url => url.includes("fast-show/ep1"))).toBe(true);
  expect(extracted.some(url => url.includes("slow-show/ep1"))).toBe(false);
});
