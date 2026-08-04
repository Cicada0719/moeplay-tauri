// 历史记录数据类型（FR-10 / FR-08 wire 对齐）
//
// 字段与子任务 4 的 Rust `HistoryRecord` 序列化输出（camelCase）对齐：
//   - `updatedAt` 为 **Unix 毫秒**（子任务 4 wire 格式如此，非 spec 草稿里的"秒"）；
//   - `deleted` 为布尔（Rust 侧 bool）；
//   - 额外的 `progress` 是 Rust 序列化层按 content_type 计算的展示字段。
// 子任务 4 若已建立等价 TS 类型则复用其定义；当前主线尚无，故在本任务内建立。

export type ContentType = 'anime' | 'manga' | 'novel';

export interface HistoryItem {
  /** uuid */
  id: string;
  contentId: string;
  contentType: ContentType;
  title: string;
  cover: string | null;
  sourceId: string;
  chapterId: string | null;
  chapterTitle: string | null;
  /** 漫画：单页序号（原子单位，与渲染模式无关）；番剧：集内位置 */
  pageIndex: number;
  /** 番剧播放秒数（本任务不使用） */
  positionSec: number;
  /** 小说滚动百分比 0~1 */
  scrollPct: number;
  /** 按类型计算的推进位置（番剧=positionSec / 漫画=pageIndex / 小说=scrollPct） */
  progress: number;
  /** Unix 毫秒 */
  updatedAt: number;
  deviceId: string;
  /** 墓碑标记；列表查询默认过滤 deleted=true */
  deleted: boolean;
}

/** 历史列表查询参数（`historyApi.ts` 适配子任务 4 的 `history_list` 签名） */
export interface HistoryQuery {
  /** 不传 = 全部 */
  contentType?: ContentType;
  /** 标题关键词（本地 IPC 全量拉取后前端过滤见 spec §4 步骤 4 决策） */
  keyword?: string;
  limit?: number;
  offset?: number;
  /** 默认 false */
  includeDeleted?: boolean;
}

export interface HistoryPage {
  items: HistoryItem[];
  /** 满足筛选条件的总数（用于虚拟滚动高度计算） */
  total: number;
}
