// 历史列表页纯工具函数（spec §4 步骤 4 / 步骤 4.7）
//
// - `formatRelativeTime`：相对时间（刚刚 / n 分钟前 / n 小时前 / n 天前 / 超 7 天显示日期）
// - `highlight`：把标题关键词命中子串拆成 `{pre, hit, post}` 片段数组，避免 `{@html}` 注入 XSS
// - 类型标签 / 进度文案等展示常量

import type { ContentType } from "../../history/types";

export const CONTENT_TYPE_LABELS: Record<ContentType, string> = {
  anime: '番剧',
  manga: '漫画',
  novel: '小说',
};

export const CONTENT_TYPE_ICONS: Record<ContentType, string> = {
  anime: 'film',
  manga: 'book',
  novel: 'collection',
};

/** 进度文案：漫画「第 N 页」、小说「N%」、番剧「N 分 N 秒」（本任务只做展示，不实现番剧续播）。 */
export function progressLabel(item: {
  contentType: ContentType;
  pageIndex: number;
  scrollPct: number;
  positionSec: number;
}): string {
  if (item.contentType === 'manga') return `第 ${Math.max(1, item.pageIndex + 1)} 页`;
  if (item.contentType === 'novel') {
    const pct = Math.max(0, Math.min(100, Math.round(item.scrollPct * 100)));
    return `${pct}%`;
  }
  const seconds = Math.max(0, Math.floor(item.positionSec));
  const minutes = Math.floor(seconds / 60);
  return minutes > 0 ? `${minutes} 分 ${seconds % 60} 秒` : `${seconds} 秒`;
}

/**
 * 相对时间（`updatedAt` 为 Unix 毫秒）。
 * 刚刚 / n 分钟前 / n 小时前 / n 天前 / 超过 7 天显示 yyyy-MM-dd。
 */
export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  if (!Number.isFinite(timestamp)) return '';
  const delta = Math.max(0, now - timestamp);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (delta < minute) return '刚刚';
  if (delta < hour) return `${Math.floor(delta / minute)} 分钟前`;
  if (delta < day) return `${Math.floor(delta / hour)} 小时前`;
  if (delta < 7 * day) return `${Math.floor(delta / day)} 天前`;
  const date = new Date(timestamp);
  const yyyy = date.getFullYear();
  const mm = String(date.getMonth() + 1).padStart(2, '0');
  const dd = String(date.getDate()).padStart(2, '0');
  return `${yyyy}-${mm}-${dd}`;
}

export interface HighlightSegment {
  /** 命中片段前的文本 */
  pre: string;
  /** 命中片段（小写化后完全匹配的关键词原文） */
  hit: string;
  /** 最后一个命中片段之后的剩余文本 */
  post: string;
}

/**
 * 把 `keyword` 在 `text` 中的每次命中拆成一段 `{pre, hit}`；仅最后一段带 `post`。
 * 无命中时返回单段 `{pre: text, hit: '', post: ''}`。渲染方式：
 * ```
 * {#each segments as seg, i}
 *   {seg.pre}<mark class="hl">{seg.hit}</mark>{i === segments.length - 1 ? seg.post : ''}
 * {/each}
 * ```
 * 纯文本渲染（无 `{@html}`），天然避免 XSS。
 */
export function highlight(text: string, keyword: string): HighlightSegment[] {
  const normalized = keyword.trim().toLowerCase();
  if (!normalized) return [{ pre: text, hit: '', post: '' }];
  const lower = text.toLowerCase();
  const segments: HighlightSegment[] = [];
  let cursor = 0;
  let index = lower.indexOf(normalized, cursor);
  while (index >= 0) {
    segments.push({
      pre: text.slice(cursor, index),
      hit: text.slice(index, index + normalized.length),
      post: '',
    });
    cursor = index + normalized.length;
    index = lower.indexOf(normalized, cursor);
  }
  if (segments.length === 0) return [{ pre: text, hit: '', post: '' }];
  segments[segments.length - 1].post = text.slice(cursor);
  return segments;
}
