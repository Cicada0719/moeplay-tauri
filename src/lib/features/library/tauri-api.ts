import { invokeCmd } from "../../api/core";
import type {
  ApplyImportRequest,
  ApplyImportResponse,
  ImportPreview,
  LibraryApi,
  LibraryHealthSnapshot,
  PreviewImportRequest,
} from "./contracts";

function throwIfAborted(signal: AbortSignal) {
  if (signal.aborted) throw new DOMException("操作已取消", "AbortError");
}

export const tauriLibraryApi: LibraryApi = {
  async preview(request: PreviewImportRequest, signal: AbortSignal): Promise<ImportPreview> {
    throwIfAborted(signal);
    const result = await invokeCmd<ImportPreview>("library_v2_preview_import", { request });
    throwIfAborted(signal);
    return result;
  },
  async apply(request: ApplyImportRequest, signal: AbortSignal): Promise<ApplyImportResponse> {
    throwIfAborted(signal);
    const result = await invokeCmd<ApplyImportResponse>("library_v2_apply_import", { request });
    throwIfAborted(signal);
    return result;
  },
  async health(signal: AbortSignal): Promise<LibraryHealthSnapshot> {
    throwIfAborted(signal);
    const result = await invokeCmd<LibraryHealthSnapshot>("library_v2_health");
    throwIfAborted(signal);
    return result;
  },
};
