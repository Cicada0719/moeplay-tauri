export const LIBRARY_V2_FLAG = "library_v2";

export function parseLibraryV2Flag(value: string | null | undefined): boolean {
  if (!value) return false;
  return ["1", "true", "enabled", "on"].includes(value.trim().toLowerCase());
}

export function readLibraryV2Flag(storage: Pick<Storage, "getItem"> | null = typeof localStorage === "undefined" ? null : localStorage): boolean {
  if (!storage) return false;
  try {
    return parseLibraryV2Flag(storage.getItem(LIBRARY_V2_FLAG));
  } catch {
    return false;
  }
}

export function writeLibraryV2Flag(
  enabled: boolean,
  storage: Pick<Storage, "setItem"> | null = typeof localStorage === "undefined" ? null : localStorage,
): void {
  if (!storage) return;
  try {
    storage.setItem(LIBRARY_V2_FLAG, enabled ? "1" : "0");
  } catch {
    // Storage can be unavailable in hardened WebViews. The in-memory UI state still works.
  }
}
