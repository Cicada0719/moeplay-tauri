/** Local reading positions. Legacy localStorage is deliberately never modified. */
export interface ReadingPosition {
  kind: "comic" | "novel";
  source: string;
  contentId: string;
  chapterId: string;
  chapterTitle: string;
  title: string;
  updatedAt: number;
  pageIndex?: number;
  pageId?: string;
  progress?: number;
  metadata: Record<string, unknown>;
}

export const bookKey = (p: Pick<ReadingPosition, "kind" | "source" | "contentId">) =>
  JSON.stringify([p.kind, p.source, p.contentId]);
export const chapterKey = (p: ReadingPosition) => JSON.stringify([bookKey(p), p.chapterId]);

export function validPosition(value: unknown): value is ReadingPosition {
  if (!value || typeof value !== "object") return false;
  const p = value as ReadingPosition;
  if (p.kind === "novel") {
    const book = p.metadata?.book as Record<string, unknown> | undefined;
    if (!book || book.id !== p.contentId || book.source !== p.source || typeof book.title !== "string") return false;
  }
  return (p.kind === "comic" || p.kind === "novel")
    && [p.source, p.contentId, p.chapterId, p.title].every(v => typeof v === "string" && v.length > 0)
    && typeof p.chapterTitle === "string" && Number.isFinite(p.updatedAt) && p.updatedAt > 0
    && !!p.metadata && typeof p.metadata === "object" && !Array.isArray(p.metadata)
    && (p.pageIndex === undefined || (Number.isInteger(p.pageIndex) && p.pageIndex >= 0))
    && (p.pageId === undefined || typeof p.pageId === "string")
    && (p.progress === undefined || (Number.isFinite(p.progress) && p.progress >= 0 && p.progress <= 1));
}

export function latestBooks(positions: ReadingPosition[]): ReadingPosition[] {
  const books = new Map<string, ReadingPosition>();
  for (const p of positions) {
    if (!validPosition(p)) continue;
    const key = bookKey(p);
    if (!books.has(key) || books.get(key)!.updatedAt < p.updatedAt) books.set(key, p);
  }
  return [...books.values()].sort((a, b) => b.updatedAt - a.updatedAt);
}

export function migrateLegacy(novel: unknown, comic: unknown): ReadingPosition[] {
  const positions: ReadingPosition[] = [];
  for (const e of Array.isArray(novel) ? novel : []) {
    if (!e || !e.book) continue;
    const p: ReadingPosition = { kind: "novel", source: e.book.source, contentId: e.book.id,
      title: e.book.title, chapterId: e.chapterId, chapterTitle: e.chapterTitle,
      progress: e.progress, updatedAt: e.updatedAt, metadata: { book: e.book } };
    if (validPosition(p)) positions.push(p);
  }
  for (const e of Array.isArray(comic) ? comic : []) {
    if (!e || typeof e.id !== "string" || !Number.isFinite(e.last_order)) continue;
    const match = /^(mangadex|baozi|dm5|ikkk):(.+)$/.exec(e.id);
    const p: ReadingPosition = { kind: "comic", source: match?.[1] ?? "picacg",
      contentId: match?.[2] ?? e.id, title: e.title, chapterId: String(e.last_order),
      chapterTitle: e.last_title ?? "", pageIndex: 0, updatedAt: e.ts,
      metadata: { legacy: e } };
    if (validPosition(p)) positions.push(p);
  }
  return positions;
}

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => { req.onsuccess = () => resolve(req.result); req.onerror = () => reject(req.error); });
}
function complete(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onabort = tx.onerror = () => reject(tx.error ?? new Error("阅读历史事务未完成"));
  });
}

export class ReadingRepository {
  private db?: IDBDatabase;
  private opening?: Promise<void>;
  private listeners = new Set<() => void>();
  private pending = new Map<string, ReadingPosition>();
  positions: ReadingPosition[] = [];
  ready = false;
  error = "";
  subscribe(listener: () => void) { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; }
  private notify() { for (const listener of this.listeners) listener(); }
  private failed(error: unknown) { this.error = `阅读记录未保存，请重试或导出备份：${String(error)}`; this.notify(); }

