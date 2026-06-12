import type { Game, GamePlatform } from "../api";

export type GameLike = Partial<
  Pick<
    Game,
    | "background"
    | "cover"
    | "description"
    | "developer"
    | "engine"
    | "genres"
    | "game_type"
    | "icon"
    | "last_played"
    | "platform"
    | "play_time_seconds"
    | "publisher"
    | "rating"
    | "release_year"
    | "screenshots"
    | "tags"
    | "vndb_id"
    | "bangumi_id"
  >
> & {
  aliases?: Partial<Game["aliases"][number]>[] | null;
  play_tracker?: Partial<Game["play_tracker"]> | null;
  metadata?: Partial<Game["metadata"]> | null;
};

export type NormalizedGame<T extends GameLike> = T & {
  metadata: Partial<Game["metadata"]>;
  play_tracker: Partial<Game["play_tracker"]>;
};

export function normalizeCompletionStatus(status: unknown): string {
  if (typeof status !== "string" || !status.trim()) return "not_started";
  return status
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[\s-]+/g, "_")
    .toLowerCase();
}

function canonicalPlatform(value: unknown, fallback: unknown): GamePlatform | undefined {
  const raw = String(value ?? fallback ?? "").trim().toLowerCase();
  if (!raw) return undefined;
  if (raw === "pc" || raw === "web" || raw === "mobile" || raw === "console" || raw === "other") return raw;
  return "pc";
}

export function normalizeGame<T extends GameLike>(game: T): NormalizedGame<T> {
  const metadata = game.metadata ?? {};
  const playTracker = game.play_tracker ?? {};
  const normalized = {
    ...game,
    metadata: {
      ...metadata,
      genres: metadata.genres?.length ? metadata.genres : (game.genres ?? []),
      languages: metadata.languages ?? [],
      voice_languages: metadata.voice_languages ?? [],
      stores: metadata.stores ?? [],
      developer: metadata.developer ?? game.developer,
      publisher: metadata.publisher ?? game.publisher,
      platform: metadata.platform ?? canonicalPlatform(game.platform, game.game_type),
      engine: metadata.engine ?? game.engine,
      release_year: metadata.release_year ?? game.release_year,
      vndb_rating: metadata.vndb_rating ?? game.rating,
      vndb_id: metadata.vndb_id ?? game.vndb_id,
      bangumi_id: metadata.bangumi_id ?? game.bangumi_id,
      cover: metadata.cover ?? game.cover ?? game.icon,
      background: metadata.background ?? game.background,
    },
    play_tracker: {
      ...playTracker,
      total_seconds: playTracker.total_seconds ?? game.play_time_seconds ?? 0,
      sessions: playTracker.sessions ?? [],
      completion_status: normalizeCompletionStatus(playTracker.completion_status),
      last_played: playTracker.last_played ?? game.last_played,
      user_rating: playTracker.user_rating ?? game.rating,
      achievements_total: playTracker.achievements_total ?? 0,
      achievements_unlocked: playTracker.achievements_unlocked ?? 0,
      finished: playTracker.finished ?? false,
      completion_count: playTracker.completion_count ?? 0,
    },
  };
  return normalized as NormalizedGame<T>;
}

export function gameCompletionStatus(game: GameLike | null | undefined): string {
  return normalizeCompletionStatus(game?.play_tracker?.completion_status);
}

export function gameTotalSeconds(game: GameLike | null | undefined): number {
  return game?.play_tracker?.total_seconds ?? game?.play_time_seconds ?? 0;
}

export function gameLastPlayed(game: GameLike | null | undefined): string | null {
  return game?.play_tracker?.last_played ?? game?.last_played ?? null;
}

export function gameRating(game: GameLike | null | undefined): number {
  return (
    game?.play_tracker?.user_rating ??
    game?.metadata?.vndb_rating ??
    game?.metadata?.bangumi_rating ??
    game?.rating ??
    0
  );
}

export function coverOf(game: GameLike | null | undefined): string | null {
  return game?.metadata?.cover ?? game?.cover ?? game?.icon ?? null;
}

export function heroImageOf(game: GameLike | null | undefined): string | null {
  return (
    game?.metadata?.background ??
    game?.background ??
    game?.screenshots?.[0] ??
    coverOf(game)
  );
}

export function hasHeroBackground(game: GameLike | null | undefined): boolean {
  return !!(game?.metadata?.background ?? game?.background ?? game?.screenshots?.[0]);
}

export function screenshotsOf(game: GameLike | null | undefined): string[] {
  const shots = (game?.screenshots ?? []).filter(Boolean);
  if (shots.length > 0) return shots;
  return [heroImageOf(game), coverOf(game)].filter((src): src is string => Boolean(src));
}

export function developerOf(game: GameLike | null | undefined): string {
  return (
    game?.metadata?.developer ??
    game?.developer ??
    game?.metadata?.publisher ??
    game?.publisher ??
    "未知社团"
  );
}

export function publisherOf(game: GameLike | null | undefined): string {
  return game?.metadata?.publisher ?? game?.publisher ?? "未记录";
}

export function originalNameOf(game: GameLike | null | undefined): string | null {
  return (
    game?.metadata?.original_name ??
    game?.aliases?.find((a) => a.language === "ja" || a.language === "jp")?.name ??
    game?.aliases?.find((a) => a.is_primary)?.name ??
    null
  );
}

export function releaseYearOf(game: GameLike | null | undefined): number | null {
  return game?.metadata?.release_year ?? game?.release_year ?? null;
}

export function platformOf(game: GameLike | null | undefined): string {
  return game?.metadata?.platform ?? game?.platform ?? game?.game_type ?? "Windows";
}

export function tagsOf(game: GameLike | null | undefined): string[] {
  const tags = new Set<string>();
  for (const genre of game?.metadata?.genres ?? game?.genres ?? []) if (genre) tags.add(genre);
  for (const tag of game?.tags ?? []) if (tag) tags.add(tag);
  return [...tags];
}

export function hasMissingCoreMetadata(game: GameLike | null | undefined): boolean {
  return !(
    (game?.vndb_id || game?.metadata?.vndb_id) &&
    (game?.bangumi_id || game?.metadata?.bangumi_id) &&
    coverOf(game) &&
    game?.description &&
    tagsOf(game).length > 0
  );
}

export function userFacingErrorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    const message = String((error as { message?: unknown }).message);
    const name = String((error as { name?: unknown }).name ?? "");
    if (
      message &&
      name !== "TypeError" &&
      !message.startsWith("TypeError:") &&
      !message.startsWith("Cannot read properties")
    ) {
      return message;
    }
  }
  if (typeof error === "string" && error && !error.startsWith("TypeError:")) {
    return error;
  }
  return "操作失败，请稍后重试。";
}
