import { readFileSync } from "node:fs";
import { test, expect } from "./fixtures";

test.beforeEach(async ({ page }) => {
  await page.route("**/media-fixture/*", async route => {
    const name = new URL(route.request().url()).pathname.split("/").pop()!;
    const contentType = name.endsWith("m3u8") ? "application/vnd.apple.mpegurl" : name.endsWith("ts") ? "video/mp2t" : name.endsWith("m4a") ? "audio/mp4" : "video/mp4";
    await route.fulfill({ contentType, body: readFileSync(`tests/visual/fixtures/media/${name}`) });
  });
  await page.goto("/?skip_wizard&platform=windows#anime");
  await expect(page.getByTestId("anime-page")).toBeVisible();
});

test("HLS decodes actual segments and a replacement source starts fresh", async ({ page }) => {
  const video = await play(page, "stream.m3u8");
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.getVideoPlaybackQuality().totalVideoFrames)).toBeGreaterThan(5);
  await play(page, "picture-and-sound.mp4");
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.currentSrc)).toContain("picture-and-sound.mp4");
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.videoWidth)).toBe(160);
});

test("sniffer injection forces hidden webpage media to remain muted", async ({ page }) => {
  const source = readFileSync("src-tauri/src/video_extractor.rs", "utf8");
  const injection = source.split('fn sniff_js() -> String {')[1].split('r##"')[1].split('"##')[0];
  // Intercept sentinel navigation without reaching a third-party source.
  await page.route("**/moe-video-found/**", route => route.abort());
  const muted = await page.evaluate(async script => {
    const nativeMuted = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "muted")!;
    (0, eval)(script);
    const video = document.createElement("video");
    document.body.append(video);
    video.muted = false;
    const initial = nativeMuted.get!.call(video);
    video.dispatchEvent(new Event("play", { bubbles: true }));
    const afterPlay = nativeMuted.get!.call(video);
    video.remove();
    return { initial, afterPlay };
  }, injection);
  expect(muted).toEqual({ initial: true, afterPlay: true });
});

test("lost enhancement context exposes the original playing video", async ({ page }) => {
  await play(page, "picture-and-sound.mp4");
  const result = await page.evaluate(async () => {
    const { LocalVideoEnhancer } = await import("/src/lib/features/anime-player/localVideoEnhancement.ts");
    const video = document.querySelector("video.player-video") as HTMLVideoElement;
    const canvas = document.createElement("canvas");
    document.body.append(canvas);
    const statuses: string[] = [];
    const enhancer = new LocalVideoEnhancer(canvas, video, "balanced", status => statuses.push(status));
    enhancer.start();
    canvas.dispatchEvent(new Event("webglcontextlost", { cancelable: true }));
    enhancer.destroy();
    canvas.remove();
    return { statuses, paused: video.paused, width: video.videoWidth };
  });
  expect(result.statuses).toContain("ready");
  expect(result.statuses.at(-1)).toBe("error");
  expect(result.paused).toBe(false);
  expect(result.width).toBe(160);
});

async function play(page: import("@playwright/test").Page, name: string) {
  await page.evaluate(async file => {
    const { animeStore } = await import("/src/lib/stores/anime.svelte.ts");
    animeStore.autoWebFallback = false;
    animeStore.playDirectVideoSource(`${location.origin}/media-fixture/${file}`, {}, "video", 0);
  }, name);
  const video = page.locator("video.player-video");
  await expect(video).toBeVisible();
  // Explicit user gesture also covers browser policies that block autoplay.
  await video.evaluate(async (v: HTMLVideoElement) => { v.muted = true; await v.play(); });
  return video;
}

test("real decoded picture advances and exiting releases audio and source", async ({ page }) => {
  const video = await play(page, "picture-and-sound.mp4");
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.videoWidth)).toBe(160);
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.getVideoPlaybackQuality().totalVideoFrames)).toBeGreaterThan(5);
  const oldVideo = await video.elementHandle();
  await page.evaluate(async () => {
    const { animeStore } = await import("/src/lib/stores/anime.svelte.ts");
    animeStore.closePlayer();
  });
  await expect(video).toHaveCount(0);
  expect(await oldVideo!.evaluate((v: HTMLVideoElement) => ({ paused: v.paused, src: v.getAttribute("src") }))).toEqual({ paused: true, src: null });
});

test("audio clock without decoded picture cannot remain in loading state", async ({ page }) => {
  const video = await play(page, "audio-only.m4a");
  await expect.poll(() => video.evaluate((v: HTMLVideoElement) => v.currentTime)).toBeGreaterThan(0.1);
  expect(await video.evaluate((v: HTMLVideoElement) => v.videoWidth)).toBe(0);
  await expect.poll(() => page.evaluate(async () => {
    const { animeStore } = await import("/src/lib/stores/anime.svelte.ts");
    return animeStore.playerExtractStatus;
  }), { timeout: 20_000 }).toBe("error");
  await expect.poll(() => page.locator("video").evaluateAll(videos => videos.every(v => v.paused))).toBe(true);
});
