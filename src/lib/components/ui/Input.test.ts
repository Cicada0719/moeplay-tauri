import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Input from "./Input.svelte";

describe("Input", () => {
  it("renders with placeholder and value", () => {
    render(Input, { props: { value: "hello", placeholder: "Type here", ariaLabel: "test input" } });
    const input = screen.getByRole("textbox", { name: "test input" }) as HTMLInputElement;
    expect(input).toBeInTheDocument();
    expect(input).toHaveValue("hello");
  });

  it("fires oninput when typing", async () => {
    const handler = vi.fn();
    render(Input, { props: { value: "", ariaLabel: "test input", oninput: handler } });
    const input = screen.getByRole("textbox", { name: "test input" }) as HTMLInputElement;

    await userEvent.type(input, "abc");
    expect(handler).toHaveBeenCalledTimes(3);
    expect(input).toHaveValue("abc");
  });
});