  async init(): Promise<void> {
    if (this.ready) return;
    if (this.opening) return this.opening;
    this.opening = this.open().catch(error => { this.failed(error); throw error; }).finally(() => { this.opening = undefined; });
    return this.opening;
  }
  private async open() {
    const req = indexedDB.open("moeplay-reading-history", 2);
    req.onupgradeneeded = () => {
      for (const store of ["books", "chapters", "meta"]) {
        if (!req.result.objectStoreNames.contains(store)) req.result.createObjectStore(store);
      }
    };
    req.onblocked = () => this.failed("请关闭旧窗口后重试");
    this.db = await request(req);
    this.db.onversionchange = () => { this.db?.close(); this.ready = false; this.failed("存储版本已更新，请重新打开应用"); };
    const tx = this.db.transaction(["chapters", "meta"], "readonly");
    const done = complete(tx);
    const [rows, migrated] = await Promise.all([
      request(tx.objectStore("chapters").getAll()), request(tx.objectStore("meta").get("legacy-v1")),
    ]);
    await done;
    this.positions = rows.filter(validPosition);
    if (!migrated) {
      const read = (key: string): unknown => {
        const raw = localStorage.getItem(key);
        try { return JSON.parse(raw ?? "[]"); } catch { return []; }
      };
      await this.merge(migrateLegacy(read("moeplay-novel-history-v1"), read("picacg-history")), true);
    }
    this.ready = true;
    this.error = "";
    this.notify();
  }

  private async merge(incoming: ReadingPosition[], migrated = false) {
    if (!this.db) throw new Error("阅读历史尚未加载");
    // Read and merge inside the same transaction: another window cannot overwrite newer positions.
    const tx = this.db.transaction(["chapters", "books", "meta"], "readwrite");
    const done = complete(tx);
    const chapters = tx.objectStore("chapters");
    const rows = await request(chapters.getAll());
    const merged = new Map<string, ReadingPosition>(rows.filter(validPosition).map(p => [chapterKey(p), p]));
    for (const p of incoming) {
      const key = chapterKey(p);
      if (!merged.has(key) || merged.get(key)!.updatedAt <= p.updatedAt) merged.set(key, p);
    }
    for (const p of incoming) {
      const key = chapterKey(p);
      if (merged.get(key) === p) chapters.put(p, key);
    }
    const values = [...merged.values()];
    for (const p of latestBooks(values)) tx.objectStore("books").put(p, bookKey(p));
    if (migrated) tx.objectStore("meta").put(true, "legacy-v1");
    await done;
    this.positions = values;
    this.error = "";
    this.notify();
  }
  async save(position: ReadingPosition) {
    try {
      if (!validPosition(position)) throw new Error("无效阅读位置");
      this.pending.set(chapterKey(position), position);
      await this.init();
      await this.merge([position]);
      if (this.pending.get(chapterKey(position)) === position) this.pending.delete(chapterKey(position));
    } catch (error) { this.failed(error); throw error; }
  }
  async retry() {
    await this.init();
    for (const p of [...this.pending.values()]) await this.save(p);
  }
  async remove(key: string) {
    try {
      await this.init();
      const tx = this.db!.transaction(["chapters", "books"], "readwrite");
      const done = complete(tx);
      const rows: ReadingPosition[] = await request(tx.objectStore("chapters").getAll());
      for (const p of rows.filter(validPosition)) if (bookKey(p) === key) tx.objectStore("chapters").delete(chapterKey(p));
      tx.objectStore("books").delete(key);
      await done;
      for (const [chapter, position] of this.pending) if (bookKey(position) === key) this.pending.delete(chapter);
      this.positions = rows.filter(p => validPosition(p) && bookKey(p) !== key);
      this.notify();
    } catch (error) { this.failed(error); throw error; }
  }
  async exportJSON() {
    await this.init();
    const tx = this.db!.transaction("chapters", "readonly");
    const rows = await request(tx.objectStore("chapters").getAll());
    const positions = new Map<string, ReadingPosition>(rows.filter(validPosition).map(p => [chapterKey(p), p]));
    for (const [key, p] of this.pending) if (!positions.has(key) || positions.get(key)!.updatedAt <= p.updatedAt) positions.set(key, p);
    return JSON.stringify({ format: "moeplay-reading-history", version: 2, exportedAt: Date.now(), positions: [...positions.values()] }, null, 2);
  }
  async importJSON(json: string) {
    const data = JSON.parse(json);
    if (data?.format !== "moeplay-reading-history" || data.version !== 2 || !Array.isArray(data.positions)) throw new Error("不支持的阅读历史备份格式");
    const positions = data.positions.filter(validPosition);
    if (data.positions.length && !positions.length) throw new Error("备份没有有效记录");
    await this.init();
    await this.merge(positions);
    return { imported: positions.length, skipped: data.positions.length - positions.length };
  }
}

export const readingRepository = new ReadingRepository();
