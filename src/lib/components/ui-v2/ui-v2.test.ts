import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { createRawSnippet, tick } from "svelte";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import AsyncSection from "./AsyncSection.svelte";
import AsyncState from "./AsyncState.svelte";
import ContentGrid from "./ContentGrid.svelte";
import DetailPanel from "./DetailPanel.svelte";
import Drawer from "./Drawer.svelte";
import FilterBar from "./FilterBar.svelte";
import MediaCard from "./MediaCard.svelte";
import MediaRow from "./MediaRow.svelte";
import PageHeader from "./PageHeader.svelte";
import PageShell from "./PageShell.svelte";
import StateBoundary from "./StateBoundary.svelte";

function htmlSnippet(html: string) {
  return createRawSnippet(() => ({ render: () => html }));
}

async function flushFocus() {
  await tick();
  await Promise.resolve();
}

describe("ui-v2 public foundation", () => {
  it("exports the frozen public component surface", async () => {
    const api = await import("./index");
    expect(Object.keys(api).sort()).toEqual([
      "AsyncSection",
      "AsyncState",
      "ContentGrid",
      "DetailPanel",
      "Drawer",
      "FilterBar",
      "MediaCard",
      "MediaRow",
      "PageHeader",
      "PageShell",
      "StateBoundary",
    ]);
  });

  it("renders a labelled main region and can switch to a non-main root", () => {
    const main = render(PageShell, { props: { ariaLabel: "游戏库主内容" } });
    expect(screen.getByRole("main", { name: "游戏库主内容" })).toBeInTheDocument();
    main.unmount();

    render(PageShell, {
      props: {
        as: "div",
        ariaLabel: "嵌入式游戏库",
      },
    });
    expect(screen.queryByRole("main")).not.toBeInTheDocument();
    expect(screen.getByRole("region", { name: "嵌入式游戏库" }).tagName).toBe("DIV");
  });

  it("renders the page heading and couch density contract", () => {
    render(PageHeader, {
      props: {
        title: "游戏库",
        description: "管理本地游戏与启动选项。",
        density: "couch",
      },
    });

    expect(screen.getByRole("heading", { level: 1, name: "游戏库" })).toBeInTheDocument();
    expect(screen.getByText("管理本地游戏与启动选项。")).toBeInTheDocument();
    expect(screen.getByRole("banner")).toHaveAttribute("data-density", "couch");
  });

  it("announces active filters and clears them with a keyboard-accessible button", async () => {
    const onClear = vi.fn();
    render(FilterBar, {
      props: {
        activeCount: 2,
        onClear,
        density: "couch",
      },
    });

    expect(screen.getByText("已启用 2 个筛选")).toBeInTheDocument();
    const clear = screen.getByRole("button", { name: "清除筛选" });
    await fireEvent.click(clear);
    expect(onClear).toHaveBeenCalledOnce();
    expect(clear.closest("section")).toHaveAttribute("data-density", "couch");
  });

  it("locks reduced-motion and couch target styles into the public primitives", () => {
    const motionFiles = [
      "AsyncState.svelte",
      "DetailPanel.svelte",
      "Drawer.svelte",
      "MediaCard.svelte",
      "MediaRow.svelte",
    ];
    for (const file of motionFiles) {
      const source = readFileSync(resolve(process.cwd(), "src/lib/components/ui-v2", file), "utf8");
      expect(source).toContain("@media (prefers-reduced-motion: reduce)");
      expect(source).toContain('[data-motion="reduce"]');
    }

    const couchFiles = [
      "AsyncState.svelte",
      "ContentGrid.svelte",
      "DetailPanel.svelte",
      "Drawer.svelte",
      "FilterBar.svelte",
      "MediaCard.svelte",
      "MediaRow.svelte",
      "PageHeader.svelte",
    ];
    for (const file of couchFiles) {
      const source = readFileSync(resolve(process.cwd(), "src/lib/components/ui-v2", file), "utf8");
      expect(source).toContain('data-density="couch"');
      expect(source).toContain("3.5rem");
    }
  });

  it("provides an auto-fill content grid with semantic and couch metadata", () => {
    render(ContentGrid, {
      props: {
        label: "媒体内容",
        density: "couch",
        minItemWidth: "12rem",
        children: htmlSnippet('<article role="listitem">项目</article>'),
      },
    });

    const grid = screen.getByRole("list", { name: "媒体内容" });
    expect(grid).toHaveAttribute("data-density", "couch");
    expect(grid).toHaveAttribute("style", expect.stringContaining("--v2-grid-card-min: 12rem"));
    expect(within(grid).getByRole("listitem")).toHaveTextContent("项目");
  });
});

