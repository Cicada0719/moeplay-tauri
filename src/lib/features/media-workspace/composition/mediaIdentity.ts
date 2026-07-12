export function normalizeMediaIdentity(src: string): string {
  const value = src.trim().replaceAll("\\", "/");
  if (!value) return "";
  if (/^[a-zA-Z]:\//.test(value)) {
    return value.replace(/\/{2,}/g, "/").toLowerCase();
  }
  try {
    const url = new URL(value);
    url.hash = "";
    const sorted = [...url.searchParams.entries()].sort(([a], [b]) => a.localeCompare(b));
    url.search = "";
    for (const [key, entry] of sorted) url.searchParams.append(key, entry);
    return url.toString();
  } catch {
    return value.replace(/\/{2,}/g, "/").toLowerCase();
  }
}
