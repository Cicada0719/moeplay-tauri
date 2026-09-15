import { offlineApi, type OfflineChapter } from "../../api/offline";

/** Chapter parsing is deliberately bounded independently from video downloads. */
export const OFFLINE_PARSE_CONCURRENCY = 2;

const FAILURE_KEY = "moeplay-offline-failures-v1";

function readFailures(): Record<string, string> {
  if (typeof localStorage === "undefined") return {};
  try {
    const parsed = JSON.parse(localStorage.getItem(FAILURE_KEY) ?? "{}");
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(Object.entries(parsed).filter((entry): entry is [string, string] => typeof entry[0] === "string" && typeof entry[1] === "string"));
  } catch {
    return {};
  }
}

function writeFailures(value: Record<string, string>) {
  if (typeof localStorage === "undefined") return;
  try { localStorage.setItem(FAILURE_KEY, JSON.stringify(value)); } catch { /* storage is best effort */ }
}

/** Keep resolver failures visible after the detail panel is closed. */
export function offlineFailure(chapterKey: string): string | undefined {
  return readFailures()[chapterKey];
}

export function rememberOfflineFailure(chapterKey: string, error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  const failures = readFailures();
  failures[chapterKey] = message.slice(0, 500);
  writeFailures(failures);
}

export function clearOfflineFailure(chapterKey: string): void {
  const failures = readFailures();
  if (!(chapterKey in failures)) return;
  delete failures[chapterKey];
  writeFailures(failures);
}

/** Rehydrate the durable queue after an app restart. The Rust store is the source of truth. */
export async function restoreOfflineQueue(): Promise<OfflineChapter[]> {
  const chapters = await offlineApi.list();
  return chapters.filter((chapter) => chapter.state === "queued" || chapter.state === "downloading" || chapter.state === "paused");
}
