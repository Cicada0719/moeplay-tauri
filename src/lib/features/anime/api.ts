import { invokeCmd } from "../../api/core";
import type {
  AnimeDetail,
  AnimeEpisode,
  AnimeEpisodeIdentity,
  AnimeProviderApi,
  AnimeProviderConfigureRequest,
  AnimeProviderDescriptor,
  AnimeProviderHealth,
  AnimeProviderFallbackOpenResponse,
  AnimeLocalMediaScanResult,
  AnimeResolveResponse,
  AnimeSearchQuery,
  AnimeSearchResponse,
} from "./contracts";

/** Real Tauri bridge for the isolated Anime Provider feature. */
export function createTauriAnimeProviderApi(): AnimeProviderApi {
  return {
    configure(request, signal) {
      return withSignal(invokeCmd<AnimeProviderDescriptor>("anime_provider_configure", { request }), signal);
    },
    list(signal) {
      return withSignal(invokeCmd<AnimeProviderDescriptor[]>("anime_provider_list"), signal);
    },
    remove(providerId, signal) {
      return withSignal(invokeCmd<boolean>("anime_provider_remove", { providerId }), signal);
    },
    health(signal) {
      return withSignal(invokeCmd<AnimeProviderHealth[]>("anime_provider_health"), signal);
    },
    pickLocalDirectory(signal) {
      return withSignal(invokeCmd<AnimeLocalMediaScanResult | null>("anime_provider_pick_local_directory"), signal);
    },
    search(query, signal, providerId) {
      return withSignal(invokeCmd<AnimeSearchResponse>("anime_provider_search", { query, providerId: providerId ?? null }), signal);
    },
    detail(providerId, itemId, signal) {
      return withSignal(invokeCmd<AnimeDetail>("anime_provider_detail", { providerId, itemId }), signal);
    },
    episodes(providerId, seriesId, signal) {
      return withSignal(invokeCmd<AnimeEpisode[]>("anime_provider_episodes", { providerId, seriesId }), signal);
    },
    resolve(episode, signal) {
      return withSignal(invokeCmd<AnimeResolveResponse>("anime_provider_resolve", { request: { episode } }), signal);
    },
    openFallback(episode, signal) {
      return withSignal(invokeCmd<AnimeProviderFallbackOpenResponse>("anime_provider_open_fallback", { request: { episode } }), signal);
    },
  };
}

function withSignal<T>(invocation: Promise<T>, signal?: AbortSignal): Promise<T> {
  if (!signal) return invocation;
  if (signal.aborted) return Promise.reject(abortError());

  return new Promise<T>((resolve, reject) => {
    const onAbort = () => reject(abortError());
    signal.addEventListener("abort", onAbort, { once: true });
    invocation.then(
      (value) => {
        signal.removeEventListener("abort", onAbort);
        resolve(value);
      },
      (error) => {
        signal.removeEventListener("abort", onAbort);
        reject(error);
      },
    );
  });
}

function abortError(): DOMException {
  return new DOMException("request cancelled", "AbortError");
}

export type {
  AnimeDetail,
  AnimeEpisode,
  AnimeEpisodeIdentity,
  AnimeProviderConfigureRequest,
  AnimeProviderDescriptor,
  AnimeProviderHealth,
  AnimeProviderFallbackOpenResponse,
  AnimeLocalMediaScanResult,
  AnimeResolveResponse,
  AnimeSearchQuery,
  AnimeSearchResponse,
};