import { gameStore, type Game } from "../../../stores/games.svelte";
import { adaptGamesToPresentation, type GamePresentationActions } from "./gamePresentation";
import type { MediaPresentationItem } from "./types";

export interface GameStorePresentationAdapter {
  readonly items: MediaPresentationItem[];
  readonly allItems: MediaPresentationItem[];
  readonly selectedItem: MediaPresentationItem | null;
  readonly loading: boolean;
  readonly error: string | null;
  refresh: () => Promise<void>;
  importGame: () => Promise<unknown>;
}

export interface GameStoreLike {
  readonly games: Game[];
  readonly allGames: Game[];
  readonly selectedId: string | null;
  readonly loading: boolean;
  readonly loadError: string | null;
  load: () => Promise<void>;
  importGame: () => Promise<unknown>;
  selectGame: (id: string | null) => void;
  launch: (id: string) => void | Promise<void>;
  toggleFavorite: (id: string) => void | Promise<void>;
}

/**
 * Reactive façade over the existing gameStore. Getters intentionally remap on
 * access so Svelte consumers track the production store's rune-backed values.
 */
export function createGameStorePresentationAdapter(
  store: GameStoreLike = gameStore,
  options: Pick<GamePresentationActions, "open"> = {},
): GameStorePresentationAdapter {
  const handlers: GamePresentationActions = {
    open: options.open,
    select: id => store.selectGame(id),
    launch: id => store.launch(id),
    toggleFavorite: id => store.toggleFavorite(id),
  };

  return {
    get items() {
      return adaptGamesToPresentation(store.games, handlers);
    },
    get allItems() {
      return adaptGamesToPresentation(store.allGames, handlers);
    },
    get selectedItem() {
      const selected = store.allGames.find(game => game.id === store.selectedId);
      return selected ? adaptGamesToPresentation([selected], handlers)[0] : null;
    },
    get loading() {
      return store.loading;
    },
    get error() {
      return store.loadError;
    },
    refresh: () => store.load(),
    importGame: () => store.importGame(),
  };
}

export const gamePresentationStore = createGameStorePresentationAdapter();
