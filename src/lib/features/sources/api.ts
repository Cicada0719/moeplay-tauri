import { invokeCmd } from "../../api/core";
import {
  normalizeExtensionIndexSnapshot,
  normalizeSourceDescriptor,
  normalizeSourceDescriptors,
  type ExtensionIndexSnapshot,
  type SourceCenterApi,
  type SourceDescriptor,
  type SourcePreferenceUpdate,
  type SourceRef,
} from "./contracts";
import { normalizeExtensionIndexEndpoint } from "./extensionIndexEndpoint";

const args = (source: SourceRef) => ({ providerId: source.providerId, mediaType: source.mediaType });

function controlledEndpoint(endpoint: string | null): string | null {
  return normalizeExtensionIndexEndpoint(endpoint);
}

/** Backend command bridge. The extension directory is never contacted without an explicit, credential-free endpoint. */
export const tauriSourceCenterApi: SourceCenterApi = {
  async listSourceDescriptors(): Promise<SourceDescriptor[]> {
    return normalizeSourceDescriptors(await invokeCmd<unknown>("list_source_descriptors"));
  },
  async updateSourcePreference(input: SourcePreferenceUpdate): Promise<SourceDescriptor | void> {
    const result = await invokeCmd<unknown>("update_source_preference", { request: input });
    return result == null ? undefined : normalizeSourceDescriptor(result);
  },
  async verifySource(source: SourceRef): Promise<void> { await invokeCmd("verify_source", { source: args(source) }); },
  async verifySourcesBatch(sources: SourceRef[]): Promise<void> {
    await invokeCmd("verify_sources_batch", { sources: sources.map(args) });
  },
  async resetSourceHealth(source: SourceRef): Promise<SourceDescriptor | void> {
    const result = await invokeCmd<unknown>("reset_source_health", { source: args(source) });
    return result == null ? undefined : normalizeSourceDescriptor(result);
  },
  async refreshExtensionIndex(endpoint: string | null): Promise<ExtensionIndexSnapshot | null> {
    const configuredEndpoint = controlledEndpoint(endpoint);
    if (!configuredEndpoint) return null;
    return normalizeExtensionIndexSnapshot(await invokeCmd<unknown>("refresh_extension_index", { endpoint: configuredEndpoint, force: true }));
  },
  async getExtensionIndexSnapshot(endpoint: string | null): Promise<ExtensionIndexSnapshot | null> {
    const configuredEndpoint = controlledEndpoint(endpoint);
    if (!configuredEndpoint) return null;
    const result = await invokeCmd<unknown>("get_extension_index_snapshot", { endpoint: configuredEndpoint });
    return result == null ? null : normalizeExtensionIndexSnapshot(result);
  },
};
