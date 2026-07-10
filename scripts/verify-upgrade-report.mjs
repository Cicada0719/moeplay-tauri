import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

export function validateUpgradeReport(report) {
  const failures = [];
  if (report?.fromVersion !== "0.12.0") failures.push("fromVersion must be 0.12.0");
  if (report?.toVersion !== "0.12.1") failures.push("toVersion must be 0.12.1");
  if (report?.schemaVersion !== 4) failures.push("schemaVersion must be 4");
  if (!/^[0-9a-f]{64}$/i.test(report?.candidateManifestSha256 ?? "")) failures.push("candidateManifestSha256 must be SHA-256");
  for (const field of [
    "installed", "launched", "migrationLedgerVerified", "libraryPreserved",
    "activityPreserved", "saveSnapshotsPreserved", "secretReferencesPreserved",
    "tasksPreserved", "providerRestartRecoveryVerified", "diagnosticRedactionVerified",
    "rollbackRecoveryVerified", "updaterSignatureVerified", "passed",
  ]) if (report?.[field] !== true) failures.push(`${field} must be true`);
  if (!new Set(["verified", "documented_exception"]).has(report?.authenticodeDecision)) failures.push("authenticodeDecision is invalid");
  if (typeof report?.operator !== "string" || !report.operator.trim()) failures.push("operator is required");
  if (!Number.isFinite(Date.parse(report?.completedAt ?? ""))) failures.push("completedAt must be RFC3339-compatible");
  return failures;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const file = process.argv[2];
  if (!file) {
    console.error("Usage: node scripts/verify-upgrade-report.mjs <report.json>");
    process.exit(2);
  }
  const report = JSON.parse(fs.readFileSync(path.resolve(file), "utf8"));
  const failures = validateUpgradeReport(report);
  if (failures.length) {
    console.error("Installed upgrade report FAILED:");
    for (const failure of failures) console.error(`- ${failure}`);
    process.exit(1);
  }
  console.log(`Installed upgrade report OK: ${report.fromVersion} -> ${report.toVersion}`);
}
