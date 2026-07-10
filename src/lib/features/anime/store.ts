import type {
  AnimeDetail,
  AnimeEpisode,
  AnimeEpisodeIdentity,
  AnimeProviderApi,
  AnimeProviderConfigureRequest,
  AnimeProviderDescriptor,
  AnimeProviderFallbackOpenResponse,
  AnimeProviderHealth,
  AnimeResolveResponse,
  AnimeSearchItem,
  ProviderError,
} from "./contracts";
import { createRequestGate } from "./request-gate";

export interface AnimeProviderFeatureState {
  query: string;
  searchGeneration: number;
  isSearching: boolean;
  isConfiguring: boolean;
  isLoadingProviders: boolean;
  isRemovingProvider: boolean;
  isLoadingDetail: boolean;
  isLoadingEpisodes: boolean;
  isResolving: boolean;
  isOpeningFallback: boolean;
  providers: AnimeProviderDescriptor[];
  selectedProviderId: string | null;
  searchItems: AnimeSearchItem[];
  searchFailures: ProviderError[];
  providerHealth: AnimeProviderHealth[];
  selectedDetail: AnimeDetail | null;
  episodes: AnimeEpisode[];
  resolution: AnimeResolveResponse | null;
  error: ProviderError | null;
}

export interface AnimeProviderFeatureStore {
  getSnapshot(): AnimeProviderFeatureState;
  subscribe(listener: (state: AnimeProviderFeatureState) => void): () => void;
  configure(request: AnimeProviderConfigureRequest): Promise<AnimeProviderDescriptor | null>;
  refreshProviders(): Promise<void>;
  selectProvider(providerId: string | null): void;
  removeProvider(providerId: string): Promise<void>;
  refreshHealth(): Promise<void>;
  search(query: string, limit?: number): Promise<void>;
  loadDetail(providerId: string, itemId: string): Promise<void>;
  loadEpisodes(providerId: string, seriesId: string): Promise<void>;
  resolve(episode: AnimeEpisodeIdentity): Promise<AnimeResolveResponse | null>;
  openFallback(episode: AnimeEpisodeIdentity): Promise<AnimeProviderFallbackOpenResponse | null>;
  cancelSearch(): void;
  cancelContent(): void;
  clearError(): void;
}

const initialState = (): AnimeProviderFeatureState => ({
  query: "",
  searchGeneration: 0,
  isSearching: false,
  isConfiguring: false,
  isLoadingProviders: false,
  isRemovingProvider: false,
  isLoadingDetail: false,
  isLoadingEpisodes: false,
  isResolving: false,
  isOpeningFallback: false,
  providers: [],
  selectedProviderId: null,
  searchItems: [],
  searchFailures: [],
  providerHealth: [],
  selectedDetail: null,
  episodes: [],
  resolution: null,
  error: null,
});

/**
 * Isolated state holder for the Tauri Anime Provider boundary. Provider
 * switching is metadata-only; source credentials never enter feature state.
 */
