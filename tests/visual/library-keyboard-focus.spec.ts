import { test, expect, DEFAULT_APP_STATE, MOCK_GAMES } from "./fixtures";

const manyGames = Array.from({ length: 120 }, (_, index) => {
  const base = MOCK_GAMES[index % MOCK_GAMES.length];
  const number = String(index + 1).padStart(3, "0");
  return {
    ...base,
    id: `fixture-game-${number}`,
    name: `游戏 ${number} · ${base.name}`,
    favorite: index % 9 === 0,
    created_at: `2026-06-${String((index % 28) + 1).padStart(2, "0")}T08:00:00.000Z`,
    updated_at: `2026-07-${String((index % 10) + 1).padStart(2, "0")}T10:00:00.000Z`,
    play_tracker: {
      ...base.play_tracker,
      last_played: index < 16 ? `2026-07-${String(10 - (index % 10)).padStart(2, "0")}T10:00:00.000Z` : null,
    },
  };
});

test.use({
  appState: {
    ...DEFAULT_APP_STATE,
    games: manyGames,
    commandResults: {
      detect_save_candidates: [],
      list_save_snapshots: [],
      list_game_notes: [],
      get_game_notes: [],
    },
  },
});

test.describe("P1-02 game library keyboard and focus", () => {
  test("detail closes to the originating card and restores virtual scroll", async ({ appPage: page }) => {
    await page.getByRole("button", { name: "查看全部游戏" }).click();
    await expect(page.getByTestId("all-games-panel")).toBeVisible();

    const scroll = page.getByTestId("game-library-scroll");
    await scroll.evaluate((node) => {
      node.scrollTop = 1600;
      node.dispatchEvent(new Event("scroll"));
    });
    await expect.poll(() => scroll.evaluate((node) => node.scrollTop)).toBeGreaterThan(1000);

    const focusKey = await scroll.evaluate((node) => {
      const viewport = node.getBoundingClientRect();
      const candidates = Array.from(node.querySelectorAll<HTMLElement>("[data-focus-key^='game-card-']"));
      return candidates.find((candidate) => {
        const rect = candidate.getBoundingClientRect();
        return rect.top >= viewport.top + 8 && rect.bottom <= viewport.bottom - 8;
      })?.dataset.focusKey ?? null;
    });
    expect(focusKey).toBeTruthy();
    const card = page.locator(`[data-focus-key="${focusKey}"]`);
    await expect(card).toBeVisible();
    const originalScroll = await scroll.evaluate((node) => node.scrollTop);
    const cardName = await card.getAttribute("aria-label");
    expect(cardName).toContain("详情");

    await card.focus();
    await expect(card).toBeFocused();
    await page.keyboard.press("Enter");

    await expect(page).toHaveURL(/#game-detail\?id=/);
    await expect(page.getByTestId("game-detail-page")).toBeVisible();
    await expect(page.locator(".game-detail-primary")).toBeFocused();

    await page.keyboard.press("Escape");
    await expect(page).toHaveURL(/#home$/);
    await expect(page.getByTestId("all-games-panel")).toBeVisible();

    const restored = page.locator(`[data-focus-key="${focusKey}"]`);
    await expect(restored).toBeFocused();
    await expect.poll(() => page.getByTestId("game-library-scroll").evaluate((node) => node.scrollTop))
      .toBeGreaterThan(originalScroll - 80);
  });

  test("delete confirmation defaults to Cancel and returns focus safely", async ({ appPage: page }) => {
    await page.getByRole("button", { name: "查看全部游戏" }).click();
    await expect(page.getByTestId("all-games-panel")).toBeVisible();

    const deleteButton = page.getByRole("button", { name: /删除 游戏 001/ });
    await deleteButton.focus();
    await deleteButton.click();

    await expect(page.getByRole("alertdialog", { name: "从游戏库删除？" })).toBeVisible();
    await expect(page.getByRole("alertdialog", { name: "从游戏库删除？" }).getByRole("button", { name: "取消", exact: true })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("alertdialog", { name: "从游戏库删除？" })).toBeHidden();
    await expect(deleteButton).toBeFocused();
  });

  test("slash scopes search to the library and clearing keeps input focus", async ({ appPage: page }) => {
    await page.keyboard.press("/");
    const search = page.getByRole("searchbox", { name: "搜索游戏库" });
    await expect(search).toBeFocused();
    await search.fill("星海");
    await expect(page.getByTestId("all-games-panel")).toBeVisible();

    await page.getByRole("button", { name: "清空搜索" }).click();
    await expect(search).toBeFocused();
    await expect(search).toHaveValue("");
  });
});

