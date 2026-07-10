import type { PlatformGameCandidate, SteamSessionGame } from "../../api";
import type { ImportSourceRecord, PreviewImportRequest } from "../../features/library";

function meaningful(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function launchPathFromUri(uri: string | null): string | null {
  if (!uri) return null;
  if (/^[a-zA-Z]:[\\/]/.test(uri) || uri.startsWith("\\\\")) return uri;
  if (!uri.toLowerCase().startsWith("file://")) return null;
  try {
    const parsed = new URL(uri);
    const decoded = decodeURIComponent(parsed.pathname);
    return /^\/[a-zA-Z]:/.test(decoded) ? decoded.slice(1) : decoded;
  } catch {
    return null;
  }
}

function platformLabel(source: string): string {
  if (source.toLowerCase() === "steam") return "Steam";
  if (source.toLowerCase() === "epic") return "Epic";
  return source;
}

export function platformCandidateToImportRecord(candidate: PlatformGameCandidate): ImportSourceRecord {
  const source = String(candidate.source || "platform").trim().toLowerCase();
  const launchUri = meaningful(candidate.launch_uri);
  const fields: Record<string, unknown> = {
    game_type: platformLabel(source),
  };
  const cover = meaningful(candidate.cover_url);
  const icon = meaningful(candidate.icon_url);
  const storeUrl = meaningful(candidate.store_url);
  if (cover) fields["metadata.cover"] = cover;
  if (icon) fields.icon = icon;
  if (storeUrl) fields["metadata.homepage"] = storeUrl;

  return {
    sourceRecordId: `${source}:${candidate.library_id}`,
    title: candidate.name.trim(),
    launchPath: launchPathFromUri(launchUri),
    installDir: meaningful(candidate.install_dir),
    platformId: candidate.library_id ? { source, id: String(candidate.library_id) } : null,
    launchUri,
    fields,
  };
}

export function platformCandidatesToPreviewRequest(
  source: string,
  candidates: PlatformGameCandidate[],
): PreviewImportRequest {
  return {
    source,
    records: candidates.map(platformCandidateToImportRecord),
  };
}

export function steamSessionGamesToPreviewRequest(games: SteamSessionGame[]): PreviewImportRequest {
  return {
    source: "steam_session",
    records: games.map((game) => {
      const id = String(game.appid);
      return {
        sourceRecordId: `steam:${id}`,
        title: game.name.trim(),
        launchPath: null,
        installDir: null,
        platformId: { source: "steam", id },
        launchUri: `steam://rungameid/${id}`,
        fields: {
          game_type: "Steam",
          "metadata.cover": `https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/${id}/library_600x900_2x.jpg`,
        },
      };
    }),
  };
}
