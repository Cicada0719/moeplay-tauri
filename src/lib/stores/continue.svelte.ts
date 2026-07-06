import { gameStore } from "./games.svelte";
import { animeStore } from "./anime.svelte";
import { comicStore } from "./comic.svelte";
import {
  buildContinueItems,
  buildContinueStats,
  priorityScore,
  type ContinueItem,
  type ContinueStats,
} from "../utils/continue";

export type { ContinueItem, ContinueStats };

let _items = $state<ContinueItem[]>([]);
let _started = false;

function refresh() {
  const items = buildContinueItems(
    gameStore.allGames,
    animeStore.history,
    comicStore.readHistory
  );
  _items = items;
}

export const continueStore = {
  /** Start reactive refresh. Must be called during component initialisation (e.g. in App). */
  start() {
    if (_started) return () => {};
    _started = true;
    $effect(() => {
      gameStore.allGames;
      animeStore.history;
      comicStore.readHistory;
      refresh();
    });
    return () => { _started = false; };
  },

  get items() { return _items; },
  get games() { return _items.filter(i => i.type === "game"); },
  get anime() { return _items.filter(i => i.type === "anime"); },
  get comics() { return _items.filter(i => i.type === "comic"); },
  get totalCount() { return _items.length; },
  get stats(): ContinueStats {
    return buildContinueStats(
      _items,
      gameStore.allGames,
      animeStore.history,
      comicStore.readHistory
    );
  },
  get topItem(): ContinueItem | null {
    if (_items.length === 0) return null;
    return _items.reduce((best, item) =>
      priorityScore(item) > priorityScore(best) ? item : best
    );
  },
};
