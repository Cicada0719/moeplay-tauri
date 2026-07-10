import { comicProviderApi } from "./api";
import { computePrefetchWindow, planPageRetry, RequestGeneration } from "./logic";
import type {
  ComicChapter,
  ComicFeatureState,
  ComicProviderApi,
  ComicProviderConfigureRequest,
  ComicProviderDescriptor,
  ComicProviderProbe,
  ComicResolvedTarget,
  ComicSearchRequest,
  ComicSeries,
  ComicSeriesDetail,
} from "./types";

function initialState(): ComicFeatureState {
  return {
    generation: 0,
    providers: [],
    loading: false,
    series: [],
    detailsBySeries: {},
    chaptersBySeries: {},
    targetsByChapter: {},
    probesByProvider: {},
  };
}

export function createComicProviderStore(api: ComicProviderApi = comicProviderApi) {
  const generation = new RequestGeneration();
  let state = initialState();

  function snapshot(): ComicFeatureState {
    return {
      ...state,
      providers: [...state.providers],
      series: [...state.series],
      detailsBySeries: { ...state.detailsBySeries },
      chaptersBySeries: { ...state.chaptersBySeries },
      targetsByChapter: { ...state.targetsByChapter },
      probesByProvider: { ...state.probesByProvider },
    };
  }

  function begin(providerId = state.providerId): number {
    const current = generation.bump();
    state = { ...state, generation: current, providerId, loading: true, error: undefined };
    return current;
  }

  function fail(token: number, error: unknown): void {
    if (generation.isCurrent(token)) state = { ...state, loading: false, error };
  }

  function cancelPending(): ComicFeatureState {
    const current = generation.bump();
    state = { ...state, generation: current, loading: false };
    return snapshot();
  }

  function selectProvider(providerId: string | undefined): ComicFeatureState {
    const current = generation.bump();
    state = {
      ...state,
      generation: current,
      providerId,
      loading: false,
      error: undefined,
      series: [],
      detailsBySeries: {},
      chaptersBySeries: {},
      targetsByChapter: {},
    };
    return snapshot();
  }

  async function refreshProviders(): Promise<ComicProviderDescriptor[]> {
    const token = begin();
    try {
      const providers = await api.list();
      if (!generation.isCurrent(token)) return providers;
      const selectedStillExists = state.providerId
        ? providers.some((provider) => provider.id === state.providerId)
        : false;
      state = {
        ...state,
        loading: false,
        providers,
        providerId: selectedStillExists ? state.providerId : providers[0]?.id,
      };
      return providers;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function configureProvider(request: ComicProviderConfigureRequest): Promise<ComicProviderDescriptor> {
    const token = begin();
    try {
      const provider = await api.configure(request);
      const providers = await api.list();
      if (generation.isCurrent(token)) {
        state = {
          ...state,
          loading: false,
          providerId: provider.id,
          providers,
          series: [],
          detailsBySeries: {},
          chaptersBySeries: {},
          targetsByChapter: {},
        };
      }
      return provider;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function removeProvider(providerId: string): Promise<boolean> {
    const token = begin(state.providerId);
    try {
      const removed = await api.remove(providerId);
      if (!generation.isCurrent(token)) return removed;
      if (!removed) { state = { ...state, loading: false }; return false; }
      const providers = state.providers.filter((provider) => provider.id !== providerId);
      const wasSelected = state.providerId === providerId;
      state = {
        ...state,
        loading: false,
        providers,
        providerId: wasSelected ? providers[0]?.id : state.providerId,
        series: wasSelected ? [] : state.series,
        detailsBySeries: wasSelected ? {} : state.detailsBySeries,
        chaptersBySeries: wasSelected ? {} : state.chaptersBySeries,
        targetsByChapter: wasSelected ? {} : state.targetsByChapter,
      };
      return true;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function probe(providerId: string): Promise<ComicProviderProbe | undefined> {
    const token = begin(state.providerId);
    try {
      const result = await api.probe(providerId);
      if (!generation.isCurrent(token)) return undefined;
      state = {
        ...state,
        loading: false,
        probesByProvider: { ...state.probesByProvider, [providerId]: result },
      };
      return result;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function search(providerId: string, request: ComicSearchRequest): Promise<ComicSeries[] | undefined> {
    const token = begin(providerId);
    try {
      const result = await api.search(providerId, { page: 1, pageSize: 50, ...request });
      if (!generation.isCurrent(token)) return undefined;
      state = {
        ...state,
        loading: false,
        series: result,
        detailsBySeries: {},
        chaptersBySeries: {},
        targetsByChapter: {},
      };
      return result;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function searchSelected(request: ComicSearchRequest): Promise<ComicSeries[] | undefined> {
    if (!state.providerId) throw new Error("No comic provider selected");
    return search(state.providerId, request);
  }

  async function loadSeries(providerId: string, seriesId: string): Promise<{ detail: ComicSeriesDetail; chapters: ComicChapter[] } | undefined> {
    const token = begin(providerId);
    try {
      const [detail, chapters] = await Promise.all([
        api.detail(providerId, seriesId),
        api.chapters(providerId, seriesId),
      ]);
      if (!generation.isCurrent(token)) return undefined;
      state = {
        ...state,
        loading: false,
        detailsBySeries: { ...state.detailsBySeries, [seriesId]: detail },
        chaptersBySeries: { ...state.chaptersBySeries, [seriesId]: chapters },
      };
      return { detail, chapters };
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function loadDetail(providerId: string, seriesId: string): Promise<ComicSeriesDetail | undefined> {
    const token = begin(providerId);
    try {
      const result = await api.detail(providerId, seriesId);
      if (!generation.isCurrent(token)) return undefined;
      state = {
        ...state,
        loading: false,
        detailsBySeries: { ...state.detailsBySeries, [seriesId]: result },
      };
      return result;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function loadChapters(providerId: string, seriesId: string): Promise<ComicChapter[] | undefined> {
    const token = begin(providerId);
    try {
      const result = await api.chapters(providerId, seriesId);
      if (!generation.isCurrent(token)) return undefined;
      state = {
        ...state,
        loading: false,
        chaptersBySeries: { ...state.chaptersBySeries, [seriesId]: result },
      };
      return result;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  async function resolve(providerId: string, seriesId: string, chapterId: string): Promise<ComicResolvedTarget | undefined> {
    const token = begin(providerId);
    try {
      const result = await api.resolve(providerId, seriesId, chapterId);
      if (!generation.isCurrent(token)) return undefined;
      state = {
        ...state,
        loading: false,
        targetsByChapter: { ...state.targetsByChapter, [chapterId]: result },
      };
      return result;
    } catch (error) {
      fail(token, error);
      throw error;
    }
  }

  function clearTarget(chapterId?: string): ComicFeatureState {
    if (!chapterId) {
      state = { ...state, targetsByChapter: {} };
      return snapshot();
    }
    const targetsByChapter = { ...state.targetsByChapter };
    delete targetsByChapter[chapterId];
    state = { ...state, targetsByChapter };
    return snapshot();
  }

  return {
    get state(): ComicFeatureState { return snapshot(); },
    cancelPending,
    selectProvider,
    refreshProviders,
    configureProvider,
    removeProvider,
    probe,
    search,
    searchSelected,
    loadSeries,
    loadDetail,
    loadChapters,
    resolve,
    clearTarget,
    planPageRetry,
    prefetchWindow: computePrefetchWindow,
  };
}



