import { tauriSourceCenterApi } from "./api";
import {
  clampSourcePriority,
  emptySourceFilters,
  filterSources,
  sortSources,
  sourceKey,
  type ExtensionIndexSnapshot,
  type SourceCenterApi,
  type SourceCenterSnapshot,
  type SourceDescriptor,
  type SourceFilters,
  type SourcePreferenceUpdate,
  type SourceRef,
} from "./contracts";
import {
  clearExtensionIndexEndpoint,
  getConfiguredExtensionIndexEndpoint,
  saveExtensionIndexEndpoint,
} from "./extensionIndexEndpoint";

export interface SourceCenterStore {
  getSnapshot(): SourceCenterSnapshot;
  subscribe(listener: (snapshot: SourceCenterSnapshot) => void): () => void;
  load(): Promise<void>;
  refresh(): Promise<void>;
  setFilters(next: Partial<SourceFilters>): Promise<void>;
  setExtensionIndexEndpoint(endpoint: string): Promise<void>;
  clearExtensionIndexEndpoint(): Promise<void>;
  toggleEnabled(source: SourceDescriptor): Promise<void>;
  adjustPriority(source: SourceDescriptor, amount: -1 | 1): Promise<void>;
  verifySource(source: SourceRef): Promise<void>;
  verifyVisible(): Promise<void>;
  resetHealth(source: SourceRef): Promise<void>;
  refreshExtensionIndex(): Promise<void>;
}

type MutableState = {
  allSources: SourceDescriptor[];
  filters: SourceFilters;
  extensionIndex: ExtensionIndexSnapshot | null;
  extensionIndexEndpoint: string | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  actionKeys: Set<string>;
  lastLoadedAt: number | null;
};

function copySnapshot(snapshot: SourceCenterSnapshot): SourceCenterSnapshot {
  return {
    ...snapshot,
    sources: [...snapshot.sources],
    allSources: [...snapshot.allSources],
    filters: { ...snapshot.filters },
    actionKeys: [...snapshot.actionKeys],
    extensionIndex: snapshot.extensionIndex ? { ...snapshot.extensionIndex } : null,
  };
}

function sourceActionKey(source: SourceRef, action: string): string {
  return `${action}:${sourceKey(source)}`;
}

function describeError(reason: unknown): string {
  return reason instanceof Error && reason.message
    ? reason.message
    : "来源中心操作失败，请稍后重试。";
}

function project(state: MutableState): SourceCenterSnapshot {
  const allSources = sortSources(state.allSources);
  return {
    allSources,
    sources: filterSources(allSources, state.filters),
    filters: state.filters,
    extensionIndex: state.extensionIndex,
    extensionIndexEndpoint: state.extensionIndexEndpoint,
    loading: state.loading,
    refreshing: state.refreshing,
    error: state.error,
    actionKeys: [...state.actionKeys],
    lastLoadedAt: state.lastLoadedAt,
  };
}

