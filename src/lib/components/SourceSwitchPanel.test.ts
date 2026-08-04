// SourceSwitchPanel 组件测试：最小接线调用点（spec §4 Step 10）
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SourceSwitchPanel from "./SourceSwitchPanel.svelte";
import { sourceSwitchState, type SwitchResult } from "../stores/sourceSwitch";

const mocks = vi.hoisted(() => ({
  switchSource: vi.fn(),
}));

vi.mock("../stores/sourceSwitch", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../stores/sourceSwitch")>();
  return { ...mod, switchSource: mocks.switchSource };
});

function readyRule(id: string, name: string) {
  return {
    id,
    origin: "builtin" as const,
    status: "ready" as const,
    error: null,
    manifest: {
      name,
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
    },
  };
}

const CTX = {
  contentId: "c1",
  title: "测试番",
  chapterIndex: 5,
  positionSec: 750,
};

const OK_RESULT: SwitchResult = {
  status: "ok",
  chapters: [{ id: "5", title: "第 5 集", url: "https://example.com/5", index: 5 }],
  targetChapter: { id: "5", title: "第 5 集", url: "https://example.com/5", index: 5 },
  parseResult: {
    urls: ["https://cdn.example.com/v.m3u8"],
    kind: "video",
    headers: { Referer: "https://example.com" },
  },
  resumeSec: 750,
};

beforeEach(() => {
  vi.clearAllMocks();
  sourceSwitchState.set({ switching: false, currentScope: null, lastError: null, lastResult: null });
});

describe("SourceSwitchPanel", () => {
  it("点击规则源调用 switchSource 并将结果透传（回调 + store）", async () => {
    mocks.switchSource.mockResolvedValue(OK_RESULT);
    const onResult = vi.fn();
    render(SourceSwitchPanel, {
      props: { rules: [readyRule("r1", "源A")], context: CTX, onResult },
    });

    await fireEvent.click(screen.getByTestId("switch-source-item"));

    expect(mocks.switchSource).toHaveBeenCalledWith("r1", CTX);
    await waitFor(() => {
      expect(onResult).toHaveBeenCalled();
    });
    const payload = onResult.mock.calls[0][0] as { ruleId: string; result: SwitchResult };
    expect(payload.ruleId).toBe("r1");
    expect(payload.result.status).toBe("ok");
    expect(payload.result.resumeSec).toBe(750);
    expect(payload.result.parseResult?.urls[0]).toBe("https://cdn.example.com/v.m3u8");
  });

  it("invalid 源置灰且禁止点击", async () => {
    const invalid = {
      ...readyRule("r-bad", "坏源"),
      status: "invalid" as const,
      error: { message: "规则缺少必需字段: parse", line: 3, phase: "schema" as const },
    };
    render(SourceSwitchPanel, { props: { rules: [invalid], context: CTX } });

    const item = screen.getByTestId("switch-source-item") as HTMLButtonElement;
    expect(item).toBeDisabled();
    expect(item).toHaveClass("invalid");
    expect(item).toHaveAttribute("title", "规则缺少必需字段: parse");

    await fireEvent.click(item);
    expect(mocks.switchSource).not.toHaveBeenCalled();
  });

  it("切换中显示遮罩并禁止重复点击", () => {
    sourceSwitchState.set({ switching: true, currentScope: "play:c1", lastError: null, lastResult: null });
    render(SourceSwitchPanel, { props: { rules: [readyRule("r1", "源A")], context: CTX } });

    expect(screen.getByTestId("panel-switching")).toBeInTheDocument();
    expect(screen.getByTestId("switch-source-item")).toBeDisabled();
  });

  it("切换失败展示 lastError 提示", () => {
    sourceSwitchState.set({
      switching: false,
      currentScope: "play:c1",
      lastError: "新源未找到该条目",
      lastResult: null,
    });
    render(SourceSwitchPanel, { props: { rules: [readyRule("r1", "源A")], context: CTX } });

    expect(screen.getByTestId("panel-error")).toHaveTextContent("新源未找到该条目");
  });
});
