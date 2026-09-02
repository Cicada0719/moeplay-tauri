// 阅读器设置持久化 store（FR-11）
//
// 采用「全局默认 + 单漫画覆盖」两级 localStorage 存储：
//   - 全局默认：localStorage["reader:settings:global"]           → ReaderSettings
//   - 单漫画覆盖：localStorage["reader:settings:manga:{contentId}"] → Partial<ReaderSettings>
// 读取优先级：单漫画覆盖 > 全局默认 > 内置默认值。
//
// 轻量偏好走 localStorage，**不允许**写入 SQLite（避免阻塞），这是本任务的既定决策。

import { writable, type Writable } from "svelte/store";

export type PageMode = 'single' | 'dual';
export type ReadingDirection = 'ltr' | 'rtl';

export interface ReaderSettings {
  pageMode: PageMode;
  /** 默认 'rtl'（日漫从右往左） */
  direction: ReadingDirection;
  /** 窗口 <800px 时用户选择"仍要双页" */
  forceNarrowDual: boolean;
}

export const DEFAULT_SETTINGS: ReaderSettings = {
  pageMode: 'single',
  direction: 'rtl',
  forceNarrowDual: false,
};

export interface ReaderSettingsLoadOptions {
  /** Android media readers start in dual-page mode unless a saved override exists. */
  android?: boolean;
}

export const GLOBAL_SETTINGS_KEY = 'reader:settings:global';

export function mangaSettingsKey(contentId: string): string {
  return `reader:settings:manga:${contentId}`;
}

function storage(): Storage | null {
  return typeof localStorage === 'undefined' ? null : localStorage;
}

function isPageMode(value: unknown): value is PageMode {
  return value === 'single' || value === 'dual';
}

function isReadingDirection(value: unknown): value is ReadingDirection {
  return value === 'ltr' || value === 'rtl';
}

/** 只保留合法字段，丢弃 JSON 中混入的脏数据。 */
function sanitize(patch: Partial<ReaderSettings>): Partial<ReaderSettings> {
  const clean: Partial<ReaderSettings> = {};
  if (isPageMode(patch.pageMode)) clean.pageMode = patch.pageMode;
  if (isReadingDirection(patch.direction)) clean.direction = patch.direction;
  if (typeof patch.forceNarrowDual === 'boolean') clean.forceNarrowDual = patch.forceNarrowDual;
  return clean;
}

function readParsed(key: string): Partial<ReaderSettings> | null {
  const store = storage();
  if (!store) return null;
  const raw = store.getItem(key);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<ReaderSettings>;
    return parsed && typeof parsed === 'object' ? sanitize(parsed) : null;
  } catch (error) {
    // JSON 损坏：回退默认并 warn，不抛异常（U12）。
    console.warn(`[readerSettings] 设置 JSON 解析失败，回退默认（key=${key}）`, error);
    return null;
  }
}

/**
 * 读取设置：合并「默认 ← 全局 ← 单漫画覆盖」。
 * `contentId` 缺省时只读全局层。
 */
export function loadSettings(contentId?: string, options: ReaderSettingsLoadOptions = {}): ReaderSettings {
  const result: ReaderSettings = { ...DEFAULT_SETTINGS, ...(options.android ? { pageMode: 'dual' as const } : {}) };
  Object.assign(result, readParsed(GLOBAL_SETTINGS_KEY));
  if (contentId) Object.assign(result, readParsed(mangaSettingsKey(contentId)));
  return result;
}

/**
 * 保存设置。
 * - `contentId` 为 null → 写全局键（merge 语义）；
 * - `contentId` 给定 → 写 `reader:settings:manga:{contentId}` 键（Partial merge 语义，
 *   仅覆盖本次 patch 的字段，不把全局设置快照写死进单漫画键）。
 */
export function saveSettings(contentId: string | null, patch: Partial<ReaderSettings>): void {
  const store = storage();
  if (!store) return;
  const clean = sanitize(patch);
  if (contentId === null) {
    const next = { ...DEFAULT_SETTINGS, ...readParsed(GLOBAL_SETTINGS_KEY), ...clean };
    store.setItem(GLOBAL_SETTINGS_KEY, JSON.stringify(next));
  } else {
    const next = { ...readParsed(mangaSettingsKey(contentId)), ...clean };
    store.setItem(mangaSettingsKey(contentId), JSON.stringify(next));
  }
}

/**
 * 创建绑定到指定漫画的 Svelte `writable<ReaderSettings>`，其 `set/update`
 * 自动调用 `saveSettings(contentId, …)` 实现持久化（U13）。
 */
export function createReaderSettingsStore(contentId: string, options: ReaderSettingsLoadOptions = {}): Writable<ReaderSettings> {
  let current = loadSettings(contentId, options);
  const { subscribe, set: rawSet } = writable<ReaderSettings>(current);
  return {
    subscribe,
    set(value: ReaderSettings) {
      current = value;
      saveSettings(contentId, value);
      rawSet(value);
    },
    update(fn: (value: ReaderSettings) => ReaderSettings) {
      const next = fn(current);
      current = next;
      saveSettings(contentId, next);
      rawSet(next);
    },
  };
}
