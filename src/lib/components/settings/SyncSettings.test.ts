import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import SyncSettings from "./SyncSettings.svelte";
import { clearMockInvokeHandler, setMockInvokeHandler } from "../../api/core";
import type { SyncStatus, WebDavConfig } from "../../types/sync";

const CONFIG: WebDavConfig = {
  base_url: "https://dav.example.com/dav",
  username: "alice",
  mode: "manual",
  remote_dir: "moeplay-sync",
};

const EMPTY_STATUS: SyncStatus = { configured: false, last_result: null, syncing: false };

function mockInvoke(handlers: Record<string, (args?: Record<string, unknown>) => unknown>) {
  setMockInvokeHandler((command, args) => {
    const handler = handlers[command];
    if (!handler) throw new Error(`[mock] 未注册命令: ${command}`);
    return handler(args ?? {});
  });
}

afterEach(() => {
  clearMockInvokeHandler();
});

describe("SyncSettings", () => {
  it("认证失败时红字显示错误文案，且不崩溃", async () => {
    mockInvoke({
      get_sync_config: () => null,
      get_sync_status: () => EMPTY_STATUS,
      test_webdav_connection: () => {
        throw { kind: "auth", message: "认证失败，请检查用户名与授权密码" };
      },
    });

    render(SyncSettings);

    await userEvent.type(screen.getByLabelText("WebDAV 地址"), CONFIG.base_url);
    await userEvent.type(screen.getByLabelText("用户名"), CONFIG.username);
    await userEvent.type(screen.getByLabelText("密码"), "app-password");
    await userEvent.click(screen.getByRole("button", { name: /测试连接/ }));

    expect(screen.getByRole("alert")).toHaveTextContent("认证失败，请检查用户名与授权密码");
    // 认证失败不触发成功 toast 文案（红色错误区，非结果条）。
    expect(screen.queryByText(/上传 \d+ 条/)).not.toBeInTheDocument();
  });

  it("立即同步进行中时按钮禁用（含 loading）", async () => {
    let resolveNow: (() => void) | undefined;
    mockInvoke({
      get_sync_config: () => CONFIG,
      get_sync_status: () => ({ ...EMPTY_STATUS, configured: true }),
      sync_now: () =>
        new Promise((resolve) => {
          resolveNow = () => resolve({ uploaded: 0, downloaded: 0, conflicts: 0, tombstones_purged: 0, synced_at: 0 });
        }),
    });

    render(SyncSettings);

    const button = await vi.waitFor(() => {
      const el = screen.getByRole("button", { name: /立即同步/ });
      expect(el).not.toBeDisabled();
      return el;
    });

    await userEvent.click(button);
    expect(screen.getByRole("button", { name: /立即同步/ })).toBeDisabled();

    // 结束挂起的同步，避免测试悬挂。
    resolveNow?.();
  });

  it("同步完成后结果条展示上传 n / 下载 m / 冲突 k", async () => {
    mockInvoke({
      get_sync_config: () => CONFIG,
      get_sync_status: () => ({ ...EMPTY_STATUS, configured: true }),
      sync_now: () => ({ uploaded: 3, downloaded: 2, conflicts: 1, tombstones_purged: 0, synced_at: 1_750_000_000 }),
    });

    render(SyncSettings);

    const button = await vi.waitFor(() => {
      const el = screen.getByRole("button", { name: /立即同步/ });
      expect(el).not.toBeDisabled();
      return el;
    });

    await userEvent.click(button);

    expect(screen.getByText("上传 3 条 / 下载 2 条 / 冲突 1 条")).toBeInTheDocument();
  });

  it("已配置时密码占位符提示已保存（不显示）", async () => {
    mockInvoke({
      get_sync_config: () => CONFIG,
      get_sync_status: () => ({ ...EMPTY_STATUS, configured: true }),
    });

    render(SyncSettings);

    await vi.waitFor(() => {
      expect(screen.getByLabelText("密码")).toHaveAttribute(
        "placeholder",
        "已保存（不显示）",
      );
    });
    expect(screen.getByLabelText("WebDAV 地址")).toHaveValue(CONFIG.base_url);
    expect(screen.getByLabelText("用户名")).toHaveValue(CONFIG.username);
  });

  it("准确说明当前同步范围，不宣称 IndexedDB 阅读历史已接入", async () => {
    mockInvoke({
      get_sync_config: () => null,
      get_sync_status: () => EMPTY_STATUS,
    });

    render(SyncSettings);

    expect(screen.getByText(/当前仅同步已接入 SQLite 的历史记录/)).toBeInTheDocument();
    expect(screen.getByText(/IndexedDB 历史尚未支持跨设备接力/)).toBeInTheDocument();
  });

  it("清除配置需二次确认，确认后调用 clear_webdav_config 并复位表单", async () => {
    const confirmMock = vi.fn().mockReturnValue(true);
    window.confirm = confirmMock as unknown as typeof window.confirm;
    let cleared = false;
    mockInvoke({
      get_sync_config: () => CONFIG,
      get_sync_status: () => ({ ...EMPTY_STATUS, configured: true }),
      clear_webdav_config: () => {
        cleared = true;
      },
    });

    render(SyncSettings);

    const clearButton = await vi.waitFor(() => {
      const el = screen.getByRole("button", { name: /清除配置/ });
      expect(el).not.toBeDisabled();
      return el;
    });
    await userEvent.click(clearButton);

    expect(confirmMock).toHaveBeenCalled();
    expect(cleared).toBe(true);
    // 复位后：地址/用户名清空，密码占位符回到默认。
    expect(screen.getByLabelText("WebDAV 地址")).toHaveValue("");
    expect(screen.getByLabelText("用户名")).toHaveValue("");
  });
});
