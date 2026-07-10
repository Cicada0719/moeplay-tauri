import { describe, expect, it } from "vitest";
import fixtureCases from "../../../../tests/fixtures/library/import-cases.json";
import type {
  ApplyImportResponse,
  ImportAction,
  ImportCandidate,
  ImportPreview,
  LibraryApi,
  LibraryHealthSnapshot,
  PreviewImportRequest,
} from "./contracts";
import { createLibraryFeatureStore } from "./store";

interface Deferred<T> {
  promise: Promise<T>;
  resolve(value: T): void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

function candidate(title: string, action: ImportAction, overrides: Partial<ImportCandidate> = {}): ImportCandidate {
  return {
    id: `candidate-${title}`,
    source: "fixture",
    identity: { launchPath: `c:/games/${title}.exe`, platformId: null, titleFingerprint: title.toLowerCase() },
    action,
    reason: { code: "new_identity", message: "fixture", recalledGameIds: [] },
    matches: [],
    targetGameId: null,
    fieldDiff: [],
    record: {
      sourceRecordId: title,
      title,
      launchPath: `c:/games/${title}.exe`,
      installDir: null,
      platformId: null,
      launchUri: null,
      fields: {},
    },
    ...overrides,
  };
}

function preview(title: string, action: ImportAction = "create"): ImportPreview {
  return {
    previewId: `preview-${title}`,
    source: "fixture",
    candidates: [candidate(title, action)],
    createdAt: "2026-07-10T00:00:00Z",
    writeCount: 0,
  };
}

const health: LibraryHealthSnapshot = {
  state: "healthy",
  totalGames: 1,
  missingLaunchTargets: 0,
  duplicateIdentityGroups: 0,
  titleRecallGroups: 0,
  unresolvedImportConflicts: 0,
  provenanceCoverage: 1,
  issues: [],
};

function unusedApi(): Pick<LibraryApi, "apply" | "health"> {
  return {
    apply: async () => { throw new Error("not used"); },
    health: async () => health,
  };
}

describe("library feature request generations", () => {
  it("isolates a late preview after a newer generation wins", async () => {
    const requests: Array<{ signal: AbortSignal; result: Deferred<ImportPreview> }> = [];
    const api: LibraryApi = {
      preview(_request, signal) {
        const result = deferred<ImportPreview>();
        requests.push({ signal, result });
        return result.promise;
      },
      ...unusedApi(),
    };
    const store = createLibraryFeatureStore(api);
    const oldRequest = store.preview({ source: "fixture", records: [] });
    const newRequest = store.preview({ source: "fixture", records: [] });

    expect(requests[0].signal.aborted).toBe(true);
    requests[1].result.resolve(preview("new"));
    await newRequest;
    requests[0].result.resolve(preview("old"));
    await oldRequest;

    expect(store.getSnapshot().preview?.previewId).toBe("preview-new");
    expect(store.getSnapshot().previewGeneration).toBe(2);
  });

  it("cancels a generation and ignores its late completion", async () => {
    const result = deferred<ImportPreview>();
    const api: LibraryApi = {
      preview: (_request, _signal) => result.promise,
      ...unusedApi(),
    };
    const store = createLibraryFeatureStore(api);
    const pending = store.preview({ source: "fixture", records: [] });
    store.cancelPreview();
    result.resolve(preview("late"));
    await pending;

    expect(store.getSnapshot().preview).toBeNull();
    expect(store.getSnapshot().isPreviewing).toBe(false);
    expect(store.getSnapshot().previewGeneration).toBe(2);
  });
});

describe("library import contract state", () => {
  it("keeps same-title recall as conflict and path move as platform update", async () => {
    const sameTitle = fixtureCases.sameTitleDifferentWorks;
    const pathMove = fixtureCases.pathMove;
    const response: ImportPreview = {
      previewId: "fixture-identities",
      source: "fixture",
      createdAt: "2026-07-10T00:00:00Z",
      writeCount: 0,
      candidates: [
        candidate(sameTitle.title, "conflict", {
          id: sameTitle.sourceRecordId,
          reason: { code: "title_recall_only", message: "recall only", recalledGameIds: ["work-a"] },
          targetGameId: null,
        }),
        candidate(pathMove.title, "update", {
          id: pathMove.sourceRecordId,
          identity: {
            launchPath: pathMove.newPath.toLowerCase().replaceAll("\\", "/"),
            platformId: { source: pathMove.platformSource!, id: pathMove.platformId! },
            titleFingerprint: pathMove.title.toLowerCase(),
          },
          targetGameId: "game-moved",
          reason: { code: "platform_id_match", message: "path moved", recalledGameIds: ["game-moved"] },
        }),
      ],
    };
    const api: LibraryApi = {
      preview: async () => response,
      ...unusedApi(),
    };
    const store = createLibraryFeatureStore(api);
    await store.preview({ source: "fixture", records: [] });
    const candidates = store.getSnapshot().preview!.candidates;
    expect(candidates[0].action).toBe("conflict");
    expect(candidates[0].targetGameId).toBeNull();
    expect(candidates[1].action).toBe("update");
    expect(candidates[1].identity.platformId?.id).toBe("480");
  });

  it("retains provenance diffs and exposes idempotent replay results", async () => {
    const imported = candidate("Example", "update", {
      id: "candidate-example",
      targetGameId: "game-1",
      fieldDiff: [{
        field: "description",
        current: "my note",
        incoming: "provider text",
        disposition: "preserve_user",
        willApply: false,
        currentProvenance: {
          gameId: "game-1",
          field: "description",
          source: "steam",
          sourceRecordId: "10",
          importedAt: "2026-01-01T00:00:00Z",
          appliedValue: "old imported",
          valueHash: "old",
        },
        incomingSource: "steam",
      }],
    });
    const importPreview: ImportPreview = {
      ...preview("Example", "update"),
      candidates: [imported],
    };
    let applyCount = 0;
    const applyResult = (replayed: boolean): ApplyImportResponse => ({
      jobId: "library-import-42",
      idempotencyKey: "batch-42",
      replayed,
      results: [{
        candidateId: imported.id,
        itemIdempotencyKey: "item-42",
        action: "update",
        status: replayed ? "already_applied" : "no_changes",
        gameId: "game-1",
        message: replayed ? "replayed" : "user field preserved",
        appliedFields: [],
        preservedFields: ["description"],
      }],
      provenanceChanges: [],
    });
    const api: LibraryApi = {
      preview: async (_request: PreviewImportRequest) => importPreview,
      apply: async () => applyResult(++applyCount > 1),
      health: async () => health,
    };
    const store = createLibraryFeatureStore(api);
    await store.preview({ source: "steam", records: [] });
    expect(store.getSnapshot().preview?.candidates[0].fieldDiff[0].willApply).toBe(false);

    await store.apply([], "batch-42");
    expect(store.getSnapshot().applyResult?.results[0].status).toBe("no_changes");
    await store.apply([], "batch-42");
    expect(store.getSnapshot().applyResult?.replayed).toBe(true);
    expect(store.getSnapshot().applyResult?.results[0].status).toBe("already_applied");
  });
});
