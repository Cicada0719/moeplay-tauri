import { animeStore } from "../../stores/anime.svelte";
import { comicStore, type ReadRecord } from "../../stores/comic.svelte";
import { novelStore } from "../novel/store.svelte";
import type { AnimeHistory } from "../anime-player/historyStore.svelte";
import type { NovelHistoryEntry } from "../novel/types";
import { navigateTo } from "../../stores/router.svelte";
import { uiStore } from "../../stores/ui.svelte";
import type { UnifiedMediaHistoryKind, UnifiedMediaHistoryPayload } from "./unified";

export type MediaHistoryOpenPayload =
  | { kind: "anime"; payload: AnimeHistory }
  | { kind: "comic"; payload: ReadRecord }
  | { kind: "novel"; payload: NovelHistoryEntry };

/**
 * 所有“继续”入口共用的恢复动作：先切到正确媒体页，再调用原 store 的
 * 续播/续读逻辑。这样历史卡片、掌机 XMB 和记录档案不会各自漂移。
 */
export async function openUnifiedMediaHistory(entry: MediaHistoryOpenPayload | { kind: UnifiedMediaHistoryKind; payload: UnifiedMediaHistoryPayload }): Promise<boolean> {
  navigateTo(entry.kind, { focus: "none" });

  if (entry.kind === "anime") {
    const payload = entry.payload as AnimeHistory;
    if (!payload.ruleName || !payload.sourceUrl) {
      uiStore.notify("这条番剧记录缺少可用来源，请到番剧页重新搜索后播放。", "error");
      return false;
    }
    await animeStore.resumeHistory(payload);
    return true;
  }

  if (entry.kind === "comic") {
    await comicStore.resumeHistory(entry.payload as ReadRecord);
    return true;
  }

  await novelStore.resume(entry.payload as NovelHistoryEntry);
  return true;
}

