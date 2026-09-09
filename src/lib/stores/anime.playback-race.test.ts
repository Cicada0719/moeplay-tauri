import { beforeEach, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (v: string) => v, invoke: vi.fn() }));
beforeEach(() => { localStorage.clear(); vi.resetModules(); });

for (const stage of ["anime_build_url", "get_video_proxy_port", "anime_get_proxy_url"]) {
  for (const action of ["close", "switch"] as const) {
    it(`ignores old ${stage} response after ${action}`, async () => {
      let release!: (value: unknown) => void;
      const pending = new Promise(resolve => { release = resolve; });
      const calls: string[] = [];
      const core = await import("../api/core");
      core.setMockInvokeHandler(command => {
        calls.push(command);
        if (command === stage) return pending;
        if (command === "anime_build_url") return "https://source.test/episode";
        if (command === "anime_extract_video_url") return { url: "https://cdn.test/old.mp4" };
        if (command === "get_video_proxy_port") return 43123;
        if (command === "anime_get_proxy_url") return "http://localhost:43123/old.mp4";
        return null;
      });
      const { animeStore: store } = await import("./anime.svelte");
      store.setRoadsForPlayback([{ name: "one", episodes: [{ name: "1", url: "/episode" }] }], "test", "/show");
      const playback = store.playEpisode(0, 0);
      await vi.waitFor(() => expect(calls).toContain(stage));
      if (action === "close") store.closePlayer();
      else store.playDirectVideoSource("https://cdn.test/new.mp4", {}, "video", 0);
      release(stage === "get_video_proxy_port" ? 43123 : "https://cdn.test/stale.mp4");
      await playback;
      expect(store.view).toBe(action === "close" ? "detail" : "player");
      expect(store.playerVideoSrc).toBe(action === "close" ? "" : "https://cdn.test/new.mp4");
      expect(store.playerUrl).not.toBe("https://cdn.test/stale.mp4");
    });
  }
}
