import { offlineControl, offlineEnqueue, offlineSupplyChapter, type OfflineChapter, type OfflineContentType, type OfflineResourceInput } from "../../api/offline";
import { selectOfflineChapters, type ChapterLike } from "./model";
import { clearOfflineFailure, OFFLINE_PARSE_CONCURRENCY, rememberOfflineFailure } from "./runtime";

export interface OfflineSupplyPayload { body?: string; resources?: OfflineResourceInput[]; }
export interface OfflineChapterResolver<T extends ChapterLike> { (chapter: T): Promise<OfflineSupplyPayload>; }
export interface OfflineScheduleResult { chapters: OfflineChapter[]; failures: Array<{ chapterId: string; error: string }>; }

function errorText(error: unknown): string { return error instanceof Error ? error.message : String(error); }

/** Run chapter parsing with a hard upper bound of two concurrent source reads. */
export async function scheduleOfflineChapters<T extends ChapterLike>(input: {
  contentType: OfflineContentType; sourceId: string; contentId: string; title: string;
  chapters: T[]; currentId: string; following?: number; selected?: T[]; resolve: OfflineChapterResolver<T>;
}): Promise<OfflineScheduleResult> {
  const selected = input.selected?.length ? input.selected : selectOfflineChapters(input.chapters, input.currentId, input.following ?? 5);
  if (!selected.length) return { chapters: [], failures: [] };
  const queued = await offlineEnqueue({
    contentType: input.contentType, sourceId: input.sourceId, contentId: input.contentId,
    title: input.title, chapters: selected.map((chapter, index) => ({
      chapterId: chapter.id, order: chapter.order ?? index, title: chapter.title ?? `第 ${chapter.order ?? index} 章`, resources: [],
    })),
  });
  const result = [...queued.chapters];
  const failures: Array<{ chapterId: string; error: string }> = [];
  let cursor = 0;
  async function worker() {
    while (cursor < selected.length) {
      const index = cursor++;
      const chapter = selected[index];
      try {
        const payload = await input.resolve(chapter);
        const saved = await offlineSupplyChapter({ chapterKey: result[index].offlineChapterKey, ...payload });
        result[index] = saved;
        clearOfflineFailure(result[index].offlineChapterKey);
      } catch (error) {
        const chapterKey = result[index]?.offlineChapterKey;
        if (chapterKey) {
          rememberOfflineFailure(chapterKey, error);
          // A resolver failure must be retryable from DownloadPage instead of
          // leaving a task spinning in the queued state forever.
          void offlineControl({ chapterKey, action: "pause" }).catch(() => {});
        }
        failures.push({ chapterId: chapter.id, error: errorText(error) });
      }
    }
  }
  await Promise.all(Array.from({ length: OFFLINE_PARSE_CONCURRENCY }, () => worker()));
  return { chapters: result, failures };
}
