import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";

const VERSION_RE = /^\d+\.\d+\.\d+$/;
const SHA256_RE = /^[0-9a-f]{64}$/i;
const FINGERPRINT_RE = /^(?:[0-9a-f]{2}:){15}[0-9a-f]{2}$|^[0-9a-f]{64}$/i;

function first(...values) {
  return values.find((value) => value !== undefined && value !== null);
}

function asNonEmptyString(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function asEvidence(report) {
  const android = report?.android && typeof report.android === "object" ? report.android : {};
  return {
    packageName: first(report?.packageName, report?.package, android.packageName, android.package),
    versionCode: first(report?.versionCode, report?.toVersionCode, android.versionCode),
    certificateFingerprint: first(
      report?.certificateFingerprint, report?.certificateSha256, report?.certFingerprint,
      android.certificateFingerprint, android.certificateSha256, android.certFingerprint,
    ),
    apkSha256: first(report?.apkSha256, report?.apkSHA256, report?.apkHash, android.apkSha256, android.apkSHA256),
    device: first(report?.device, report?.deviceModel, android.device, android.deviceModel),
    dataCheck: first(report?.dataCheck, report?.dataChecks, report?.dataCheckResults, report?.dataVerification, report?.dataPreserved, android.dataCheck, android.dataChecks, android.dataCheckResults),
    channel: first(report?.channel, report?.variant, android.channel),
    coveragePassed: first(
      report?.coverageUpgrade, report?.coveragePassed, report?.coverage?.passed, report?.covered,
      android.coverageUpgrade, android.coveragePassed, android.coverage?.passed, android.covered,
    ),
  };
}

function dataCheckPassed(value) {
  if (value === true) return true;
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const checks = Object.values(value);
  return checks.length > 0 && checks.every((check) => check === true || (check && check.passed === true));
}

function expectedValue(options, ...keys) {
  for (const key of keys) if (options?.[key] !== undefined) return options[key];
  return undefined;
}

/**
 * Validate an installed-upgrade report against the candidate supplied by the
 * caller. No historical version is assumed; callers may pass expected values
 * directly or via a candidate object.
 */
export function validateUpgradeReport(report, options = {}) {
  const failures = [];
  const candidate = options.candidate && typeof options.candidate === "object" ? options.candidate : {};
  const expectedFrom = first(expectedValue(options, "fromVersion", "expectedFromVersion", "previousVersion"), candidate.fromVersion, candidate.previousVersion);
  const expectedTo = first(expectedValue(options, "toVersion", "expectedToVersion", "candidateVersion", "expectedVersion"), candidate.toVersion, candidate.version, candidate.candidateVersion);
  const fromVersion = report?.fromVersion;
  const toVersion = report?.toVersion;
  if (!VERSION_RE.test(fromVersion ?? "")) failures.push("fromVersion must be a semantic version");
  if (!VERSION_RE.test(toVersion ?? "")) failures.push("toVersion must be a semantic version");
  if (expectedFrom !== undefined && fromVersion !== expectedFrom) failures.push(`fromVersion must be ${expectedFrom}`);
  if (expectedTo !== undefined && toVersion !== expectedTo) failures.push(`toVersion must be ${expectedTo}`);
  if (report?.schemaVersion !== 4) failures.push("schemaVersion must be 4");
  if (!SHA256_RE.test(report?.candidateManifestSha256 ?? "")) failures.push("candidateManifestSha256 must be SHA-256");
  const expectedManifestSha256 = first(expectedValue(options, "candidateManifestSha256", "expectedCandidateManifestSha256"), candidate.candidateManifestSha256, candidate.manifestSha256);
  if (expectedManifestSha256 !== undefined && String(report.candidateManifestSha256 ?? "").toLowerCase() !== String(expectedManifestSha256).toLowerCase()) failures.push("candidateManifestSha256 does not match candidate");

  const evidence = asEvidence(report);
  const expectedPackage = first(expectedValue(options, "package", "packageName", "expectedPackage"), candidate.package, candidate.packageName, candidate.android?.package, candidate.android?.packageName);
  const expectedVersionCode = first(expectedValue(options, "versionCode", "toVersionCode", "expectedVersionCode"), candidate.versionCode, candidate.toVersionCode, candidate.android?.versionCode);
  const expectedCertificate = first(expectedValue(options, "certificate", "certificateFingerprint", "certificateSha256", "expectedCertificateFingerprint"), candidate.certificateFingerprint, candidate.certificateSha256, candidate.android?.certificateFingerprint, candidate.android?.certificateSha256);
  const expectedApkSha256 = first(expectedValue(options, "apkSha256", "expectedApkSha256"), candidate.apkSha256, candidate.apkSHA256, candidate.android?.apkSha256, candidate.android?.apkSHA256);
  const expectedChannel = first(expectedValue(options, "channel", "variant", "expectedChannel"), candidate.channel, candidate.variant);
  const expectedDevice = first(expectedValue(options, "device", "deviceModel", "expectedDevice"), candidate.device, candidate.deviceModel, candidate.android?.device, candidate.android?.deviceModel);

  if (!asNonEmptyString(evidence.packageName)) failures.push("packageName is required");
  if (expectedPackage !== undefined && evidence.packageName !== expectedPackage) failures.push("packageName does not match candidate");
  if (!Number.isInteger(evidence.versionCode) || evidence.versionCode < 1) failures.push("versionCode must be a positive integer");
  if (expectedVersionCode !== undefined && evidence.versionCode !== Number(expectedVersionCode)) failures.push("versionCode does not match candidate");
  if (!FINGERPRINT_RE.test(evidence.certificateFingerprint ?? "")) failures.push("certificateFingerprint must be SHA-256 or colon fingerprint");
  if (expectedCertificate !== undefined && String(evidence.certificateFingerprint ?? "").toLowerCase() !== String(expectedCertificate).toLowerCase()) failures.push("certificateFingerprint does not match candidate");
  if (!SHA256_RE.test(evidence.apkSha256 ?? "")) failures.push("apkSha256 must be SHA-256");
  if (expectedApkSha256 !== undefined && String(evidence.apkSha256 ?? "").toLowerCase() !== String(expectedApkSha256).toLowerCase()) failures.push("apkSha256 does not match candidate");
  if (expectedDevice !== undefined && JSON.stringify(evidence.device) !== JSON.stringify(expectedDevice)) failures.push("device does not match candidate");
  if (!asNonEmptyString(evidence.device) && !(evidence.device && typeof evidence.device === "object")) failures.push("device is required");
  if (!dataCheckPassed(evidence.dataCheck)) failures.push("dataCheck must contain passing checks");
  if (!new Set(["release", "compat"]).has(evidence.channel)) failures.push("channel must be release or compat");
  if (expectedChannel !== undefined && evidence.channel !== expectedChannel) failures.push("channel does not match candidate");
  if (evidence.coveragePassed !== true) failures.push("coverageUpgrade must be true");

  const fromCode = first(options.fromVersionCode, candidate.fromVersionCode, candidate.android?.fromVersionCode, report?.fromVersionCode, report?.android?.fromVersionCode);
  if (fromCode !== undefined && (!Number.isInteger(Number(fromCode)) || evidence.versionCode <= Number(fromCode))) failures.push("versionCode must be greater than the installed versionCode");

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

export function sha256File(filePath) {
  return createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const file = args.shift();
  if (!file) {
    console.error("Usage: node scripts/verify-upgrade-report.mjs <report.json> [--from-version x.y.z] [--to-version x.y.z] [--candidate candidate.json] [--apk file.apk]");
    process.exit(2);
  }
  const report = JSON.parse(fs.readFileSync(path.resolve(file), "utf8"));
  const options = {};
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    const value = args[++index];
    if (arg === "--from-version") options.fromVersion = value;
    else if (arg === "--to-version" || arg === "--candidate-version") options.toVersion = value;
    else if (arg === "--from-version-code") options.fromVersionCode = Number(value);
    else if (arg === "--candidate") options.candidate = JSON.parse(fs.readFileSync(path.resolve(value), "utf8"));
    else if (arg === "--apk") options.apkSha256 = sha256File(path.resolve(value));
    else if (arg === "--package") options.packageName = value;
    else if (arg === "--version-code") options.versionCode = Number(value);
    else if (arg === "--certificate") options.certificateFingerprint = value;
    else if (arg === "--channel") options.channel = value;
    else if (arg === "--device") options.device = value;
    else throw new Error(`Unknown option: ${arg}`);
  }
  const failures = validateUpgradeReport(report, options);
  if (failures.length) {
    console.error("Installed upgrade report FAILED:");
    for (const failure of failures) console.error(`- ${failure}`);
    process.exit(1);
  }
  console.log(`Installed upgrade report OK: ${report.fromVersion} -> ${report.toVersion}`);
}
