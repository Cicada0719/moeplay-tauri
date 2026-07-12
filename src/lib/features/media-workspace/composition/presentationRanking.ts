import type { MediaPresentationItem } from "../model";

export function recencyTimestamp(item: MediaPresentationItem): number {
  return Date.parse(item.metadata.lastPlayed || "") || 0;
}

export function qualityWeight(item: MediaPresentationItem): number {
  return ({ a: 24, b: 12, c: 4, d: 0 } as const)[item.mediaQuality];
}

export function hasEnabledAction(item: MediaPresentationItem, id: string): boolean {
  return item.actions.some((action) => action.id === id && action.enabled);
}

export function compareContinueCandidates(a: MediaPresentationItem, b: MediaPresentationItem): number {
  return recencyTimestamp(b) - recencyTimestamp(a)
    || (b.metadata.totalSeconds ?? 0) - (a.metadata.totalSeconds ?? 0)
    || a.id.localeCompare(b.id);
}

export function featuredScore(item: MediaPresentationItem): number {
  return (item.favorite ? 40 : 0)
    + (item.installed && hasEnabledAction(item, "launch") ? 25 : 0)
    + qualityWeight(item)
    + (item.hero ? 10 : 0)
    + (item.screenshots.length >= 2 ? 8 : 0)
    + (recencyTimestamp(item) === 0 ? 6 : 0);
}

export function compareFeaturedCandidates(a: MediaPresentationItem, b: MediaPresentationItem): number {
  return featuredScore(b) - featuredScore(a)
    || recencyTimestamp(b) - recencyTimestamp(a)
    || a.id.localeCompare(b.id);
}


export function presentationIdentity(item: MediaPresentationItem): string {
  return (item.originalTitle || item.title)
    .normalize("NFKC")
    .trim()
    .replace(/\s+/g, " ")
    .toLocaleLowerCase("zh-CN");
}

function presentationRichness(item: MediaPresentationItem): number {
  return qualityWeight(item) * 100
    + item.media.length * 8
    + (item.installed ? 20 : 0)
    + (item.favorite ? 12 : 0)
    + (recencyTimestamp(item) > 0 ? 6 : 0);
}

/** Collapses duplicate library records for the same software into the richest presentation. */
export function dedupePresentationItems(items: readonly MediaPresentationItem[]): MediaPresentationItem[] {
  const byIdentity = new Map<string, MediaPresentationItem>();
  for (const item of items) {
    const identity = presentationIdentity(item) || `id:${item.id}`;
    const current = byIdentity.get(identity);
    if (!current || presentationRichness(item) > presentationRichness(current)
      || (presentationRichness(item) === presentationRichness(current) && item.id.localeCompare(current.id) < 0)) {
      byIdentity.set(identity, item);
    }
  }
  return [...byIdentity.values()];
}

export function selectDefaultItem(items: readonly MediaPresentationItem[]): MediaPresentationItem | null {
  return [...items].sort((a, b) =>
    compareContinueCandidates(a, b)
    || Number(b.favorite) - Number(a.favorite)
    || a.id.localeCompare(b.id)
  )[0] ?? null;
}