describe("ui-v2 async state contract", () => {
  it.each([
    ["loading", "正在加载"],
    ["empty", "暂无内容"],
    ["no-results", "没有匹配结果"],
    ["offline", "当前处于离线状态"],
    ["stale", "显示的是较早的数据"],
    ["partial", "部分内容未能加载"],
  ] as const)("renders the %s state in Chinese", (state, title) => {
    render(AsyncState, { props: { state, loadingDelayMs: 0 } });
    expect(screen.getByText(title)).toBeInTheDocument();
  });

  it("does not flash the default loading skeleton before its delay", () => {
    const { container } = render(AsyncState, { props: { state: "loading" } });
    expect(container.querySelector(".v2-async-state__skeleton")).not.toBeInTheDocument();
    expect(screen.getByLabelText("正在加载")).toHaveAttribute("aria-busy", "true");
  });

  it("supports primary, secondary, details and no-results semantics", async () => {
    const onPrimary = vi.fn();
    const onSecondary = vi.fn();
    render(AsyncState, {
      props: {
        state: "no-results",
        primaryAction: { label: "清除筛选", onSelect: onPrimary },
        secondaryAction: { label: "切换来源", onSelect: onSecondary },
        details: "query=galgame",
      },
    });

    await fireEvent.click(screen.getByRole("button", { name: "清除筛选" }));
    await fireEvent.click(screen.getByRole("button", { name: "切换来源" }));
    expect(onPrimary).toHaveBeenCalledOnce();
    expect(onSecondary).toHaveBeenCalledOnce();
    expect(screen.getByText("技术详情")).toBeInTheDocument();
  });

  it("preserves old content while refreshing and exposes section-level busy state", () => {
    const content = htmlSnippet('<p data-testid="cached-content">缓存内容</p>');
    render(AsyncSection, {
      props: {
        title: "漫画来源",
        description: "Komga",
        state: "refreshing",
        children: content,
      },
    });

    const section = screen.getByRole("region", { name: "漫画来源" });
    expect(section).toHaveAttribute("aria-busy", "true");
    expect(screen.getByText("正在刷新")).toBeInTheDocument();
    expect(screen.getByTestId("cached-content")).toHaveTextContent("缓存内容");
  });

  it("keeps StateBoundary as a compatible lower-level alias", async () => {
    const onRetry = vi.fn();
    render(StateBoundary, { props: { state: "error", onRetry } });
    expect(screen.getByRole("alert")).toHaveTextContent("加载失败");
    await fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(onRetry).toHaveBeenCalledOnce();
  });
});

describe("ui-v2 media patterns", () => {
  it("uses a native button for an interactive selected MediaCard", async () => {
    const onActivate = vi.fn();
    render(MediaCard, {
      props: {
        title: "樱色四重奏",
        subtitle: "已安装",
        variant: "poster",
        density: "couch",
        selected: true,
        ariaLabel: "打开樱色四重奏",
        onActivate,
      },
    });

    const button = screen.getByRole("button", { name: "打开樱色四重奏" });
    expect(button).toHaveAttribute("aria-pressed", "true");
    expect(button.closest("article")).toHaveAttribute("data-density", "couch");
    await fireEvent.click(button);
    expect(onActivate).toHaveBeenCalledOnce();
  });

  it("disables activation and announces busy state while a MediaCard is loading", () => {
    const onActivate = vi.fn();
    render(MediaCard, {
      props: {
        title: "加载中的条目",
        loading: true,
        ariaLabel: "打开加载中的条目",
        onActivate,
      },
    });

    const button = screen.getByRole("button", { name: "打开加载中的条目" });
    expect(button).toBeDisabled();
    expect(button.closest("article")).toHaveAttribute("aria-busy", "true");
  });

  it("uses a native link for MediaRow and removes disabled rows from tab order", () => {
    render(MediaRow, {
      props: {
        title: "继续阅读",
        href: "/comic/1",
        disabled: true,
        ariaLabel: "继续阅读第一章",
      },
    });

    const link = screen.getByRole("link", { name: "继续阅读第一章" });
    expect(link).toHaveAttribute("aria-disabled", "true");
    expect(link).toHaveAttribute("tabindex", "-1");
    expect(link).toHaveAttribute("href", "/comic/1");
  });
});

describe("ui-v2 overlay focus contract", () => {
  it("DetailPanel applies custom initial focus, traps Tab, handles Escape, and restores focus", async () => {
    const opener = document.createElement("button");
    opener.textContent = "打开详情";
    document.body.append(opener);
    opener.focus();

    const onClose = vi.fn();
    const props = {
      open: true,
      title: "游戏详情",
      description: "查看游戏信息。",
      initialFocus: "#detail-primary" as const,
      children: htmlSnippet('<div><button id="detail-primary">主要操作</button><button id="detail-last">最后操作</button></div>'),
      onClose,
    };
    const view = render(DetailPanel, { props });
    await flushFocus();

    const dialog = screen.getByRole("dialog");
    const primary = screen.getByRole("button", { name: "主要操作" });
    const last = screen.getByRole("button", { name: "最后操作" });
    await waitFor(() => expect(primary).toHaveFocus());

    last.focus();
    await fireEvent.keyDown(last, { key: "Tab" });
    const close = within(dialog).getByRole("button", { name: "关闭详情面板" });
    expect(close).toHaveFocus();

    await fireEvent.keyDown(close, { key: "Tab", shiftKey: true });
    expect(last).toHaveFocus();

    await fireEvent.keyDown(dialog, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();

    await view.rerender({ ...props, open: false });
    await flushFocus();
    expect(opener).toHaveFocus();
    opener.remove();
  });

  it("Drawer defaults initial focus to its safe close action and restores the trigger", async () => {
    const opener = document.createElement("button");
    opener.textContent = "打开工具";
    document.body.append(opener);
    opener.focus();

    const onClose = vi.fn();
    const props = {
      open: true,
      title: "工具",
      children: htmlSnippet('<button id="dangerous-action">删除全部</button>'),
      onClose,
      density: "couch" as const,
    };
    const view = render(Drawer, { props });
    await flushFocus();

    const dialog = screen.getByRole("dialog", { name: "工具" });
    const close = within(dialog).getByRole("button", { name: "关闭工具" });
    await waitFor(() => expect(close).toHaveFocus());
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAttribute("data-density", "couch");

    await fireEvent.keyDown(dialog, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
    await view.rerender({ ...props, open: false });
    await flushFocus();
    expect(opener).toHaveFocus();
    opener.remove();
  });
});
