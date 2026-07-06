import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import DialogTestWrapper from "./DialogTestWrapper.svelte";

describe("Dialog", () => {
  it("renders when open and calls onClose on Escape", async () => {
    const onClose = vi.fn();
    render(DialogTestWrapper, { props: { open: true, onClose, title: "测试弹窗" } });

    expect(screen.getByRole("dialog", { name: "测试弹窗" })).toBeInTheDocument();

    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("does not render when closed", () => {
    render(DialogTestWrapper, { props: { open: false, onClose: vi.fn() } });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("traps focus inside the dialog", async () => {
    render(DialogTestWrapper, { props: { open: true, onClose: vi.fn(), title: "聚焦测试" } });

    const dialog = screen.getByRole("dialog", { name: "聚焦测试" });
    const innerButton = screen.getByRole("button", { name: "内部按钮" });

    // Dialog panel itself should receive initial focus
    expect(dialog).toHaveFocus();

    await userEvent.tab();
    expect(innerButton).toHaveFocus();

    await userEvent.tab();
    // With only one inner focusable, focus should cycle back to the panel
    expect(dialog).toHaveFocus();
  });
});
