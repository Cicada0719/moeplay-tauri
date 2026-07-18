export type NovelSource = "all" | "gutenberg" | "wikisource";

export interface NovelBook {
  id: string;
  source: Exclude<NovelSource, "all">;
  title: string;
  author?: string;
  summary?: string;
  coverUrl?: string;
  language?: string;
  subjects: string[];
  publicDomain: boolean;
  sourceUrl: string;
  downloadUrl?: string;
}

export interface NovelChapter {
  id: string;
  title: string;
  order: number;
}

export interface NovelDetail {
  book: NovelBook;
  chapters: NovelChapter[];
}

export interface NovelChapterContent {
  bookId: string;
  source: Exclude<NovelSource, "all">;
  chapter: NovelChapter;
  content: string;
}

export interface NovelHistoryEntry {
  key: string;
  book: NovelBook;
  chapterId: string;
  chapterTitle: string;
  progress: number;
  updatedAt: number;
}
