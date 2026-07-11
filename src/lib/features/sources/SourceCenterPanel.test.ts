import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SourceCenterPanel from "./SourceCenterPanel.svelte";
import { createSourceCenterStore } from "./store";
import type { SourceCenterApi, SourceDescriptor } from "./contracts";

const source: SourceDescriptor = {
  providerId: "suwayomi-local", mediaType: "comic", displayName: "Suwayomi", kind: "suwayomi",
  capabilities: ["search", "children", "resolve", "verify"], enabled: true, priority: -5,
  health: { state: "degraded", latencyMs: 120, lastCheckedAt: "2026-07-11T00:00:00Z", consecutiveFailures: 1, successRate: 0.8, lastFailure: { code: "offline", message: "服务不可达" } },
  latencyMs: 120, lastCheckedAt: "2026-07-11T00:00:00Z", authState: "missing", runtimeState: "deferred", languages: ["zh"], nsfw: "exclude", recentFailures: [{ code: "offline", message: "服务不可达" }],
};

function api(): SourceCenterApi {
  return {
    listSourceDescriptors: vi.fn(async () => [source]),
    updateSourcePreference: vi.fn(async () => undefined),
    verifySource: vi.fn(async () => {}),
    verifySourcesBatch: vi.fn(async () => {}),
    resetSourceHealth: vi.fn(async () => undefined),
    refreshExtensionIndex: vi.fn(async () => ({ entries: [{ id: "x", name: "fixture" }], fetchedAt: "2026-07-11T00:00:00Z", expiresAt: null, isOfflineSnapshot: true, lastError: "offline" })),
    getExtensionIndexSnapshot: vi.fn(async () => null),
  };
}

describe("SourceCenterPanel", () => {
  beforeEach(() => localStorage.clear());

  it("keeps extension synchronization disabled until a controlled endpoint is explicitly saved", async () => {
    const sourceApi = api();
    render(SourceCenterPanel, { props: { store: createSourceCenterStore(sourceApi) } });
    await screen.findByText("Suwayomi");

    expect(screen.getByTestId("extension-index-state")).toHaveTextContent("同步已禁用");
    expect(screen.getByRole("button", { name: "同步目录" })).toBeDisabled();
    expect(sourceApi.getExtensionIndexSnapshot).not.toHaveBeenCalled();

    await fireEvent.input(screen.getByLabelText("扩展目录端点"), { target: { value: "https://directory.example/extensions.json" } });
    await fireEvent.click(screen.getByRole("button", { name: "保存端点" }));

    await waitFor(() => expect(sourceApi.getExtensionIndexSnapshot).toHaveBeenCalledWith("https://directory.example/extensions.json"));
    expect(screen.getByRole("button", { name: "同步目录" })).not.toBeDisabled();
  });

  it("renders mapped health/auth/runtime/capabilities, filters, and source actions", async () => {
    const sourceApi = api();
    render(SourceCenterPanel, { props: { store: createSourceCenterStore(sourceApi) } });
    await screen.findByText("Suwayomi");
    expect(screen.getByText("缺少认证配置")).toBeInTheDocument();
    expect(screen.getAllByText("延后连接").length).toBeGreaterThan(0);
    expect(screen.getAllByText("子项").length).toBeGreaterThan(0);
    expect(screen.getAllByText("解析").length).toBeGreaterThan(0);
    expect(screen.getAllByText("验证").length).toBeGreaterThan(0);
    await fireEvent.change(screen.getByLabelText("媒体类型"), { target: { value: "anime" } });
    await waitFor(() => expect(screen.getByText("没有匹配的来源")).toBeInTheDocument());
    await fireEvent.change(screen.getByLabelText("媒体类型"), { target: { value: "comic" } });
    await screen.findByText("Suwayomi");
    await fireEvent.click(screen.getByRole("switch", { name: "Suwayomi 已启用" }));
    expect(sourceApi.updateSourcePreference).toHaveBeenCalled();
    await fireEvent.click(screen.getByText("健康详情与最近失败"));
    expect(screen.getAllByText("offline").length).toBeGreaterThan(0);
  });
});
