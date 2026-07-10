import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import { DOCK_ITEMS } from "../../nav";
import SystemDock from "./SystemDock.svelte";

describe("SystemDock accessibility contract", () => {
  it("marks the current page and exposes tools drawer state", () => {
    render(SystemDock, {
      props: {
        items: DOCK_ITEMS,
        current: "anime",
        toolsOpen: true,
        toolsControlsId: "tools-drawer",
        onpick: vi.fn(),
      },
    });

    expect(screen.getByRole("navigation", { name: "主导航" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开番剧" })).toHaveAttribute("aria-current", "page");
    const tools = screen.getByRole("button", { name: "打开工具抽屉" });
    expect(tools).toHaveAttribute("aria-expanded", "true");
    expect(tools).toHaveAttribute("aria-controls", "tools-drawer");
    expect(tools).not.toHaveAttribute("aria-current");
  });

  it("uses an explicit accessible name for Big Picture and dispatches picks", async () => {
    const onpick = vi.fn();
    render(SystemDock, {
      props: {
        items: DOCK_ITEMS,
        current: "home",
        onpick,
      },
    });

    const bigPicture = screen.getByRole("button", { name: "进入大屏模式" });
    await fireEvent.click(bigPicture);
    expect(onpick).toHaveBeenCalledWith("__bigpicture");
  });

  it("keeps shortcut badges sourced from navigation metadata", () => {
    render(SystemDock, {
      props: {
        items: DOCK_ITEMS,
        current: "home",
        onpick: vi.fn(),
      },
    });

    for (const item of DOCK_ITEMS.filter(entry => entry.shortcut)) {
      const button = screen.getByRole("button", { name: item.ariaLabel });
      expect(button).toHaveTextContent(item.shortcut!);
    }
  });
});