export function createAnimeProviderFeatureStore(api: AnimeProviderApi): AnimeProviderFeatureStore {
  let state = initialState();
  const listeners = new Set<(snapshot: AnimeProviderFeatureState) => void>();
  const configureGate = createRequestGate();
  const providersGate = createRequestGate();
  const searchGate = createRequestGate();
  const detailGate = createRequestGate();
  const episodesGate = createRequestGate();
  const resolveGate = createRequestGate();
  const fallbackGate = createRequestGate();

  const snapshot = (): AnimeProviderFeatureState => ({
    ...state,
    providers: state.providers.map(cloneProviderDescriptor),
    searchItems: state.searchItems.map((item) => ({ ...item })),
    searchFailures: state.searchFailures.map((failure) => ({ ...failure })),
    providerHealth: state.providerHealth.map((health) => ({ ...health })),
    selectedDetail: state.selectedDetail ? { ...state.selectedDetail, genres: [...state.selectedDetail.genres] } : null,
    episodes: state.episodes.map((episode) => ({ ...episode, identity: { ...episode.identity } })),
    resolution: state.resolution ? cloneSafeResolution(state.resolution) : null,
    error: state.error ? { ...state.error } : null,
  });
  const publish = () => {
    const next = snapshot();
    listeners.forEach((listener) => listener(next));
  };
  const patch = (next: Partial<AnimeProviderFeatureState>) => {
    state = { ...state, ...next };
    publish();
  };
  const cancelContentRequests = () => {
    detailGate.cancel();
    episodesGate.cancel();
    resolveGate.cancel();
    fallbackGate.cancel();
  };

  const toProviderError = (error: unknown, operation: string): ProviderError => {
    if (isProviderError(error)) return { ...error };
    if (error instanceof DOMException && error.name === "AbortError") {
      return {
        kind: "cancelled",
        message: "request cancelled",
        retryable: false,
        retryAfterMs: null,
        providerId: null,
        operation,
      };
    }
    return {
      kind: "unknown",
      message: error instanceof Error ? error.message : "provider request failed",
      retryable: false,
      retryAfterMs: null,
      providerId: null,
      operation,
    };
  };

  return {
    getSnapshot: snapshot,
    subscribe(listener) {
      listeners.add(listener);
      listener(snapshot());
      return () => listeners.delete(listener);
    },
    async configure(request) {
      const lease = configureGate.begin();
      patch({ isConfiguring: true, error: null });
      try {
        const configured = await api.configure(request, lease.signal);
        if (!configureGate.isCurrent(lease.generation)) return null;
        const providers = replaceProvider(state.providers, configured);
        cancelContentRequests();
        patch({
          isConfiguring: false,
          providers,
          selectedProviderId: configured.id,
          searchItems: [],
          searchFailures: [],
          selectedDetail: null,
          episodes: [],
          resolution: null,
        });
        return cloneProviderDescriptor(configured);
      } catch (error) {
        if (configureGate.isCurrent(lease.generation)) {
          patch({ isConfiguring: false, error: toProviderError(error, "configure") });
        }
        return null;
      }
    },
    async refreshProviders() {
      const lease = providersGate.begin();
      patch({ isLoadingProviders: true, error: null });
      try {
        const providers = await api.list(lease.signal);
        if (!providersGate.isCurrent(lease.generation)) return;
        const selectedProviderId = providers.some((provider) => provider.id === state.selectedProviderId)
          ? state.selectedProviderId
          : providers[0]?.id ?? null;
        patch({ providers, selectedProviderId, isLoadingProviders: false });
      } catch (error) {
        if (providersGate.isCurrent(lease.generation)) {
          patch({ isLoadingProviders: false, error: toProviderError(error, "list") });
        }
      }
    },
    selectProvider(providerId) {
      if (providerId !== null && !state.providers.some((provider) => provider.id === providerId)) {
        patch({ error: toProviderError(new Error("provider is not configured"), "select_provider") });
        return;
      }
      searchGate.cancel();
      cancelContentRequests();
      patch({
        selectedProviderId: providerId,
        searchGeneration: searchGate.currentGeneration(),
        isSearching: false,
        isLoadingDetail: false,
        isLoadingEpisodes: false,
        isResolving: false,
        isOpeningFallback: false,
        searchItems: [],
        searchFailures: [],
        selectedDetail: null,
        episodes: [],
        resolution: null,
        error: null,
      });
    },
    async removeProvider(providerId) {
      patch({ isRemovingProvider: true, error: null });
      try {
        const removed = await api.remove(providerId);
        if (!removed) {
          patch({ isRemovingProvider: false });
          return;
        }
        const providers = state.providers.filter((provider) => provider.id !== providerId);
        const selectedProviderId = state.selectedProviderId === providerId
          ? providers[0]?.id ?? null
          : state.selectedProviderId;
        searchGate.cancel();
        cancelContentRequests();
        patch({
          providers,
          selectedProviderId,
          isRemovingProvider: false,
          isSearching: false,
          searchGeneration: searchGate.currentGeneration(),
          searchItems: [],
          searchFailures: [],
          selectedDetail: null,
          episodes: [],
          resolution: null,
        });
      } catch (error) {
        patch({ isRemovingProvider: false, error: toProviderError(error, "remove") });
      }
    },
    async refreshHealth() {
      try {
        patch({ providerHealth: await api.health() });
      } catch (error) {
        patch({ error: toProviderError(error, "health") });
      }
    },
    async search(query, limit = 50) {
      const lease = searchGate.begin();
      cancelContentRequests();
      patch({
        query,
        searchGeneration: lease.generation,
        isSearching: true,
        isLoadingDetail: false,
        isLoadingEpisodes: false,
        isResolving: false,
        error: null,
        selectedDetail: null,
        episodes: [],
        resolution: null,
        searchFailures: [],
      });
      try {
        const response = await api.search({ query, limit }, lease.signal, state.selectedProviderId);
        if (!searchGate.isCurrent(lease.generation)) return;
        patch({
          isSearching: false,
          searchItems: response.items,
          searchFailures: response.failures,
          providerHealth: response.providerHealth,
        });
      } catch (error) {
        if (!searchGate.isCurrent(lease.generation)) return;
        patch({ isSearching: false, error: toProviderError(error, "search") });
      }
    },
    async loadDetail(providerId, itemId) {
      const lease = detailGate.begin();
      resolveGate.cancel();
      fallbackGate.cancel();
      patch({
        isLoadingDetail: true,
        isResolving: false,
        isOpeningFallback: false,
        selectedDetail: null,
        resolution: null,
        error: null,
      });
      try {
        const detail = await api.detail(providerId, itemId, lease.signal);
        if (detailGate.isCurrent(lease.generation)) {
          patch({ selectedDetail: detail, isLoadingDetail: false });
        }
      } catch (error) {
        if (detailGate.isCurrent(lease.generation)) {
          patch({ isLoadingDetail: false, error: toProviderError(error, "detail") });
        }
      }
    },
    async loadEpisodes(providerId, seriesId) {
      const lease = episodesGate.begin();
      resolveGate.cancel();
      fallbackGate.cancel();
      patch({
        isLoadingEpisodes: true,
        isResolving: false,
        isOpeningFallback: false,
        episodes: [],
        resolution: null,
        error: null,
      });
      try {
        const episodes = await api.episodes(providerId, seriesId, lease.signal);
        if (episodesGate.isCurrent(lease.generation)) {
          patch({ episodes, isLoadingEpisodes: false });
        }
      } catch (error) {
        if (episodesGate.isCurrent(lease.generation)) {
          patch({ isLoadingEpisodes: false, error: toProviderError(error, "episodes") });
        }
      }
    },
    async resolve(episode) {
      const lease = resolveGate.begin();
      patch({ isResolving: true, resolution: null, error: null });
      try {
        const resolution = cloneSafeResolution(await api.resolve(episode, lease.signal));
        if (!resolveGate.isCurrent(lease.generation)) return null;
        patch({ isResolving: false, resolution });
        return cloneSafeResolution(resolution);
      } catch (error) {
        if (resolveGate.isCurrent(lease.generation)) {
          patch({ isResolving: false, error: toProviderError(error, "resolve") });
        }
        return null;
      }
    },
    async openFallback(episode) {
      const lease = fallbackGate.begin();
      patch({ isOpeningFallback: true, error: null });
      try {
        const response = await api.openFallback(episode, lease.signal);
        if (!fallbackGate.isCurrent(lease.generation)) return null;
        patch({ isOpeningFallback: false });
        return response;
      } catch (error) {
        if (fallbackGate.isCurrent(lease.generation)) {
          patch({ isOpeningFallback: false, error: toProviderError(error, "open_fallback") });
        }
        return null;
      }
    },
    cancelSearch() {
      searchGate.cancel();
      patch({ searchGeneration: searchGate.currentGeneration(), isSearching: false });
    },
    cancelContent() {
      cancelContentRequests();
      patch({
        isLoadingDetail: false,
        isLoadingEpisodes: false,
        isResolving: false,
        isOpeningFallback: false,
      });
    },
    clearError() {
      patch({ error: null });
    },
  };
}

