import { expect, test } from "./fixtures";
import { DEFAULT_APP_STATE } from "./fixtures/mock-app-state";

const animeHistory = {
  key: "fixture:夏日动画",
  name: "夏日动画",
  image: "https://example.test/anime.jpg",
  ruleName: "本地源",
  sourceUrl: "https://example.test/anime",
  lastRoad: 0,
  lastEpisode: 2,
  lastEpisodeName: "第 3 集",
  progressMs: 12_000,
  updatedAt: "2026-07-10T11:00:00.000Z",
};

const comicHistory = {
  id: "fixture-comic",
  title: "海边漫画",
  thumb_url: "https://example.test/comic.jpg",
  author: "作者",
  last_order: 8,
  last_title: "第 8 话",
  ts: new Date("2026-07-10T10:00:00.000Z").getTime(),
};

const novelHistory = {
  key: "biquge:fixture-novel:chapter-2",
  book: {
    id: "fixture-novel",
    source: "biquge",
    title: "山间小说",
    subjects: [],
    publicDomain: false,
    sourceUrl: "https://example.test/novel",
    coverUrl: "https://example.test/novel.jpg",
  },
  chapterId: "chapter-2",
  chapterTitle: "第二章",
  progress: 0.42,
  updatedAt: new Date("2026-07-10T12:00:00.000Z").getTime(),
};

test.use({
  appState: {
    ...DEFAULT_APP_STATE,
    games: [],
    localStorage: {
      ...DEFAULT_APP_STATE.localStorage,
      "anime-history": JSON.stringify([animeHistory]),
      "picacg-history": JSON.stringify([comicHistory]),
      "moeplay-novel-history-v1": JSON.stringify([novelHistory]),
    },
  },
});

test("unified media history exposes all media kinds and opens the matching reader", async ({ page }) => {
  await page.goto("/?skip_wizard#continue");

  const history = page.getByTestId("unified-media-history");
  await expect(history).toBeVisible({ timeout: 15_000 });
  await expect(history.getByText("夏日动画", { exact: true })).toBeVisible();
  await expect(history.getByText("海边漫画", { exact: true })).toBeVisible();
  await expect(history.getByText("山间小说", { exact: true })).toBeVisible();
  await expect(history.getByText("第二章 · 42%", { exact: true })).toBeVisible();

  await history.getByRole("button", { name: "继续阅读 山间小说" }).click();
  await expect(page).toHaveURL(/#novel$/);
});

test("text backup restores a locally deleted book and search filters the result", async ({ page }) => {
  await page.goto("/?skip_wizard#continue");
  const history = page.getByTestId("unified-media-history");
  await expect(history.getByText("山间小说", { exact: true })).toBeVisible();
  await history.getByText("移动端文本备份与恢复", { exact: true }).click();
  await history.getByRole("button", { name: "生成备份文本" }).click();
  const backup = history.getByRole("textbox", { name: "阅读历史 JSON 备份文本" });
  await expect(backup).toHaveValue(/moeplay-reading-history/);
  const exported = JSON.parse(await backup.inputValue());
  expect(exported.positions).toHaveLength(2);
  await history.getByRole("textbox", { name: "搜索阅读历史" }).fill("山间");
  await expect(history.getByRole("listitem")).toHaveCount(1);
  await history.getByRole("button", { name: "管理记录" }).click();
  await history.getByRole("button", { name: /删除/ }).click();
  await expect(history.getByRole("listitem")).toHaveCount(0);
  await history.getByRole("button", { name: "导入文本", exact: true }).click();
  await expect(history.getByRole("listitem")).toHaveCount(1);
  await expect(history.getByText("第二章 · 42%", { exact: true })).toBeVisible();
});
