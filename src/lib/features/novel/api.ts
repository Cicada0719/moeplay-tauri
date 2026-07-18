import { invokeCmd } from "../../api/core";
import type { NovelBook, NovelChapterContent, NovelDetail, NovelSource } from "./types";

export function searchNovels(source: NovelSource, query: string): Promise<NovelBook[]> {
  return invokeCmd("novel_search", { source, query });
}

export function loadNovelDetail(source: Exclude<NovelSource, "all">, bookId: string): Promise<NovelDetail> {
  return invokeCmd("novel_detail", { source, bookId });
}

export function readNovelChapter(
  source: Exclude<NovelSource, "all">,
  bookId: string,
  chapterId: string,
): Promise<NovelChapterContent> {
  return invokeCmd("novel_read_chapter", { source, bookId, chapterId });
}
