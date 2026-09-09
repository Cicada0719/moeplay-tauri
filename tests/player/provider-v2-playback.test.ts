import { afterEach, expect, it, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import { tick } from "svelte";
import Player from "../../src/lib/components/anime/provider-v2/ProviderV2Player.svelte";

vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (path: string) => `https://media.test/${path}` }));
const identity = { providerId: "local", seriesId: "series", episodeId: "1" };
const episode = { identity, title: "1", number: 1, artworkUrl: null };
afterEach(() => vi.restoreAllMocks());

it("reacts to source replacement and releases failed/unmounted native media", async () => {
  const pause = vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => {});
  vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue();
  vi.spyOn(HTMLMediaElement.prototype, "load").mockImplementation(() => {});
  const { container, rerender, unmount } = render(Player, {
    episode, seriesTitle: "test", onClose: () => {}, onFallback: () => {},
    resolution: { episode: identity, target: { mode: "native_file", path: "first.mp4" } },
  });
  await tick();
  const video = container.querySelector("video")!;
  expect(video.src).toBe("https://media.test/first.mp4");
  await rerender({ resolution: { episode: identity, target: { mode: "native_file", path: "second.mp4" } } });
  await tick();
  expect(video.src).toBe("https://media.test/second.mp4");
  await fireEvent.error(video);
  expect(pause).toHaveBeenCalled();
  expect(video.getAttribute("src")).toBeNull();
  expect(container.querySelector('[role="alert"]')?.textContent).toContain("重试");
  await unmount();
  expect(video.getAttribute("src")).toBeNull();
});
