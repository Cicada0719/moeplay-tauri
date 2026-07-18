// 搜索结果封面懒补
//
// 流程：合并列表（调用方截 top N）→ 逐项用条目名查 anime_bangumi_search
//   → 取首个带 image 的 subject.image → onCover(key, imageUrl)（调用方负责 _proxyImages 入缓存）。
// 约束：
//   - 并发 ≤ concurrency（默认 3）；
//   - 同 key 只查一次（queried 集合跨 fetch 复用；成功结果缓存并重复投递，流式搜索的
//     多次 ensure 不会产生重复请求）；
//   - isCurrent() 返回 false（搜索 token 已变）时停止取新任务，且结果不再投递；
//   - 失败/无匹配静默——条目保持文字卡。

export interface CoverFetchEntry {
  key: string; // 合并条目 key
  name: string; // 查询关键词（条目展示名）
}

export interface CoverSubjectLike {
  image?: string | null;
}

export interface CoverFetchDeps {
  searchSubjects: (keyword: string) => Promise<CoverSubjectLike[]>;
  onCover: (entryKey: string, imageUrl: string) => void;
  isCurrent?: () => boolean;
  concurrency?: number;
}

export interface SearchCoverFetcher {
  fetch(entries: CoverFetchEntry[], deps: CoverFetchDeps): Promise<void>;
  clear(): void;
}

const CACHE_MAX = 500;
const CACHE_TRIM = 100;

export function createSearchCoverFetcher(): SearchCoverFetcher {
  const resolved = new Map<string, string>(); // key → bangumi 封面原始 URL
  const queried = new Set<string>(); // 已发起过查询的 key（含失败/无图）
  const inFlight = new Set<string>();

  function cacheCover(key: string, image: string) {
    resolved.set(key, image);
    if (resolved.size > CACHE_MAX) {
      const drop = [...resolved.keys()].slice(0, CACHE_TRIM);
      for (const k of drop) resolved.delete(k);
    }
  }

  async function queryOne(entry: CoverFetchEntry, deps: CoverFetchDeps): Promise<void> {
    const cached = resolved.get(entry.key);
    if (cached) {
      deps.onCover(entry.key, cached); // 缓存命中也要投递（新 token 下可重新激活展示）
      return;
    }
    if (queried.has(entry.key) || inFlight.has(entry.key)) return;
    queried.add(entry.key);
    inFlight.add(entry.key);
    try {
      const subjects = await deps.searchSubjects(entry.name.trim());
      const image = subjects.find((s) => s?.image)?.image ?? "";
      if (!image) return;
      cacheCover(entry.key, image);
      if (deps.isCurrent && !deps.isCurrent()) return; // 过期搜索：入缓存但不投递
      deps.onCover(entry.key, image);
    } catch {
      // 静默：保持文字卡
    } finally {
      inFlight.delete(entry.key);
    }
  }

  async function fetch(entries: CoverFetchEntry[], deps: CoverFetchDeps): Promise<void> {
    const queue = entries.filter((e) => e.key && e.name.trim());
    if (queue.length === 0) return;
    const concurrency = Math.max(1, deps.concurrency ?? 3);
    let cursor = 0;
    const worker = async () => {
      while (cursor < queue.length) {
        if (deps.isCurrent && !deps.isCurrent()) return; // 丢弃过期结果
        const entry = queue[cursor++];
        await queryOne(entry, deps);
      }
    };
    const workers = Array.from({ length: Math.min(concurrency, queue.length) }, () => worker());
    await Promise.all(workers);
  }

  return {
    fetch,
    clear() {
      resolved.clear();
      queried.clear();
      inFlight.clear();
    },
  };
}
