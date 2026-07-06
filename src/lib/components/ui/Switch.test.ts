import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Switch from "./Switch.svelte";

describe("Switch", () => {
  it("renders switch with checked state", () => {
    render(Switch, { props: { checked: true } });
    const sw = screen.getByRole("switch") as HTMLInputElement;
    expect(sw).toBeInTheDocument();
    expect(sw).toBeChecked();
  });

  it("fires onchange when toggled", async () => {
    const handler = vi.fn();
    render(Switch, { props: { checked: false, onchange: handler } });
    const sw = screen.getByRole("switch") as HTMLInputElement;

    await userEvent.click(sw);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(sw).toBeChecked();
  });

  it("does not toggle when disabled", async () => {
    const handler = vi.fn();
    render(Switch, { props: { checked: false, disabled: true, onchange: handler } });
    const sw = screen.getByRole("switch") as HTMLInputElement;

    await userEvent.click(sw);
    expect(handler).not.toHaveBeenCalled();
    expect(sw).not.toBeChecked();
  });
});
