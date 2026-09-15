import { invokeCmd } from "./core";

export interface VideoUrlResult {
  url: string;
  source: string;
  tab_url: string;
  session_id?: string;
}

export interface AnimeExtractVideoUrlOptions {
  episodeUrl: string;
  referer?: string;
  useLegacyParser: boolean;
  userAgent?: string;
  sessionId?: string;
  scope?: string;
}

/** Extract a playable stream while binding the operation to an optional scope. */
export function animeExtractVideoUrl(
  options: AnimeExtractVideoUrlOptions,
): Promise<VideoUrlResult> {
  return invokeCmd<VideoUrlResult>("anime_extract_video_url", { ...options });
}

/** Cancel all extraction work currently registered under the scope. */
export function animeCancelExtract(scope: string): Promise<void> {
  return invokeCmd<void>("anime_cancel_extract", { scope });
}
