import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ButtonTestWrapper from "./ButtonTestWrapper.svelte";

describe("Button", () => {
  it("renders children and responds to click", async () => {
    const handler = vi.fn();
    render(ButtonTestWrapper, { props: { label: "保存", onclick: handler } });

    const btn = screen.getByRole("button", { name: "保存" });
    expect(btn).toBeInTheDocument();

    await userEvent.click(btn);
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("supports aria-label for icon-only buttons", () => {
    render(ButtonTestWrapper, { props: { label: "", ariaLabel: "关闭" } });
    expect(screen.getByRole("button", { name: "关闭" })).toBeInTheDocument();
  });

  it("is disabled and has aria-busy when loading", () => {
    render(ButtonTestWrapper, { props: { label: "提交", loading: true } });
    const btn = screen.getByRole("button", { name: "提交" });
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("aria-busy", "true");
  });

  it("does not fire click when disabled", async () => {
    const handler = vi.fn();
    render(ButtonTestWrapper, { props: { label: "提交", disabled: true, onclick: handler } });

    await userEvent.click(screen.getByRole("button"));
    expect(handler).not.toHaveBeenCalled();
  });
});
