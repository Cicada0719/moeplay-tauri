// SourceHealth 徽标渲染测试（spec §6.2 测试 17）
import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import SourceHealth from "./SourceHealth.svelte";

describe("SourceHealth", () => {
  it.each([
    ["Healthy", "可用"],
    ["Degraded", "波动"],
    ["Abnormal", "异常"],
    ["Unknown", "未知"],
  ] as const)("%s 渲染正确文案与样式类", (status, text) => {
    render(SourceHealth, { props: { status, size: "sm" } });
    const badge = screen.getByTestId("source-health");
    expect(badge).toHaveAttribute("data-status", status);
    expect(badge.textContent).toContain(text);
    expect(badge.className).toContain(`source-health--${status.toLowerCase()}`);
  });

  it("Abnormal 时 title 提示 lastError", () => {
    render(SourceHealth, {
      props: { status: "Abnormal", lastError: "网络错误: 超时", size: "sm" },
    });
    const badge = screen.getByTestId("source-health");
    expect(badge).toHaveAttribute("title", "网络错误: 超时");
  });

  it("Healthy 不带 lastError title", () => {
    render(SourceHealth, {
      props: { status: "Healthy", lastError: "忽略", size: "sm" },
    });
    const badge = screen.getByTestId("source-health");
    expect(badge).not.toHaveAttribute("title");
  });

  it("md 尺寸展示延迟毫秒", () => {
    render(SourceHealth, { props: { status: "Healthy", latencyMs: 123, size: "md" } });
    expect(screen.getByTestId("source-health-latency")).toHaveTextContent("123ms");
  });

  it("sm 尺寸不展示延迟", () => {
    render(SourceHealth, { props: { status: "Healthy", latencyMs: 123, size: "sm" } });
    expect(screen.queryByTestId("source-health-latency")).not.toBeInTheDocument();
  });
});
