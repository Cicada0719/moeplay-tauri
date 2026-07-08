import type { MediaType } from "./sourceRegistry";
import { MEDIA_TYPE_LABELS } from "./sourceRegistry";
import {
  filterExtensionSourceCandidates,
  summarizeExtensionCandidateStatuses,
  type ExtensionCandidateFilters,
  type ExtensionCandidateStatus,
  type ExtensionSourceCandidate,
} from "./extensionIndex";

export const EXTENSION_CANDIDATE_STATUS_LABELS: Record<ExtensionCandidateStatus, string> = {
  discoverable: "已发现",
  requiresRuntime: "需运行时",
  nativeAdapterPlanned: "可原生适配",
  unsupported: "暂不支持",
};

export interface ExtensionCandidateCatalogRow {
  id: string;
  sourceName: string;
  extensionName: string;
  repositoryName: string;
  mediaType: MediaType;
  mediaTypeLabel: string;
  language: string;
  status: ExtensionCandidateStatus;
  statusLabel: string;
  statusReason: string;
  baseUrl: string;
  badges: string[];
}

export interface ExtensionCandidateFilterOptions {
  mediaTypes: MediaType[];
  languages: string[];
  statuses: ExtensionCandidateStatus[];
  repositories: string[];
}

export interface ExtensionCandidateCatalogSummary {
  total: number;
  visible: number;
  repositories: number;
  languages: number;
  nsfw: number;
  cloudflare: number;
  withBaseUrl: number;
  byStatus: Record<ExtensionCandidateStatus, number>;
}

export interface ExtensionCandidateCatalog {
  rows: ExtensionCandidateCatalogRow[];
  summary: ExtensionCandidateCatalogSummary;
  filterOptions: ExtensionCandidateFilterOptions;
}

const STATUS_PRIORITY: Record<ExtensionCandidateStatus, number> = {
  nativeAdapterPlanned: 0,
  discoverable: 1,
  requiresRuntime: 2,
  unsupported: 3,
};

function uniqueSorted<T extends string>(values: T[]): T[] {
  return Array.from(new Set(values)).sort((a, b) => a.localeCompare(b));
}

function compareLanguage(a: string, b: string): number {
  const preferredOrder = ["zh", "zh-Hans", "zh-Hant", "all", "en"];
  const aIndex = preferredOrder.indexOf(a);
  const bIndex = preferredOrder.indexOf(b);

  if (aIndex !== -1 || bIndex !== -1) {
    return (aIndex === -1 ? preferredOrder.length : aIndex) - (bIndex === -1 ? preferredOrder.length : bIndex);
  }

  return a.localeCompare(b);
}

function toCatalogRow(candidate: ExtensionSourceCandidate): ExtensionCandidateCatalogRow {
  const badges = [candidate.language];

  if (candidate.baseUrl) {
    badges.push("baseUrl");
  }
  if (candidate.hasCloudflare) {
    badges.push("Cloudflare");
  }
  if (candidate.nsfw) {
    badges.push("NSFW");
  }

  return {
    id: candidate.id,
    sourceName: candidate.sourceName,
    extensionName: candidate.extensionName,
    repositoryName: candidate.repositoryName,
    mediaType: candidate.mediaType,
    mediaTypeLabel: MEDIA_TYPE_LABELS[candidate.mediaType],
    language: candidate.language,
    status: candidate.status,
    statusLabel: EXTENSION_CANDIDATE_STATUS_LABELS[candidate.status],
    statusReason: candidate.statusReason,
    baseUrl: candidate.baseUrl,
    badges,
  };
}

function sortCatalogRows(a: ExtensionCandidateCatalogRow, b: ExtensionCandidateCatalogRow): number {
  return (
    STATUS_PRIORITY[a.status] - STATUS_PRIORITY[b.status] ||
    a.mediaType.localeCompare(b.mediaType) ||
    compareLanguage(a.language, b.language) ||
    a.repositoryName.localeCompare(b.repositoryName) ||
    a.sourceName.localeCompare(b.sourceName)
  );
}

export function buildExtensionCandidateCatalog(
  candidates: ExtensionSourceCandidate[],
  filters: ExtensionCandidateFilters = {},
): ExtensionCandidateCatalog {
  const visibleCandidates = filterExtensionSourceCandidates(candidates, filters);
  const rows = visibleCandidates.map(toCatalogRow).sort(sortCatalogRows);

  return {
    rows,
    summary: {
      total: candidates.length,
      visible: visibleCandidates.length,
      repositories: new Set(candidates.map((candidate) => candidate.repositoryId)).size,
      languages: new Set(candidates.map((candidate) => candidate.language)).size,
      nsfw: candidates.filter((candidate) => candidate.nsfw).length,
      cloudflare: candidates.filter((candidate) => candidate.hasCloudflare).length,
      withBaseUrl: candidates.filter((candidate) => candidate.baseUrl.length > 0).length,
      byStatus: summarizeExtensionCandidateStatuses(candidates),
    },
    filterOptions: {
      mediaTypes: uniqueSorted(candidates.map((candidate) => candidate.mediaType)),
      languages: uniqueSorted(candidates.map((candidate) => candidate.language)),
      statuses: uniqueSorted(candidates.map((candidate) => candidate.status)),
      repositories: uniqueSorted(candidates.map((candidate) => candidate.repositoryName)),
    },
  };
}
