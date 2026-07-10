import { expect, test, DEFAULT_APP_STATE } from "./fixtures/app-fixture";

const event = {
  id: "event-1",
  resourceKind: "game",
  resourceId: "fixture-game-1",
  eventType: "progressed",
  startedAt: "2026-07-10T10:00:00.000Z",
  endedAt: "2026-07-10T11:00:00.000Z",
  durationSeconds: 3600,
  providerId: null,
  payload: {},
  durationQuality: "exact",
  sourceLegacyId: "legacy-session-1",
};

const games = DEFAULT_APP_STATE.games.map((game, index) => index === 0 ? {
  ...game,
  play_tracker: {
    ...game.play_tracker,
    total_seconds: 7200,
    last_played: "2026-07-10T10:00:00.000Z",
    completion_status: "playing",
    sessions: [{ id: "session-1", start_time: "2026-07-10T10:00:00.000Z", end_time: "2026-07-10T11:00:00.000Z", duration_seconds: 3600, notes: "" }],
  },
} : game);

test.use({
  appState: {
    ...DEFAULT_APP_STATE,
    games,
    commandResults: {
      backfill_legacy_game_activity: { created: 1 },
      get_playtime_summary: {
        total_seconds: 3600,
        session_count: 1,
        play_days: 1,
        average_session_seconds: 3600,
        daily: [{ date: "2026-07-10", seconds: 3600, sessions: 1 }],
        monthly: [{ month: "2026-07", seconds: 3600, sessions: 1 }],
        recent_sessions: [{ game_id: "fixture-game-1", game_name: "星海回声", session: { id: "session-1", start_time: "2026-07-10T10:00:00.000Z", end_time: "2026-07-10T11:00:00.000Z", duration_seconds: 3600, notes: "" } }],
        top_games: [{ game_id: "fixture-game-1", game_name: "星海回声", total_seconds: 3600, sessions: 1, last_played: "2026-07-10T10:00:00.000Z" }],
      },
      get_activity_events: { events: [event], nextCursor: null },
      get_activity_summary: { eventCount: 1, exactDurationSeconds: 3600, estimatedDurationSeconds: 0, progressOnlyCount: 0, days: [] },
      get_continue_candidates: [{ resourceKind: "game", resourceId: "fixture-game-1", providerId: null, title: "星海回声", artworkUrl: null, position: {}, updatedAt: "2026-07-10T11:00:00.000Z", completed: false, durationQuality: "exact", exactDurationSeconds: 3600, estimatedDurationSeconds: null }],
      edit_activity_event: event,
    },
  },
});

test.describe("records and Continue UI-v2 keyboard focus", () => {
  test("activity editor traps Escape and restores the edit trigger", async ({ appPage: page }) => {
    await page.getByRole("button", { name: "打开游玩记录" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "游玩记录" })).toBeVisible();
    const edit = page.getByRole("button", { name: "编辑 fixture-game-1 活动" });
    await edit.focus();
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "编辑活动记录" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel("开始时间")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(edit).toBeFocused();
  });

  test("Continue rows are keyboard activatable MediaRows", async ({ appPage: page }) => {
    await page.getByRole("button", { name: "打开工具抽屉" }).click();
    await page.getByRole("dialog", { name: "工具" }).getByRole("button", { name: "打开继续游玩" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "今日继续" })).toBeVisible();
    const row = page.getByRole("button", { name: "继续 星海回声" }).first();
    await row.focus();
    await expect(row).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/#game-detail(?:\?id=fixture-game-1)?$/);
  });
});



