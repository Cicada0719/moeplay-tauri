// 漫画预取 LRU 缓存（spec §4 步骤 5.6）
//
// 模块级单例：跨 ComicReader 实例共享，避免每次挂载新建、预取状态丢失。
// 占用按估算字节（width*height*4）累计，>200MB 时淘汰最久未用项；
// 淘汰时置 `img.src=''` 并删除 Image 引用，释放解码位图依赖浏览器 GC
//（与 spec「记录元数据即可 + 释放引用」语义一致）。

export interface PreloadEntry {
  url: string;
  lastUsed: number;
  /** 估算占用字节 = width*height*4；未探测到尺寸时为 0 */
  bytes: number;
  /** 预取创建的 Image 实例引用；淘汰时置 src='' 并置 null 释放 */
  img: HTMLImageElement | null;
}

export const MAX_PRELOAD_BYTES = 200 * 1024 * 1024;

const preloadCache = new Map<string, PreloadEntry>();
let preloadBytes = 0;

/**
 * 触碰 URL：标记为最近使用。`bytes` 为估算占用（width*height*4）；
 * 对已在缓存中的条目，若传入的 bytes 更大则回填（首次探测到尺寸后补账），
 * 并触发超预算淘汰。
 */
export function touchPreload(url: string, bytes = 0): void {
  const existing = preloadCache.get(url);
  if (existing) {
    existing.lastUsed = Date.now();
    if (bytes > existing.bytes) {
      preloadBytes += bytes - existing.bytes;
      existing.bytes = bytes;
    }
    evictOverBudget();
    return;
  }
  preloadCache.set(url, { url, lastUsed: Date.now(), bytes, img: null });
  preloadBytes += bytes;
  evictOverBudget();
}

/**
 * 为 URL 关联预取 Image 引用（仅在缓存中存有该 entry 时生效）。
 * 替换已有引用时先置旧引用 src='' 释放。
 */
export function attachPreloadImage(url: string, img: HTMLImageElement | null): void {
  const entry = preloadCache.get(url);
  if (!entry) return;
  if (entry.img && entry.img !== img) entry.img.src = '';
  entry.img = img;
}

/** 淘汰最久未用项直到总占用 ≤ 上限。 */
function evictOverBudget(): void {
  while (preloadBytes > MAX_PRELOAD_BYTES && preloadCache.size > 0) {
    let oldestKey: string | null = null;
    let oldest = Number.POSITIVE_INFINITY;
    for (const [key, entry] of preloadCache) {
      if (entry.lastUsed < oldest) {
        oldest = entry.lastUsed;
        oldestKey = key;
      }
    }
    if (oldestKey === null) break;
    const entry = preloadCache.get(oldestKey);
    if (entry) {
      if (entry.img) {
        entry.img.src = '';
        entry.img = null;
      }
      preloadBytes = Math.max(0, preloadBytes - entry.bytes);
    }
    preloadCache.delete(oldestKey);
  }
}

export function hasPreload(url: string): boolean {
  return preloadCache.has(url);
}

export function getPreloadEntry(url: string): PreloadEntry | undefined {
  return preloadCache.get(url);
}

export function getPreloadBytes(): number {
  return preloadBytes;
}

export function getPreloadSize(): number {
  return preloadCache.size;
}

/** 清空缓存并释放所有 Image 引用（测试隔离 / 应用卸载用）。 */
export function clearPreloadCache(): void {
  for (const entry of preloadCache.values()) {
    if (entry.img) {
      entry.img.src = '';
      entry.img = null;
    }
  }
  preloadCache.clear();
  preloadBytes = 0;
}
