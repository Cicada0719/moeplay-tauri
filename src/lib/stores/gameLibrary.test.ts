import { describe, it, expect, vi, beforeEach } from "vitest";
import { libraryStore, matchesPinyin, type Game } from "./gameLibrary.svelte";

vi.mock("../api", () => ({
  getGames: vi.fn(),
  searchGames: vi.fn(),
  addGameByDialog: vi.fn(),
  applyScrapeResult: vi.fn(),
  deleteGame: vi.fn(),
  launchGame: vi.fn(),
  setSaveDir: vi.fn(),
  toggleFavorite: vi.fn(),
  toggleHidden: vi.fn(),
  addSimpleTag: vi.fn(),
  removeSimpleTag: vi.fn(),
  updateCompletionStatus: vi.fn(),
}));

const { getGames } = await import("../api");

function makeGame(overrides: Partial<Game> = {}): Game {
  return {
    id: crypto.randomUUID(),
    name: "Test Game",
    favorite: false,
    install_dir: null,
    exe_path: null,
    description: "",
    cover: null,
    icon: null,
    background: null,
    metadata: {},
    play_tracker: { total_playtime_seconds: 0, sessions: [] },
    save_data: {},
    aliases: [],
    simple_tags: [],
    tag_entries: [],
    created_at: new Date().toISOString(),
    add_date: new Date().toISOString(),
    ...overrides,
  } as Game;
}

async function setGames(games: Game[]) {
  vi.mocked(getGames).mockResolvedValueOnce(games);
  await libraryStore.load();
}

function resetStore() {
  libraryStore.searchQuery = "";
  libraryStore.filterTag = null;
  libraryStore.quickFilter = null;
  libraryStore.sortBy = "recent";
  libraryStore.activateCollection(null);
}

describe("libraryStore", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  it("loads and normalizes games", async () => {
    const g1 = makeGame({ name: "Alpha" });
    await setGames([g1]);
    expect(libraryStore.allGames.length).toBe(1);
    expect(libraryStore.games.length).toBe(1);
    expect(libraryStore.loading).toBe(false);
  });

  it("filters by quick filter favorite", async () => {
    await setGames([makeGame({ favorite: false }), makeGame({ favorite: true })]);
    libraryStore.quickFilter = "favorite";
    expect(libraryStore.games.every(g => g.favorite)).toBe(true);
  });

  it("filters by tag", async () => {
    await setGames([
      makeGame({ tags: ["RPG"], metadata: { genres: ["JRPG"], languages: [], voice_languages: [], stores: [] } as any }),
      makeGame({ tags: ["ACT"], metadata: { genres: ["Action"], languages: [], voice_languages: [], stores: [] } as any }),
    ]);
    libraryStore.filterTag = "RPG";
    expect(libraryStore.games.length).toBe(1);
    expect(libraryStore.games[0].tags).toContain("RPG");
  });

  it("searches by name", async () => {
    await setGames([makeGame({ name: "命运石之门" }), makeGame({ name: "Ever17" })]);
    libraryStore.searchQuery = "ever";
    expect(libraryStore.games.length).toBe(1);
    expect(libraryStore.games[0].name).toBe("Ever17");
  });

  it("sorts by name", async () => {
    await setGames([makeGame({ name: "Beta" }), makeGame({ name: "Alpha" })]);
    libraryStore.sortBy = "name";
    expect(libraryStore.games[0].name).toBe("Alpha");
  });

  it("manages smart collections", () => {
    const col = libraryStore.saveCurrentAsCollection("Favorites", "heart");
    expect(libraryStore.smartCollections).toContainEqual(expect.objectContaining({ name: "Favorites" }));
    libraryStore.activateCollection(col.id);
    expect(libraryStore.activeCollectionId).toBe(col.id);
    libraryStore.removeCollection(col.id);
    expect(libraryStore.smartCollections.find(c => c.id === col.id)).toBeUndefined();
  });
});

describe("matchesPinyin", () => {
  it("matches direct substring", () => {
    expect(matchesPinyin("命运石之门", "命运")).toBe(true);
  });
  it("matches pinyin letters", () => {
    // pinyin-pro default separator is space
    expect(matchesPinyin("命运石之门", "ming yun")).toBe(true);
    expect(matchesPinyin("命运石之门", "m y s z m")).toBe(true);
  });
  it("returns false on miss", () => {
    expect(matchesPinyin("命运石之门", "abcdef")).toBe(false);
  });
});
