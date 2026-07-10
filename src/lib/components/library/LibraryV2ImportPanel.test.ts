import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  ApplyImportResponse,
  ImportPreview,
  LibraryApi,
  LibraryHealthSnapshot,
  PreviewImportRequest,
} from "../../features/library";
import LibraryV2ImportPanel from "./LibraryV2ImportPanel.svelte";

const request: PreviewImportRequest = {
  source: "steam",
  records: [{
    sourceRecordId: "steam:480",
    title: "Spacewar",
    launchPath: null,
    installDir: "D:/Games/Spacewar",
    platformId: { source: "steam", id: "480" },
    launchUri: "steam://rungameid/480",
    fields: { "metadata.cover": "cover.jpg" },
  }],
};

const preview: ImportPreview = {
  previewId: "preview-1",
  source: "steam",
  createdAt: "2026-07-10T00:00:00Z",
  writeCount: 0,
  candidates: [{
    id: "candidate-1",
    source: "steam",
    identity: { launchPath: null, platformId: { source: "steam", id: "480" }, titleFingerprint: "spacewar" },
    action: "create",
    reason: { code: "new_identity", message: "no strong identity match was found", recalledGameIds: [] },
    matches: [],
    targetGameId: null,
    fieldDiff: [{
      field: "name",
      current: null,
      incoming: "Spacewar",
      disposition: "fill_empty",
      willApply: true,
      currentProvenance: null,
      incomingSource: "steam",
    }],
    record: request.records[0],
  }],
};

const applyResult: ApplyImportResponse = {
  jobId: "job-1",
  idempotencyKey: "key-1",
  replayed: false,
  results: [{
    candidateId: "candidate-1",
    itemIdempotencyKey: "item-1",
    action: "create",
    status: "created",
    gameId: "game-1",
    message: "game created",
    appliedFields: ["name", "library_id"],
    preservedFields: [],
  }],
  provenanceChanges: [],
};

const noHealth: LibraryHealthSnapshot = {
  state: "healthy",
  totalGames: 0,
  missingLaunchTargets: 0,
  duplicateIdentityGroups: 0,
  titleRecallGroups: 0,
  unresolvedImportConflicts: 0,
  provenanceCoverage: 1,
  issues: [],
};

function api(): LibraryApi {
  return {
    preview: vi.fn(async () => preview),
    apply: vi.fn(async () => applyResult),
    health: vi.fn(async () => noHealth),
  };
}

afterEach(cleanup);

describe("LibraryV2ImportPanel", () => {
  it("shows create/update/conflict/ignore diff summary and an explicit zero-write preview notice", async () => {
    const libraryApi = api();
    render(LibraryV2ImportPanel, { props: { request, api: libraryApi } });

    expect(await screen.findByRole("heading", { name: "Spacewar" })).toBeInTheDocument();
    expect(screen.getByText("预览阶段零写入")).toBeInTheDocument();
    expect(screen.getByText("后端写入计数：0")).toBeInTheDocument();
    expect(screen.getByText("1 项预计写入")).toBeInTheDocument();
    expect(libraryApi.preview).toHaveBeenCalledOnce();
    expect(libraryApi.apply).not.toHaveBeenCalled();
  });

  it("applies confirmed decisions and renders the apply result", async () => {
    const libraryApi = api();
    const onApplied = vi.fn();
    render(LibraryV2ImportPanel, { props: { request, api: libraryApi, onApplied } });

    await screen.findByRole("heading", { name: "Spacewar" });
    await fireEvent.click(screen.getByRole("button", { name: /确认应用/ }));

    await waitFor(() => expect(libraryApi.apply).toHaveBeenCalledOnce());
    expect(await screen.findByText("应用完成")).toBeInTheDocument();
    expect(screen.getByText("新增 1")).toBeInTheDocument();
    expect(screen.getByText("写入：name, library_id")).toBeInTheDocument();
    expect(onApplied).toHaveBeenCalledWith(applyResult);
  });
});
