import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import ImportDiff from "./ImportDiff.svelte";
import LibraryHealth from "./LibraryHealth.svelte";
import type { ImportCandidate, LibraryHealthSnapshot } from "./contracts";

const candidate: ImportCandidate = {
  id: "candidate-1",
  source: "local_scan",
  identity: { launchPath: "d:/games/moonlight/game.exe", platformId: null, titleFingerprint: "moonlight" },
  action: "conflict",
  reason: { code: "title_recall_only", message: "title fingerprint is recall-only", recalledGameIds: ["work-a"] },
  matches: [{ gameId: "work-a", gameTitle: "Moonlight", kind: "title_recall", confidence: 0.5 }],
  targetGameId: null,
  record: {
    sourceRecordId: "moonlight-b",
    title: "Moonlight",
    launchPath: "D:/Games/Moonlight/game.exe",
    installDir: null,
    platformId: null,
    launchUri: null,
    fields: {},
  },
  fieldDiff: [{
    field: "description",
    current: "my note",
    incoming: "provider text",
    disposition: "preserve_user",
    willApply: false,
    currentProvenance: null,
    incomingSource: "local_scan",
  }],
};

describe("library pure feature components", () => {
  it("renders recall-only and provenance-preserving import diff", () => {
    render(ImportDiff, { props: { candidate } });
    expect(screen.getByText(/不会自动合并/)).toBeInTheDocument();
    expect(screen.getByText("preserve_user")).toBeInTheDocument();
    expect(screen.getByText("my note")).toBeInTheDocument();
  });

  it("renders health metrics without page/store coupling", () => {
    const health: LibraryHealthSnapshot = {
      state: "needs_attention",
      totalGames: 8,
      missingLaunchTargets: 1,
      duplicateIdentityGroups: 0,
      titleRecallGroups: 2,
      unresolvedImportConflicts: 1,
      provenanceCoverage: 0.75,
      issues: [{ code: "title_recall_group", severity: "info", message: "same-title games remain separate", gameIds: ["a", "b"] }],
    };
    render(LibraryHealth, { props: { health } });
    expect(screen.getByText("75%")).toBeInTheDocument();
    expect(screen.getByText("same-title games remain separate")).toBeInTheDocument();
  });
});
