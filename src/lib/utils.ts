import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * Convert a local file path to a WebView-safe URL via Tauri's asset protocol.
 * Remote URLs (http/https) and null/empty values are returned unchanged.
 */
export function fileSrc(path: string | null | undefined): string | null {
  if (!path) return null;
  if (path.startsWith("http://") || path.startsWith("https://")) return path;
  return convertFileSrc(path);
}
