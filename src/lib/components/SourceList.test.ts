// SourceList 组件测试（对应 spec §6.2 测试 20）
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SourceList from "./SourceList.svelte";
import { sourceSwitchState } from "../stores/sourceSwitch";
import type { LoadedRule } from "../api/rules";

const mocks = vi.hoisted(() => ({
  open: vi.fn(),
  importRule: vi.fn(),
  removeCustomRule: vi.fn(),
  getHealth: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: mocks.open,
}));

vi.mock("../api/rules", () => ({
  importRule: mocks.importRule,
  removeCustomRule: mocks.removeCustomRule,
  getHealth: mocks.getHealth,
}));

function rule(overrides: Partial<LoadedRule> & { id: string; manifest: LoadedRule["manifest"] }): LoadedRule {
  return {
    origin: "builtin",
    status: "ready",
    error: null,
    ...overrides,
  };
}

const READY_MANIFEST = {
  name: "测试源",
  version: "1.0.0",
  contentType: "anime" as const,
  baseUrl: "https://example.com",
  language: "zh-CN",
  nsfw: false,
  author: null,
  search: "function search(k,p){ return []; }",
  detail: "function detail(u){ return {}; }",
  chapter: "function chapter(u){ return []; }",
  parse: "function parse(u){ return { urls: [], kind: 'video' }; }",
};

beforeEach(() => {
  vi.clearAllMocks();
  mocks.getHealth.mockResolvedValue([]);
  sourceSwitchState.set({ switching: false, currentScope: null, lastError: null, lastResult: null });
});

describe("SourceList", () => {
  it("invalid 源置灰、禁止点击，title 提示错误信息", () => {
    const invalid = rule({
      id: "invalid-1",
      manifest: { ...READY_MANIFEST, name: "坏源" },
      status: "invalid",
      error: { message: "规则缺少必需字段: parse", line: 3, phase: "schema" },
    });
    render(SourceList, { props: { rules: [invalid], activeRuleId: null } });

    const item = screen.getByTestId("source-item") as HTMLButtonElement;
    expect(item).toBeDisabled();
    expect(item).toHaveClass("invalid");
    expect(item).toHaveAttribute("title", "规则缺少必需字段: parse");
  });

  it("custom 源显示「自定义」徽标与删除按钮", () => {
    const custom = rule({
      id: "custom-1",
      manifest: { ...READY_MANIFEST, name: "我的源" },
      origin: "custom",
    });
    render(SourceList, { props: { rules: [custom], activeRuleId: null } });

    expect(screen.getByTestId("custom-badge")).toHaveTextContent("自定义");
    expect(screen.getByTestId("remove-btn")).toBeInTheDocument();
  });

  it("切换中显示 loading 遮罩并禁止点击", () => {
    const ready = rule({
      id: "ready-1",
      manifest: { ...READY_MANIFEST, name: "正常源" },
    });
    sourceSwitchState.set({ switching: true, currentScope: "play:c1", lastError: null, lastResult: null });
    render(SourceList, { props: { rules: [ready], activeRuleId: null } });

    expect(screen.getByTestId("switching-overlay")).toBeInTheDocument();
    expect(screen.getByTestId("source-item")).toBeDisabled();
  });

  it("导入失败展示具体字段名与行号", async () => {
    mocks.open.mockResolvedValue("C:\\rules\\bad.json");
    mocks.importRule.mockRejectedValue({
      message: "规则缺少必需字段: parse",
      line: 3,
      phase: "schema",
    });
    render(SourceList, { props: { rules: [], activeRuleId: null } });

    await fireEvent.click(screen.getByTestId("import-btn"));
    await waitFor(() => {
      expect(screen.getByTestId("import-error")).toHaveTextContent("parse");
      expect(screen.getByTestId("import-error")).toHaveTextContent("第 3 行");
    });
    expect(mocks.open).toHaveBeenCalledWith({
      multiple: false,
      filters: [{ name: "规则文件", extensions: ["json", "yaml", "yml"] }],
    });
  });

  it("导入成功：调用 open 对话框并提交 importRule", async () => {
    const imported = rule({
      id: "custom-2",
      manifest: { ...READY_MANIFEST, name: "新导入" },
      origin: "custom",
    });
    mocks.open.mockResolvedValue("C:\\rules\\ok.json");
    mocks.importRule.mockResolvedValue(imported);

    render(SourceList, { props: { rules: [], activeRuleId: null } });
    await fireEvent.click(screen.getByTestId("import-btn"));
    await waitFor(() => {
      expect(mocks.importRule).toHaveBeenCalledWith("C:\\rules\\ok.json");
    });
    // 成功后无错误提示
    expect(screen.queryByTestId("import-error")).not.toBeInTheDocument();
  });

  it("健康状态：Abnormal 沉底、置灰但可点击，Healthy 置顶（spec task-02 Step 7.4）", async () => {
    const healthy = rule({
      id: "healthy-1",
      manifest: { ...READY_MANIFEST, name: "健康源" },
    });
    const abnormal = rule({
      id: "abnormal-1",
      manifest: { ...READY_MANIFEST, name: "异常源" },
    });
    mocks.getHealth.mockResolvedValue([
      { sourceId: "healthy-1", status: "Healthy", consecutiveFailures: 0, lastCheckedAt: 1, lastLatencyMs: 50, lastError: null },
      { sourceId: "abnormal-1", status: "Abnormal", consecutiveFailures: 3, lastCheckedAt: 1, lastLatencyMs: 500, lastError: "网络错误" },
    ]);

    // 入参顺序为 [abnormal, healthy]，渲染后应被 sortByHealth 重排
    render(SourceList, { props: { rules: [abnormal, healthy], activeRuleId: null } });

    await waitFor(() => {
      const items = screen.getAllByTestId("source-item") as HTMLButtonElement[];
      expect(items).toHaveLength(2);
      expect(items[0].textContent).toContain("健康源");
      expect(items[1].textContent).toContain("异常源");
      expect(items[1]).toHaveClass("health-abnormal");
      expect(items[1]).not.toBeDisabled(); // 异常源仍可点击（FR-04 不阻止手动尝试）
    });

    const badges = screen.getAllByTestId("source-health");
    expect(badges[0]).toHaveAttribute("data-status", "Healthy");
    expect(badges[1]).toHaveAttribute("data-status", "Abnormal");
    expect(badges[1]).toHaveAttribute("title", "网络错误");
  });
});
