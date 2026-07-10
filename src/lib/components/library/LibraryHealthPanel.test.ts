import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LibraryApi, LibraryHealthSnapshot } from "../../features/library";
import LibraryHealthPanel from "./LibraryHealthPanel.svelte";

const health: LibraryHealthSnapshot = {
  state: "degraded",
  totalGames: 4,
  missingLaunchTargets: 1,
  duplicateIdentityGroups: 1,
  titleRecallGroups: 0,
  unresolvedImportConflicts: 1,
  provenanceCoverage: 0.5,
  issues: [
    { code: "missing_launch_target", severity: "error", message: "missing", gameIds: ["game-1"] },
    { code: "duplicate_strong_identity", severity: "error", message: "duplicate", gameIds: ["game-1", "game-2"] },
    { code: "unresolved_import_conflicts", severity: "warning", message: "conflict", gameIds: [] },
  ],
};

function api(): LibraryApi {
  return {
    preview: vi.fn(async () => { throw new Error("unused"); }),
    apply: vi.fn(async () => { throw new Error("unused"); }),
    health: vi.fn(async () => health),
  };
}

afterEach(cleanup);

describe("LibraryHealthPanel", () => {
  it("loads health and shows invalid paths, duplicates, conflicts, and provenance coverage", async () => {
    const libraryApi = api();
    render(LibraryHealthPanel, {
      props: {
        open: true,
        api: libraryApi,
        onClose: vi.fn(),
        games: [{ id: "game-1", name: "失效游戏", exe_path: "D:/Missing/game.exe", launch_uri: null }],
      },
    });

    expect(await screen.findByText("50%")).toBeInTheDocument();
    expect(screen.getByText("失效路径明细")).toBeInTheDocument();
    expect(screen.getByText("D:/Missing/game.exe")).toBeInTheDocument();
    expect(screen.getByText("多个游戏共享同一启动路径或平台 ID")).toBeInTheDocument();
    expect(screen.getByText("仍有 Library v2 导入冲突等待人工决策")).toBeInTheDocument();
    expect(libraryApi.health).toHaveBeenCalledOnce();
  });
});
