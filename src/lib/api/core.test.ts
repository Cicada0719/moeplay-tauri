import { describe, it, expect, vi } from "vitest";
import {
  invokeCmd,
  setMockInvokeHandler,
  clearMockInvokeHandler,
  isMockEnabled,
  mockRouter,
} from "./core";

describe("api/core", () => {
  it("throws when no mock is registered and tauri is unavailable", async () => {
    clearMockInvokeHandler();
    expect(isMockEnabled()).toBe(false);
    // In unit tests the real @tauri-apps/api/core is not available,
    // so invoking without a mock should reject.
    await expect(invokeCmd("get_games")).rejects.toBeDefined();
  });

  it("uses mock handler when registered", async () => {
    setMockInvokeHandler((cmd, args) => {
      if (cmd === "echo") return { cmd, args };
      throw new Error("unknown");
    });

    expect(isMockEnabled()).toBe(true);
    const result = await invokeCmd<{ cmd: string }>("echo", { hello: "world" });
    expect(result.cmd).toBe("echo");

    clearMockInvokeHandler();
  });

  it("mockRouter routes commands to handlers", async () => {
    setMockInvokeHandler(
      mockRouter({
        get_settings: () => ({ theme: "dark" }),
        ping: () => "pong",
      })
    );

    const settings = await invokeCmd<{ theme: string }>("get_settings");
    expect(settings.theme).toBe("dark");

    const pong = await invokeCmd<string>("ping");
    expect(pong).toBe("pong");

    await expect(invokeCmd("unknown")).rejects.toThrow("未注册命令");

    clearMockInvokeHandler();
  });
});
