// 历史续读入口接线（spec §4 步骤 7）
//
// 项目为自定义 hash 路由（`src/lib/stores/router.svelte.ts`），当前不存在
// `src/routes/history/+page.svelte` 或等价的历史页路由文件；且路由/导航文件不在
// 本任务模块边界内。这里把「点击历史条目 → 阅读器路由参数」的映射封装成纯函数，
// 供未来历史页路由在 `HistoryList` 的 `onresume` 回调中调用：
//   - manga → 漫画阅读器（`initialPageIndex = item.pageIndex`，±0 页）
//   - novel → 小说阅读器（`initialScrollPct = item.scrollPct`，±5%）
//   - anime → 番剧占位（任务 3 范围，本任务不实现番剧续播）
//
// 该函数零依赖，可被 vitest 直测。

import type { HistoryItem } from "./types";

export type ResumeTarget =
  | {
      kind: "manga";
      contentId: string;
      sourceId: string;
      chapterId: string | null;
      chapterTitle: string | null;
      title: string;
      cover: string | null;
      /** 单页原子序号（0-based，与渲染模式无关） */
      pageIndex: number;
    }
  | {
      kind: "novel";
      contentId: string;
      sourceId: string;
      chapterId: string | null;
      chapterTitle: string | null;
      title: string;
      cover: string | null;
      /** 滚动百分比 0~1 */
      scrollPct: number;
    }
  | {
      kind: "anime";
      contentId: string;
      sourceId: string;
      chapterId: string | null;
      /** 播放秒数（本任务不消费，供任务 3 续播） */
      positionSec: number;
    };

/**
 * 把一条历史记录解析为续读目标。
 * 页索引以单页原子单位透传（R8），不在此处做渲染层配对。
 */
export function resolveResumeTarget(item: HistoryItem): ResumeTarget {
  if (item.contentType === "manga") {
    return {
      kind: "manga",
      contentId: item.contentId,
      sourceId: item.sourceId,
      chapterId: item.chapterId,
      chapterTitle: item.chapterTitle,
      title: item.title,
      cover: item.cover,
      pageIndex: Math.max(0, Math.trunc(item.pageIndex) || 0),
    };
  }
  if (item.contentType === "novel") {
    return {
      kind: "novel",
      contentId: item.contentId,
      sourceId: item.sourceId,
      chapterId: item.chapterId,
      chapterTitle: item.chapterTitle,
      title: item.title,
      cover: item.cover,
      scrollPct: Math.max(0, Math.min(1, item.scrollPct)),
    };
  }
  return {
    kind: "anime",
    contentId: item.contentId,
    sourceId: item.sourceId,
    chapterId: item.chapterId,
    positionSec: Math.max(0, item.positionSec),
  };
}
