import { gameStore } from "./games.svelte";
import { continueSource } from "./continue-source.svelte";
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
    continueSource.animeHistory,
    continueSource.comicHistory,
  );
  _items = items;
}

export const continueStore = {
  /** Start reactive refresh. Must be called during component initialisation (e.g. in App). */
  start() {
    if (_started) return () => {};
    _started = true;
    // 主包不再静态加载 anime/comic store：启动后延迟在后台预载，
    // store 模块加载完成会自动把历史同步进 continueSource。
    const timer = setTimeout(() => {
      void import("./anime.svelte");
      void import("./comic.svelte");
    }, 1000);
    $effect(() => {
      gameStore.allGames;
      continueSource.animeHistory;
      continueSource.comicHistory;
      refresh();
    });
    return () => { clearTimeout(timer); _started = false; };
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
      continueSource.animeHistory,
      continueSource.comicHistory,
    );
  },
  get topItem(): ContinueItem | null {
    if (_items.length === 0) return null;
    return _items.reduce((best, item) =>
      priorityScore(item) > priorityScore(best) ? item : best
    );
  },
};
