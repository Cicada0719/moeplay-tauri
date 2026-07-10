import assert from "node:assert/strict";
import test from "node:test";
import { validateUpgradeReport } from "./verify-upgrade-report.mjs";

const valid = {
  fromVersion: "0.12.0", toVersion: "0.12.1", schemaVersion: 4,
  candidateManifestSha256: "a".repeat(64), environment: "Windows 11 x64",
  installed: true, launched: true, migrationLedgerVerified: true, libraryPreserved: true,
  activityPreserved: true, saveSnapshotsPreserved: true, secretReferencesPreserved: true,
  tasksPreserved: true, providerRestartRecoveryVerified: true, diagnosticRedactionVerified: true,
  rollbackRecoveryVerified: true, updaterSignatureVerified: true,
  authenticodeDecision: "documented_exception", passed: true,
  operator: "release-owner", completedAt: "2026-07-10T12:00:00Z",
};

test("accepts a complete installed upgrade report", () => assert.deepEqual(validateUpgradeReport(valid), []));
test("rejects incomplete or falsely passing reports", () => {
  const failures = validateUpgradeReport({ ...valid, libraryPreserved: false, operator: "", passed: false });
  assert.ok(failures.some((failure) => failure.includes("libraryPreserved")));
  assert.ok(failures.some((failure) => failure.includes("passed")));
  assert.ok(failures.some((failure) => failure.includes("operator")));
});
