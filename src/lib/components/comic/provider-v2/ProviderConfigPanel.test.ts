import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import ProviderConfigPanel from "./ProviderConfigPanel.svelte";

afterEach(cleanup);

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("Provider v2 configuration UI", () => {
  it("uses the existing directory picker for local roots", async () => {
    const onconfigure = vi.fn();
    const pickRoot = vi.fn().mockResolvedValue("C:\\Comics");
    render(ProviderConfigPanel, { props: { onconfigure, pickRoot } });

    await fireEvent.click(screen.getByRole("button", { name: /选择目录/ }));
    expect(pickRoot).toHaveBeenCalledOnce();
    expect(screen.getByLabelText("本地漫画根目录")).toHaveValue("C:\\Comics");

    await fireEvent.click(screen.getByRole("button", { name: /保存并切换/ }));
    expect(onconfigure).toHaveBeenCalledWith({ kind: "local", root: "C:\\Comics" });
  });

  it("clears a Komga credential immediately after creating the one-time request", async () => {
    const pending = deferred();
    const onconfigure = vi.fn(() => pending.promise);
    render(ProviderConfigPanel, { props: { onconfigure } });

    await fireEvent.click(screen.getByRole("button", { name: /Komga/ }));
    await fireEvent.input(screen.getByLabelText("漫画服务器地址"), { target: { value: "https://example.com" } });
    await fireEvent.click(screen.getByRole("button", { name: "Basic" }));
    await fireEvent.input(screen.getByLabelText("Komga 用户名"), { target: { value: "reader" } });
    const secret = screen.getByLabelText("一次性漫画源凭据");
    await fireEvent.input(secret, { target: { value: "one-time-password" } });
    await fireEvent.click(screen.getByRole("button", { name: /保存并切换/ }));

    expect(onconfigure).toHaveBeenCalledWith({
      kind: "komga",
      baseUrl: "https://example.com",
      authMode: "basic",
      username: "reader",
      secret: "one-time-password",
    });
    expect(secret).toHaveValue("");
    pending.resolve();
  });

  it("locks Kavita configuration to API Key authentication", async () => {
    const onconfigure = vi.fn();
    render(ProviderConfigPanel, { props: { onconfigure } });

    await fireEvent.click(screen.getByRole("button", { name: /Kavita/ }));
    expect(screen.getByRole("button", { name: "API Key" })).toBeInTheDocument();
    await fireEvent.input(screen.getByLabelText("漫画服务器地址"), { target: { value: "https://kavita.example" } });
    await fireEvent.input(screen.getByLabelText("一次性漫画源凭据"), { target: { value: "key" } });
    await fireEvent.click(screen.getByRole("button", { name: /保存并切换/ }));

    expect(onconfigure).toHaveBeenCalledWith({
      kind: "kavita",
      baseUrl: "https://kavita.example",
      authMode: "api_key",
      secret: "key",
    });
  });
});
