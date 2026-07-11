export const EXTENSION_INDEX_ENDPOINT_STORAGE_KEY = "moeplay-extension-index-endpoint-v1";

const MAX_ENDPOINT_LENGTH = 2048;

function isLoopbackHost(hostname: string): boolean {
  const normalized = hostname.toLowerCase();
  return normalized === "localhost" || normalized === "::1" || /^127(?:\.\d{1,3}){3}$/.test(normalized);
}

/**
 * Mirrors the backend endpoint policy before anything is persisted or invoked.
 * This configuration intentionally contains only a public metadata URL: user
 * info, query strings, and fragments are rejected so credentials cannot enter
 * localStorage or the command bridge.
 */
export function normalizeExtensionIndexEndpoint(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const endpoint = value.trim();
  if (!endpoint || endpoint.length > MAX_ENDPOINT_LENGTH || /[\u0000-\u001F\u007F]/.test(endpoint)) return null;

  try {
    const url = new URL(endpoint);
    const loopbackHttp = url.protocol === "http:" && isLoopbackHost(url.hostname);
    if ((url.protocol !== "https:" && !loopbackHttp) || url.username || url.password || url.search || url.hash) return null;
    if (!url.pathname) url.pathname = "/";
    return url.toString();
  } catch {
    return null;
  }
}

function browserStorage(): Storage | null {
  return typeof localStorage === "undefined" ? null : localStorage;
}

export function getConfiguredExtensionIndexEndpoint(): string | null {
  try {
    const storage = browserStorage();
    const endpoint = normalizeExtensionIndexEndpoint(storage?.getItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY));
    if (!endpoint) storage?.removeItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY);
    return endpoint;
  } catch {
    return null;
  }
}

export function saveExtensionIndexEndpoint(value: unknown): string {
  const endpoint = normalizeExtensionIndexEndpoint(value);
  if (!endpoint) {
    throw new Error("请输入不含凭据、查询参数或片段的 HTTPS 目录端点（仅允许本机 HTTP）。");
  }

  try {
    browserStorage()?.setItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY, endpoint);
  } catch {
    // A configured endpoint remains usable for the current session if storage is unavailable.
  }
  return endpoint;
}

export function clearExtensionIndexEndpoint(): void {
  try {
    browserStorage()?.removeItem(EXTENSION_INDEX_ENDPOINT_STORAGE_KEY);
  } catch {
    // Storage is optional and must not prevent disabling sync for this session.
  }
}
