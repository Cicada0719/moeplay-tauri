import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ErrorOverlay from "./ErrorOverlay.svelte";
import type { PlayerError } from "../../stores/player";

function makeError(overrides: Partial<PlayerError> = {}): PlayerError {
  return { kind: "HTTP_FORBIDDEN", message: "该源拒绝访问", detail: "403 Forbidden", httpStatus: 403, occurredAt: 0, ...overrides };
}

describe("ErrorOverlay", () => {
  it("渲染错误类型文案与 httpStatus", () => {
    render(ErrorOverlay, {
      props: {
        error: makeError(),
        retryCount: 0,
        onRetry: vi.fn(),
        onSwitchSource: vi.fn(),
        onCopyLog: vi.fn(),
      },
    });

    expect(screen.getByText("该源拒绝访问（防盗链），建议切换源")).toBeInTheDocument();
    expect(screen.getByText(/HTTP_FORBIDDEN/)).toBeInTheDocument();
    expect(screen.getByText(/HTTP 403/)).toBeInTheDocument();
    expect(screen.getByText("403 Forbidden")).toBeInTheDocument();
  });

  it("HTTP_FORBIDDEN 时「切换源」按钮默认聚焦", () => {
    render(ErrorOverlay, {
      props: {
        error: makeError({ kind: "HTTP_FORBIDDEN" }),
        retryCount: 0,
        onRetry: vi.fn(),
        onSwitchSource: vi.fn(),
        onCopyLog: vi.fn(),
      },
    });
    expect(screen.getByRole("button", { name: /切换源/ })).toHaveFocus();
  });

  it("PARSE_EMPTY 时「切换源」按钮默认聚焦", () => {
    render(ErrorOverlay, {
      props: {
        error: makeError({ kind: "PARSE_EMPTY", httpStatus: undefined }),
        retryCount: 0,
        onRetry: vi.fn(),
        onSwitchSource: vi.fn(),
        onCopyLog: vi.fn(),
      },
    });
    expect(screen.getByRole("button", { name: /切换源/ })).toHaveFocus();
  });

  it("NETWORK 时不默认聚焦「切换源」", () => {
    render(ErrorOverlay, {
      props: {
        error: makeError({ kind: "NETWORK", httpStatus: undefined }),
        retryCount: 0,
        onRetry: vi.fn(),
        onSwitchSource: vi.fn(),
        onCopyLog: vi.fn(),
      },
    });
    expect(screen.getByRole("button", { name: /切换源/ })).not.toHaveFocus();
  });

  it("点击重试/切换源/复制日志分别触发对应回调，重试次数文案展示", async () => {
    const onRetry = vi.fn();
    const onSwitchSource = vi.fn();
    const onCopyLog = vi.fn();
    render(ErrorOverlay, {
      props: {
        error: makeError({ kind: "MEDIA_DECODE", httpStatus: undefined }),
        retryCount: 2,
        onRetry,
        onSwitchSource,
        onCopyLog,
      },
    });

    await userEvent.click(screen.getByRole("button", { name: /重试（2）/ }));
    await userEvent.click(screen.getByRole("button", { name: /切换源/ }));
    await userEvent.click(screen.getByRole("button", { name: /复制日志/ }));

    expect(onRetry).toHaveBeenCalledTimes(1);
    expect(onSwitchSource).toHaveBeenCalledTimes(1);
    expect(onCopyLog).toHaveBeenCalledTimes(1);
  });

  it("传入 onClose 时渲染关闭按钮，点击触发 onClose（错误弹层可关闭）", async () => {
    const onClose = vi.fn();
    render(ErrorOverlay, {
      props: {
        error: makeError(),
        retryCount: 0,
        onRetry: vi.fn(),
        onSwitchSource: vi.fn(),
        onCopyLog: vi.fn(),
        onClose,
      },
    });
    await userEvent.click(screen.getByRole("button", { name: "关闭错误提示" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("未传 onClose 时不渲染关闭按钮", () => {
    render(ErrorOverlay, {
      props: {
        error: makeError(),
        retryCount: 0,
        onRetry: vi.fn(),
        onSwitchSource: vi.fn(),
        onCopyLog: vi.fn(),
      },
    });
    expect(screen.queryByRole("button", { name: "关闭错误提示" })).not.toBeInTheDocument();
  });
});
