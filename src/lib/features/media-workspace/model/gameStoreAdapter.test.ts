import { describe, expect, it, vi } from "vitest";
import type { Game } from "../../../api";
import { createGameStorePresentationAdapter, type GameStoreLike } from "./gameStoreAdapter";

function game(id: string): Game {
  return {
    id,
    name: `Game ${id}`,
    exe_path: `C:/Games/${id}.exe`,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    screenshots: [],
    favorite: false,
    hidden: false,
    tags: [],
    metadata: { genres: [], languages: [], voice_languages: [], stores: [] },
    play_tracker: {
      total_seconds: 0,
      sessions: [],
      completion_status: "not_started",
      achievements_total: 0,
      achievements_unlocked: 0,
      finished: false,
      completion_count: 0,
    },
    save_data: {
      auto_backup: false,
      backup_interval_minutes: 30,
      max_backups: 10,
      backups: [],
      cloud_sync: false,
    },
    aliases: [],
    tag_entries: [],
    play_time_seconds: 0,
  };
}

describe("createGameStorePresentationAdapter", () => {
  it("reflects filtered/all/selected production store views and delegates actions", async () => {
    const first = game("one");
    const second = game("two");
    const store: GameStoreLike = {
      games: [first],
      allGames: [first, second],
      selectedId: "two",
      loading: false,
      loadError: null,
      load: vi.fn(async () => undefined),
      importGame: vi.fn(async () => null),
      selectGame: vi.fn(),
      launch: vi.fn(),
      toggleFavorite: vi.fn(),
    };
    const adapter = createGameStorePresentationAdapter(store);

    expect(adapter.items.map(item => item.id)).toEqual(["one"]);
    expect(adapter.allItems.map(item => item.id)).toEqual(["one", "two"]);
    expect(adapter.selectedItem?.id).toBe("two");

    await adapter.items[0].actions.find(action => action.id === "launch")?.run();
    await adapter.items[0].actions.find(action => action.id === "toggle-favorite")?.run();
    adapter.items[0].actions.find(action => action.id === "select")?.run();
    await adapter.refresh();
    await adapter.importGame();

    expect(store.launch).toHaveBeenCalledWith("one");
    expect(store.toggleFavorite).toHaveBeenCalledWith("one");
    expect(store.selectGame).toHaveBeenCalledWith("one");
    expect(store.load).toHaveBeenCalledOnce();
    expect(store.importGame).toHaveBeenCalledOnce();
  });
});
