import type { OfflineChapter, OfflineContentType } from "../../api/offline";

export interface ChapterLike { id: string; title?: string; order?: number; }

/** Select only entries in the already loaded catalog, preserving catalog order. */
export function selectOfflineChapters<T extends ChapterLike>(chapters: T[], currentId: string, following = 5): T[] {
  const index = chapters.findIndex((chapter) => chapter.id === currentId);
  if (index < 0) return [];
  const count = Number.isFinite(following) ? Math.max(0, Math.trunc(following)) : 5;
  return chapters.slice(index, index + count + 1);
}

export function findOfflineChapter(chapters: OfflineChapter[], identity: {
  contentType: OfflineContentType; sourceId: string; contentId: string; chapterId: string;
}): OfflineChapter | undefined {
  return chapters.find((chapter) => chapter.contentType === identity.contentType
    && chapter.sourceId === identity.sourceId && chapter.contentId === identity.contentId
    && chapter.chapterId === identity.chapterId);
}

export function offlineSupportsSource(source: string, mode?: string): { supported: boolean; reason?: string } {
  if (mode === "webview" || mode === "external" || source === "dm5" || source === "ikkk") {
    return { supported: false, reason: "网页/外部阅读模式不支持章节离线" };
  }
  return { supported: true };
}
