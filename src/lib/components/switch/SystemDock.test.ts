import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import { DOCK_ITEMS } from "../../nav";
import SystemDock from "./SystemDock.svelte";

const railItems = DOCK_ITEMS.filter((item) => item.id === "tools" || item.surface === "mode");

describe("SystemDock accessibility contract", () => {
  it("exposes the tools drawer without duplicating primary navigation", () => {
    render(SystemDock, {
      props: {
        items: railItems,
        current: "__tools",
        toolsOpen: true,
        toolsControlsId: "tools-drawer",
        onpick: vi.fn(),
      },
    });

    expect(screen.getByRole("navigation", { name: "系统快捷操作" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "打开番剧" })).not.toBeInTheDocument();
    const tools = screen.getByRole("button", { name: "打开工具抽屉" });
    expect(tools).toHaveAttribute("aria-expanded", "true");
    expect(tools).toHaveAttribute("aria-controls", "tools-drawer");
  });

  it("uses an explicit accessible name for Big Picture and dispatches picks", async () => {
    const onpick = vi.fn();
    render(SystemDock, { props: { items: railItems, current: "home", onpick } });
    const bigPicture = screen.getByRole("button", { name: "进入大屏模式" });
    await fireEvent.click(bigPicture);
    expect(onpick).toHaveBeenCalledWith("__bigpicture");
  });
});
