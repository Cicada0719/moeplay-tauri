import { describe, expect, it, vi } from "vitest";
import type { Game } from "../../../api";
import { adaptGameToPresentation, adaptGamesToPresentation } from "./gamePresentation";

function game(overrides: Partial<Game> = {}): Game {
  return {
    id: "game-1",
    name: "MoePlay Test",
    exe_path: "C:/Games/Test/game.exe",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    screenshots: ["shot-1.webp", "shot-2.webp"],
    favorite: false,
    hidden: false,
    tags: ["剧情"],
    metadata: {
      genres: ["视觉小说"],
      languages: [],
      voice_languages: [],
      stores: [],
      developer: "Moe Studio",
      publisher: "Moe Publisher",
      platform: "pc",
      release_year: 2026,
      cover: "cover.webp",
      background: "hero.webp",
    },
    play_tracker: {
      total_seconds: 7200,
      sessions: [],
      completion_status: "playing",
      achievements_total: 0,
      achievements_unlocked: 0,
      finished: false,
      completion_count: 0,
      last_played: "2026-07-10T12:00:00Z",
      user_rating: 8,
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
    play_time_seconds: 7200,
    ...overrides,
  };
}

describe("adaptGameToPresentation", () => {
  it("normalizes game media and metadata for all workspace modes", () => {
    const item = adaptGameToPresentation(game());

    expect(item).toMatchObject({
      id: "game-1",
      module: "games",
      title: "MoePlay Test",
      subtitle: "Moe Studio · 2026",
      mediaQuality: "a",
      favorite: false,
      installed: true,
    });
    expect(item.cover?.src).toBe("cover.webp");
    expect(item.hero?.src).toBe("hero.webp");
    expect(item.screenshots.map(asset => asset.src)).toEqual(["shot-1.webp", "shot-2.webp"]);
    expect(item.metadata.tags).toEqual(["视觉小说", "剧情"]);
    expect(item.metadata.totalSeconds).toBe(7200);
  });

  it("grades cover-only and missing artwork without inventing assets", () => {
    const coverOnly = game({
      screenshots: [],
      background: undefined,
      cover: "cover-only.webp",
      icon: undefined,
      metadata: { genres: [], languages: [], voice_languages: [], stores: [], cover: "cover-only.webp" },
    });
    const noMedia = game({
      screenshots: [],
      cover: undefined,
      background: undefined,
      icon: undefined,
      metadata: { genres: [], languages: [], voice_languages: [], stores: [] },
    });

    expect(adaptGameToPresentation(coverOnly).mediaQuality).toBe("c");
    expect(adaptGameToPresentation(noMedia).mediaQuality).toBe("d");
    expect(adaptGameToPresentation(noMedia).media).toEqual([]);
  });

  it("exposes real store actions and disables launch when no target exists", async () => {
    const launch = vi.fn();
    const toggleFavorite = vi.fn();
    const item = adaptGameToPresentation(game({ favorite: true }), { launch, toggleFavorite });

    await item.actions.find(action => action.id === "launch")?.run();
    await item.actions.find(action => action.id === "toggle-favorite")?.run();

    expect(launch).toHaveBeenCalledWith("game-1");
    expect(toggleFavorite).toHaveBeenCalledWith("game-1");
    expect(item.actions.find(action => action.id === "toggle-favorite")).toMatchObject({ active: true });

    const unavailable = adaptGameToPresentation(game({ exe_path: "", launch_uri: undefined }), { launch });
    expect(unavailable.actions.find(action => action.id === "launch")?.enabled).toBe(false);
  });

  it("adapts lists deterministically", () => {
    expect(adaptGamesToPresentation([game(), game({ id: "game-2", name: "Second" })]).map(item => item.id))
      .toEqual(["game-1", "game-2"]);
  });
});
