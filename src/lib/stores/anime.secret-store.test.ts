import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (value: string) => value,
  invoke: vi.fn(),
}));

type Call = { command: string; args?: Record<string, unknown> };

async function loadAnimeStore(
  handler: (command: string, args?: Record<string, unknown>) => unknown,
) {
  vi.resetModules();
  const core = await import("../api/core");
  core.setMockInvokeHandler(handler);
  const { animeStore } = await import("./anime.svelte");
  return animeStore;
}

describe("anime Bangumi SecretStore contract", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("migrates the legacy localStorage token once and deletes plaintext after validation", async () => {
    const calls: Call[] = [];
    localStorage.setItem("bangumi-token", JSON.stringify("legacy-bangumi-secret"));
    localStorage.setItem("bangumi-username", JSON.stringify("stale-user"));

    const store = await loadAnimeStore((command, args) => {
      calls.push({ command, args });
      if (command === "anime_bangumi_get_username") {
        return { username: "alice", configured: true };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    await store.init();

    expect(calls).toContainEqual({
      command: "anime_bangumi_get_username",
      args: { token: "legacy-bangumi-secret" },
    });
    expect(localStorage.getItem("bangumi-token")).toBeNull();
    expect(localStorage.getItem("bangumi-username")).toBeNull();
    expect(store.bangumiConfigured).toBe(true);
    expect(store.bangumiConnected).toBe(true);
    expect(store.bangumiUsername).toBe("alice");
    expect("bangumiToken" in store).toBe(false);
  });


  it("deletes invalid legacy plaintext and redacts it from startup state and logs", async () => {
    const secret = "legacy-bangumi-secret";
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    localStorage.setItem("bangumi-token", JSON.stringify(secret));
    localStorage.setItem("bangumi-username", JSON.stringify("stale-user"));

    const store = await loadAnimeStore((command, args) => {
      if (command === "anime_bangumi_get_username") {
        expect(args).toEqual({ token: secret });
        throw new Error(`invalid token ${secret}`);
      }
      throw new Error(`unexpected command: ${command}`);
    });

    await store.init();

    expect(localStorage.getItem("bangumi-token")).toBeNull();
    expect(localStorage.getItem("bangumi-username")).toBeNull();
    expect(store.bangumiSyncError).not.toContain(secret);
    expect(warn.mock.calls.flat().join(" ")).not.toContain(secret);
    warn.mockRestore();
  });

  it("does not persist a rejected token and redacts it from the frontend error", async () => {
    const secret = "rejected-bangumi-secret";
    const store = await loadAnimeStore((command, args) => {
      if (command === "anime_bangumi_get_username" && args?.token === null) {
        return { username: "", configured: false };
      }
      if (command === "anime_bangumi_get_username") {
        throw new Error(`Bangumi rejected ${secret}`);
      }
      throw new Error(`unexpected command: ${command}`);
    });

    await store.init();
    await expect(store.setBangumiToken(secret)).rejects.toThrow("Bangumi rejected [redacted]");

    expect(localStorage.getItem("bangumi-token")).toBeNull();
    expect(store.bangumiSyncError).toBe("Bangumi rejected [redacted]");
  });

  it("restores status without a frontend token and omits token from collection IPC", async () => {
    const calls: Call[] = [];
    const store = await loadAnimeStore((command, args) => {
      calls.push({ command, args });
      if (command === "anime_bangumi_get_username") {
        return { username: "alice", configured: true };
      }
      if (command === "anime_bangumi_get_all_collections") {
        return [{
          subject_id: 42,
          subject_name: "Example",
          subject_name_cn: "示例",
          subject_image: "",
          collection_type: 1,
          updated_at: "",
        }];
      }
      if (command === "anime_bangumi_update_collection") return true;
      throw new Error(`unexpected command: ${command}`);
    });

    await store.init();
    await store.loadBangumiCollection();
    await store.syncToBangumi("Example", 4);

    expect(calls[0]).toEqual({
      command: "anime_bangumi_get_username",
      args: { token: null },
    });
    expect(calls).toContainEqual({
      command: "anime_bangumi_get_all_collections",
      args: { username: "alice" },
    });
    expect(calls).toContainEqual({
      command: "anime_bangumi_update_collection",
      args: { subjectId: 42, collectionType: 4 },
    });
    for (const call of calls.filter(({ command }) =>
      command === "anime_bangumi_get_all_collections" ||
      command === "anime_bangumi_update_collection"
    )) {
      expect(call.args).not.toHaveProperty("token");
    }
  });

  it("deletes the Bangumi secret before clearing connected state", async () => {
    const calls: Call[] = [];
    const store = await loadAnimeStore((command, args) => {
      calls.push({ command, args });
      if (command === "anime_bangumi_get_username") {
        return { username: "alice", configured: true };
      }
      if (command === "secret_delete") {
        return { kind: "bangumi_token", configured: false };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    await store.init();
    await store.disconnectBangumi();

    expect(calls).toContainEqual({
      command: "secret_delete",
      args: { kind: "bangumi_token", origin: null },
    });
    expect(store.bangumiConfigured).toBe(false);
    expect(store.bangumiConnected).toBe(false);
    expect(store.bangumiUsername).toBe("");
  });
});
