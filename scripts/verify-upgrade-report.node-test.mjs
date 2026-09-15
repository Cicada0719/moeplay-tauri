import assert from "node:assert/strict";
import test from "node:test";
import { validateUpgradeReport } from "./verify-upgrade-report.mjs";

const valid = {
  fromVersion: "0.23.1", toVersion: "0.24.0", schemaVersion: 4,
  candidateManifestSha256: "a".repeat(64), environment: "Windows 11 x64",
  packageName: "com.moeplay.app", versionCode: 24000,
  certificateFingerprint: "b".repeat(64), apkSha256: "c".repeat(64),
  device: "Pixel C", dataCheck: { history: true, library: true }, channel: "compat", coverageUpgrade: true,
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

test("uses caller supplied candidate versions and APK evidence", () => {
  assert.deepEqual(validateUpgradeReport(valid, {
    candidate: {
      fromVersion: "0.23.1", toVersion: "0.24.0", packageName: "com.moeplay.app",
      versionCode: 24000, certificateFingerprint: "b".repeat(64), apkSha256: "c".repeat(64),
      channel: "compat", fromVersionCode: 23001, device: "Pixel C",
    },
  }), []);
  assert.ok(validateUpgradeReport(valid, { toVersion: "0.24.1" }).some((failure) => failure.includes("toVersion")));
  assert.ok(validateUpgradeReport({ ...valid, apkSha256: "d".repeat(64) }, { apkSha256: "c".repeat(64) }).some((failure) => failure.includes("apkSha256")));
  assert.ok(validateUpgradeReport({ ...valid, versionCode: 23001 }, { fromVersionCode: 23001 }).some((failure) => failure.includes("greater")));
});
