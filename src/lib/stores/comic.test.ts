import { beforeEach, describe, expect, it, vi } from "vitest";

async function loadStore(
  handler: (command: string, args?: Record<string, unknown>) => unknown,
) {
  const api = await import("../api/core");
  api.setMockInvokeHandler(handler);
  const { comicStore } = await import("./comic.svelte");
  return { comicStore, clearMock: api.clearMockInvokeHandler };
}

describe("comicStore PicACG credential handling", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetModules();
  });

  it("uses a non-sensitive login status and never persists or exposes the token", async () => {
    const secret = "must-not-reach-the-store";
    const { comicStore, clearMock } = await loadStore((command) => {
      expect(command).toBe("comic_login");
      return { configured: true, loggedIn: true, email: "reader@example.com" };
    });

    await comicStore.login("reader@example.com", secret);

    expect(comicStore.isLoggedIn).toBe(true);
    expect(comicStore.configured).toBe(true);
    expect("token" in comicStore).toBe(false);
    expect(localStorage.getItem("picacg-token")).toBeNull();
    expect(JSON.stringify([...Array(localStorage.length)].map((_, i) => {
      const key = localStorage.key(i)!;
      return [key, localStorage.getItem(key)];
    }))).not.toContain(secret);
    clearMock();
  });

  it("restores login state through the backend without receiving a token", async () => {
    const { comicStore, clearMock } = await loadStore((command) => {
      expect(command).toBe("comic_restore_session");
      return { configured: true, loggedIn: true, email: null };
    });

    await comicStore.rehydrate();

    expect(comicStore.configured).toBe(true);
    expect(comicStore.isLoggedIn).toBe(true);
    expect("token" in comicStore).toBe(false);
    clearMock();
  });

  it("logs out through the backend and clears frontend login state", async () => {
    const commands: string[] = [];
    const { comicStore, clearMock } = await loadStore((command) => {
      commands.push(command);
      if (command === "comic_login") {
        return { configured: true, loggedIn: true, email: "reader@example.com" };
      }
      if (command === "comic_logout") {
        return { configured: false, loggedIn: false, email: null };
      }
      throw new Error(`unexpected command: ${command}`);
    });

    await comicStore.login("reader@example.com", "password");
    await comicStore.logout();

    expect(commands).toEqual(["comic_login", "comic_logout"]);
    expect(comicStore.configured).toBe(false);
    expect(comicStore.isLoggedIn).toBe(false);
    clearMock();
  });

  it("migrates a legacy localStorage token once and deletes it immediately on success", async () => {
    const legacyToken = "legacy-browser-secret";
    localStorage.setItem("picacg-token", legacyToken);
    const { comicStore, clearMock } = await loadStore((command, args) => {
      expect(command).toBe("comic_set_token");
      expect(args).toEqual({ token: legacyToken });
      return { configured: true, loggedIn: true, email: null };
    });

    await comicStore.rehydrate();

    expect(comicStore.configured).toBe(true);
    expect(comicStore.isLoggedIn).toBe(true);
    expect(localStorage.getItem("picacg-token")).toBeNull();
    expect("token" in comicStore).toBe(false);
    clearMock();
  });
});