/** Feature-local store: every mutation is backend-owned and rolls back on failure. */
export function createSourceCenterStore(api: SourceCenterApi = tauriSourceCenterApi): SourceCenterStore {
  let state: MutableState = {
    allSources: [],
    filters: emptySourceFilters(),
    extensionIndex: null,
    extensionIndexEndpoint: getConfiguredExtensionIndexEndpoint(),
    loading: false,
    refreshing: false,
    error: null,
    actionKeys: new Set(),
    lastLoadedAt: null,
  };
  let snapshot = project(state);
  const listeners = new Set<(snapshot: SourceCenterSnapshot) => void>();
  let requestVersion = 0;

  function commit(next: Partial<MutableState>) {
    state = { ...state, ...next };
    snapshot = project(state);
    for (const listener of listeners) listener(copySnapshot(snapshot));
  }

  function replaceSource(next: SourceDescriptor) {
    commit({ allSources: state.allSources.map((source) => sourceKey(source) === sourceKey(next) ? next : source) });
  }

  async function load(initial: boolean) {
    const version = ++requestVersion;
    const endpoint = state.extensionIndexEndpoint;
    commit({ loading: initial, refreshing: !initial, error: null });
    const [sources, index] = await Promise.allSettled([
      api.listSourceDescriptors(),
      endpoint ? api.getExtensionIndexSnapshot(endpoint) : Promise.resolve(null),
    ]);
    if (version !== requestVersion) return;
    if (sources.status === "rejected") {
      commit({ loading: false, refreshing: false, error: describeError(sources.reason) });
      return;
    }
    commit({
      allSources: sources.value,
      extensionIndex: index.status === "fulfilled" ? index.value : state.extensionIndex,
      loading: false,
      refreshing: false,
      error: index.status === "rejected" ? "来源已加载，但扩展目录快照暂不可用。" : null,
      lastLoadedAt: Date.now(),
    });
  }

  async function run(source: SourceRef, action: string, work: () => Promise<SourceDescriptor | void>) {
    const key = sourceActionKey(source, action);
    if (state.actionKeys.has(key)) return;
    const nextKeys = new Set(state.actionKeys);
    nextKeys.add(key);
    commit({ actionKeys: nextKeys, error: null });
    try {
      const result = await work();
      if (result) replaceSource(result);
      await load(false);
    } catch (reason) {
      commit({ error: describeError(reason) });
      throw reason;
    } finally {
      const remaining = new Set(state.actionKeys);
      remaining.delete(key);
      commit({ actionKeys: remaining });
    }
  }

  function preference(source: SourceDescriptor, enabled: boolean, priority: number): SourcePreferenceUpdate {
    return { providerId: source.providerId, mediaType: source.mediaType, enabled, priority: clampSourcePriority(priority) };
  }

  return {
    getSnapshot: () => copySnapshot(snapshot),
    subscribe(listener) {
      listeners.add(listener);
      listener(copySnapshot(snapshot));
      return () => listeners.delete(listener);
    },
    load: () => load(state.allSources.length === 0),
    refresh: () => load(false),
    async setFilters(next) {
      commit({ filters: { ...state.filters, ...next } });
    },
    async setExtensionIndexEndpoint(endpoint) {
      let configuredEndpoint: string;
      try {
        configuredEndpoint = saveExtensionIndexEndpoint(endpoint);
      } catch (reason) {
        commit({ error: describeError(reason) });
        throw reason;
      }
      commit({ extensionIndexEndpoint: configuredEndpoint, extensionIndex: null, error: null });
      try {
        const index = await api.getExtensionIndexSnapshot(configuredEndpoint);
        commit({ extensionIndex: index });
      } catch (reason) {
        commit({ error: "目录端点已保存，但快照暂不可用。" });
        throw reason;
      }
    },
    async clearExtensionIndexEndpoint() {
      clearExtensionIndexEndpoint();
      commit({ extensionIndexEndpoint: null, extensionIndex: null, error: null });
    },
    async toggleEnabled(source) {
      const original = state.allSources;
      const optimistic = { ...source, enabled: !source.enabled };
      replaceSource(optimistic);
      try {
        await run(source, "preference", () => api.updateSourcePreference(preference(source, optimistic.enabled, source.priority)));
      } catch {
        commit({ allSources: original });
      }
    },
    async adjustPriority(source, amount) {
      const original = state.allSources;
      const optimistic = { ...source, priority: clampSourcePriority(source.priority + amount) };
      replaceSource(optimistic);
      try {
        await run(source, "preference", () => api.updateSourcePreference(preference(source, source.enabled, optimistic.priority)));
      } catch {
        commit({ allSources: original });
      }
    },
    verifySource: (source) => run(source, "verify", async () => {
      await api.verifySource(source);
    }),
    async verifyVisible() {
      const visible = project(state).sources.filter((source) => source.enabled);
      if (visible.length === 0 || state.actionKeys.has("verify-batch")) return;
      const nextKeys = new Set(state.actionKeys);
      nextKeys.add("verify-batch");
      commit({ actionKeys: nextKeys, error: null });
      try {
        await api.verifySourcesBatch(visible.map(({ providerId, mediaType }) => ({ providerId, mediaType })));
        await load(false);
      } catch (reason) {
        commit({ error: describeError(reason) });
        throw reason;
      } finally {
        const remaining = new Set(state.actionKeys);
        remaining.delete("verify-batch");
        commit({ actionKeys: remaining });
      }
    },
    resetHealth: (source) => run(source, "reset-health", () => api.resetSourceHealth(source)),
    async refreshExtensionIndex() {
      const endpoint = state.extensionIndexEndpoint;
      if (!endpoint || state.actionKeys.has("refresh-extension-index")) return;
      const nextKeys = new Set(state.actionKeys);
      nextKeys.add("refresh-extension-index");
      commit({ actionKeys: nextKeys, error: null });
      try {
        const result = await api.refreshExtensionIndex(endpoint);
        commit({ extensionIndex: result ?? await api.getExtensionIndexSnapshot(endpoint) });
      } catch (reason) {
        commit({ error: describeError(reason) });
        throw reason;
      } finally {
        const remaining = new Set(state.actionKeys);
        remaining.delete("refresh-extension-index");
        commit({ actionKeys: remaining });
      }
    },
  };
}
