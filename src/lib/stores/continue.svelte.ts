import { gameStore } from "./games.svelte";
import { animeStore } from "./anime.svelte";
import { comicStore } from "./comic.svelte";
import { gameLastPlayed, gameCompletionStatus, gameTotalSeconds, coverOf } from "../utils/game";

export interface ContinueItem {
  id: string;
  type: "game" | "anime" | "comic";
  title: string;
  cover: string | null;
  progress: number;
  progressLabel: string;
  lastActivity: number;
}

let _items = $state<ContinueItem[]>([]);

function refreshItems() {
  const items: ContinueItem[] = [];

  // Games: recently played
  for (const g of gameStore.allGames) {
    const lastPlayed = gameLastPlayed(g);
    if (!lastPlayed) continue;
    const status = gameCompletionStatus(g);
    if (status === "completed" || status === "dropped") continue;
    const ts = new Date(lastPlayed).getTime();
    if (isNaN(ts)) continue;
    const totalSec = gameTotalSeconds(g);
    items.push({
      id: `game-${g.id}`,
      type: "game",
      title: g.name,
      cover: coverOf(g) || null,
      progress: 0,
      progressLabel: totalSec > 0 ? `${(totalSec / 3600).toFixed(1)}h` : "未玩",
      lastActivity: ts,
    });
  }

  // Anime: history items with episodes
  for (const h of animeStore.history) {
    if (h.lastEpisode <= 0) continue;
    const ts = h.updatedAt ? new Date(h.updatedAt).getTime() : 0;
    items.push({
      id: `anime-${h.key}`,
      type: "anime",
      title: h.name,
      cover: h.image ? animeStore.getImg(h.image) || h.image : null,
      progress: 0,
      progressLabel: `第${h.lastEpisode}话`,
      lastActivity: ts || Date.now(),
    });
  }

  // Comics: read history
  for (const h of comicStore.readHistory) {
    const ts = h.ts || Date.now();
    items.push({
      id: `comic-${h.id}`,
      type: "comic",
      title: h.title,
      cover: h.thumb_url || null,
      progress: 0,
      progressLabel: h.last_title || `第${h.last_order}话`,
      lastActivity: ts,
    });
  }

  // Sort by last activity descending, take top 30
  items.sort((a, b) => b.lastActivity - a.lastActivity);
  _items = items.slice(0, 30);
}

// Auto-refresh when stores change
$effect(() => {
  // Touch reactive dependencies
  gameStore.allGames;
  animeStore.history;
  comicStore.readHistory;
  refreshItems();
});

export const continueStore = {
  get items() { return _items; },
  get games() { return _items.filter(i => i.type === "game"); },
  get anime() { return _items.filter(i => i.type === "anime"); },
  get comics() { return _items.filter(i => i.type === "comic"); },
  get totalCount() { return _items.length; },
};
