import { test, expect, DEFAULT_APP_STATE } from "./fixtures";

const book = { id: "reading-book", source: "gutenberg", title: "星海航行日志", subjects: [], publicDomain: true, sourceUrl: "" };
const chapters = Array.from({ length: 80 }, (_, i) => ({ id: String(i + 1), order: i + 1, title: `第 ${i + 1} 章` }));
const image = (index: number) => `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="${index === 4 ? 1600 : 600}" height="900"><rect width="100%" height="100%" fill="#152c31"/><text x="100" y="250" fill="#63e4d4" font-size="100">PAGE ${index + 1}</text></svg>`)}`;
test.use({ appState: { ...DEFAULT_APP_STATE,
  settings: { ...DEFAULT_APP_STATE.settings, appearance: { theme_pack: "borderless-lumen", color_mode: "dark", wallpaper_rotation: "fixed", mascot_enabled: false, decorative_effects: false, online_gallery_enabled: false } },
  commandResults: { ...DEFAULT_APP_STATE.commandResults,
    comic_detail: { id: "reading-comic", title: "星海漫画", author: "MoePlay", thumb_url: image(0), categories: [], tags: [], eps_count: 12 },
    comic_chapters: [{ id: "chapter12", order: 12, title: "第 12 话" }, { id: "chapter13", order: 13, title: "第 13 话" }],
    comic_chapter_images: Array.from({ length: 14 }, (_, i) => ({ id: `page${i + 1}`, url: image(i) })),
    novel_detail: { book, chapters },
    novel_read_chapter: { bookId: book.id, source: book.source, chapter: chapters[29], content: "星海旅程，翻开下一页。探索与阅读，让每一次出发都有迹可循。\n\n".repeat(500) },
  },
} });

async function openComic(page: import("@playwright/test").Page) {
  await page.goto("/?skip_wizard&platform=windows#comic");
  await expect(page.getByTestId("comic-page")).toBeVisible();
  await page.evaluate(async () => {
    const { comicStore } = await import("/src/lib/stores/comic.svelte.ts");
    await comicStore.openComic("reading-comic");
    await comicStore.openChapter(12, "第 12 话");
  });
  await expect(page.locator(".reader-overlay .comic-img").first()).toBeVisible();
}

test("comic chapter 12 page 7 survives reload, display switches and background", async ({ page }) => {
  await openComic(page);
  await page.locator(".reader-overlay").press("d");
  for (let i = 0; i < 6; i++) await page.locator(".reader-overlay").press("PageDown");
  await expect(page.locator(".reader-progress-text")).toContainText("7 / 14");
  await page.locator(".reader-overlay").press("s");
  await expect(page.locator(".reader-progress-text")).toContainText("7 / 14");
  await page.locator(".reader-overlay").press("s");
  await page.evaluate(() => window.dispatchEvent(new Event("pagehide")));
  await expect.poll(() => page.evaluate(async () => {
    const { readingRepository } = await import("/src/lib/features/reading-history/repository.ts");
    return JSON.parse(await readingRepository.exportJSON()).positions.find((p: any) => p.contentId === "reading-comic")?.pageIndex;
  })).toBe(6);
  await openComic(page);
  await expect(page.locator(".reader-progress-text")).toContainText("7 / 14");
  await page.setViewportSize({ width: 960, height: 640 });
  await expect(page.locator(".reader-progress-text")).toContainText("7 / 14");
});

test("novel restores chapter 30 at 45 percent after content layout", async ({ page }, testInfo) => {
  await page.goto("/?skip_wizard&platform=windows#novel");
  await expect(page.getByTestId("novel-page")).toBeVisible();
  await page.evaluate(async ({ book, chapter }) => {
    const { readingRepository } = await import("/src/lib/features/reading-history/repository.ts");
    await readingRepository.save({ kind: "novel", source: book.source, contentId: book.id, title: book.title,
      chapterId: chapter.id, chapterTitle: chapter.title, progress: .45, updatedAt: Date.now(), metadata: { book } });
    const { novelStore } = await import("/src/lib/features/novel/store.svelte.ts");
    await novelStore.openBook(book); await novelStore.readChapter(chapter);
  }, { book, chapter: chapters[29] });
  const reader = page.getByRole("region", { name: "小说正文" });
  await expect(reader).toBeVisible();
  await expect.poll(() => reader.evaluate(el => el.scrollTop / (el.scrollHeight - el.clientHeight))).toBeGreaterThan(.40);
  expect(await reader.evaluate(el => el.scrollTop / (el.scrollHeight - el.clientHeight))).toBeLessThan(.50);
  await page.screenshot({ path: testInfo.outputPath("reading.png") });
});

test("wide comic page occupies its own spread and chapter positions stay separate", async ({ page }) => {
  await openComic(page);
  const reader = page.locator(".reader-overlay");
  await reader.press("d");
  for (let i = 0; i < 4; i++) await reader.press("PageDown");
  await expect(reader.locator('.comic-img:visible')).toHaveCount(1);
  await expect.poll(() => reader.locator('.comic-img:visible').evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(1600);
  await reader.press("s");
  await expect(reader.locator('.comic-img:visible')).toHaveCount(1);
  await expect(page.locator(".reader-progress-text")).toContainText("5 / 14");
  await reader.press("]");
  await expect(page.locator(".reader-progress-text")).toContainText("1 / 14");
  await reader.press("[");
  await expect(page.locator(".reader-progress-text")).toContainText("5 / 14");
});