function replaceProvider(
  providers: AnimeProviderDescriptor[],
  configured: AnimeProviderDescriptor,
): AnimeProviderDescriptor[] {
  const others = providers.filter((provider) => provider.id !== configured.id);
  return [...others, configured].sort((left, right) => left.id.localeCompare(right.id));
}

function cloneProviderDescriptor(provider: AnimeProviderDescriptor): AnimeProviderDescriptor {
  return {
    ...provider,
    allowedPaths: provider.allowedPaths ? [...provider.allowedPaths] : null,
    manifest: {
      ...provider.manifest,
      resourceKinds: [...provider.manifest.resourceKinds],
      capabilities: [...provider.manifest.capabilities],
      allowedHosts: [...provider.manifest.allowedHosts],
    },
  };
}

function cloneSafeResolution(resolution: AnimeResolveResponse): AnimeResolveResponse {
  const target = resolution.target.mode === "native_hls"
    ? { mode: "native_hls" as const, url: resolution.target.url, headers: [] }
    : resolution.target.mode === "webview"
      ? { ...resolution.target, allowedHosts: [...resolution.target.allowedHosts] }
      : { ...resolution.target };
  return { episode: { ...resolution.episode }, target };
}

function isProviderError(value: unknown): value is ProviderError {
  return typeof value === "object"
    && value !== null
    && "kind" in value
    && "message" in value
    && "retryable" in value;
}
