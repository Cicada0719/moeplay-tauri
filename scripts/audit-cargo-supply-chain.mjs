#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(scriptPath), "..");
const policyPath = resolve(repoRoot, "scripts", "cargo-supply-chain-policy.json");
const manifestPath = resolve(repoRoot, "src-tauri", "Cargo.toml");
const lockfilePath = resolve(repoRoot, "src-tauri", "Cargo.lock");
const denyConfigPath = resolve(repoRoot, "scripts", "cargo-deny.toml");

export function parseIsoDate(value, fieldName = "date") {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    throw new Error(`${fieldName} must use YYYY-MM-DD format`);
  }

  const parsed = new Date(`${value}T00:00:00.000Z`);
  if (Number.isNaN(parsed.valueOf()) || parsed.toISOString().slice(0, 10) !== value) {
    throw new Error(`${fieldName} is not a valid calendar date`);
  }
  return parsed;
}

export function validatePolicy(policy, now = new Date()) {
  if (!policy || policy.schemaVersion !== 1) {
    throw new Error("cargo supply-chain policy schemaVersion must be 1");
  }
  if (!/^\d+\.\d+\.\d+$/.test(policy.cargoDenyVersion ?? "")) {
    throw new Error("cargoDenyVersion must be an exact semantic version");
  }
  if (!/^[A-Za-z0-9_]+-[A-Za-z0-9_]+-[A-Za-z0-9_]+-[A-Za-z0-9_]+$/.test(policy.target ?? "")) {
    throw new Error("target must be an explicit Rust target triple");
  }
  if (!Array.isArray(policy.advisoryExceptions)) {
    throw new Error("advisoryExceptions must be an array");
  }

  const today = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()));
  const ids = new Set();
  for (const exception of policy.advisoryExceptions) {
    if (!/^RUSTSEC-\d{4}-\d{4}$/.test(exception.id ?? "")) {
      throw new Error(`invalid RustSec advisory id: ${exception.id ?? "<missing>"}`);
    }
    if (ids.has(exception.id)) {
      throw new Error(`duplicate RustSec advisory exception: ${exception.id}`);
    }
    ids.add(exception.id);

    if (typeof exception.reason !== "string" || exception.reason.trim().length < 40) {
      throw new Error(`${exception.id} must include a specific remediation reason`);
    }
    const expiry = parseIsoDate(exception.expiresOn, `${exception.id}.expiresOn`);
    if (today >= expiry) {
      throw new Error(`${exception.id} expired on ${exception.expiresOn}; update dependencies or explicitly re-review the exception`);
    }
  }

  return policy;
}

export function advisoryIdsFromDenyConfig(configText) {
  return [...configText.matchAll(/\bid\s*=\s*"(RUSTSEC-\d{4}-\d{4})"/g)].map((match) => match[1]);
}

export function assertPolicyMatchesDenyConfig(policy, configText) {
  const policyIds = policy.advisoryExceptions.map((entry) => entry.id).sort();
  const configIds = advisoryIdsFromDenyConfig(configText).sort();
  if (JSON.stringify(policyIds) !== JSON.stringify(configIds)) {
    throw new Error(`cargo-deny.toml advisory ignores do not match policy: policy=[${policyIds.join(", ")}], deny.toml=[${configIds.join(", ")}]`);
  }
  if (!configText.includes(`targets = ["${policy.target}"]`)) {
    throw new Error(`cargo-deny.toml must pin graph.targets to ${policy.target}`);
  }
}

export function parseCargoDenyVersion(output) {
  const match = /^cargo-deny\s+(\d+\.\d+\.\d+)\s*$/m.exec(output ?? "");
  return match?.[1] ?? null;
}

export function parsePackageIdentity(manifestText) {
  const packageHeader = /^\[package\]\s*$/m.exec(manifestText);
  const afterHeader = packageHeader ? manifestText.slice(packageHeader.index + packageHeader[0].length) : "";
  const nextSection = /^\[/m.exec(afterHeader);
  const packageSection = nextSection ? afterHeader.slice(0, nextSection.index) : afterHeader;
  const name = /^name\s*=\s*"([^"]+)"\s*$/m.exec(packageSection)?.[1];
  const version = /^version\s*=\s*"([^"]+)"\s*$/m.exec(packageSection)?.[1];
  if (!name || !version) {
    throw new Error("unable to read package name/version from src-tauri/Cargo.toml");
  }
  return `${name}@${version}`;
}

export function assertOnlyWorkspaceIsUnlicensed(listOutput, workspaceIdentity) {
  const match = /^Unlicensed \((\d+)\):\s*(.+)$/m.exec(listOutput ?? "");
  if (!match) {
    throw new Error("cargo-deny license inventory did not contain an Unlicensed summary");
  }
  const count = Number(match[1]);
  const entries = match[2].split(",").map((entry) => entry.trim()).filter(Boolean);
  if (count !== 1 || entries.length !== 1 || entries[0] !== workspaceIdentity) {
    throw new Error(`only the private workspace crate may lack an SPDX manifest license; found: ${entries.join(", ") || "<none>"}`);
  }
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    env: { ...process.env, CARGO_TERM_COLOR: "never" },
    stdio: options.capture ? "pipe" : "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const detail = options.capture ? `\n${result.stdout ?? ""}${result.stderr ?? ""}` : "";
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}${detail}`);
  }
  return `${result.stdout ?? ""}${result.stderr ?? ""}`;
}

function ensureCargoDeny(expectedVersion, skipInstall) {
  let versionOutput = "";
  try {
    versionOutput = run("cargo", ["deny", "--version"], { capture: true });
  } catch {
    // Installation below gives the actionable error and preserves a single path on Windows and CI.
  }
  const installedVersion = parseCargoDenyVersion(versionOutput);
  if (installedVersion === expectedVersion) {
    console.log(`[cargo-supply-chain] cargo-deny ${installedVersion} is installed`);
    return;
  }
  if (skipInstall) {
    throw new Error(`cargo-deny ${expectedVersion} is required, found ${installedVersion ?? "not installed"}`);
  }

  console.log(`[cargo-supply-chain] installing pinned cargo-deny ${expectedVersion}`);
  const installArgs = ["install", "--locked", "cargo-deny", "--version", expectedVersion];
  if (installedVersion) installArgs.push("--force");
  run("cargo", installArgs);

  const verified = parseCargoDenyVersion(run("cargo", ["deny", "--version"], { capture: true }));
  if (verified !== expectedVersion) {
    throw new Error(`cargo-deny installation mismatch: expected ${expectedVersion}, found ${verified ?? "unknown"}`);
  }
}

export function loadAndValidatePolicy(now = new Date()) {
  const policy = validatePolicy(JSON.parse(readFileSync(policyPath, "utf8")), now);
  const denyConfig = readFileSync(denyConfigPath, "utf8");
  assertPolicyMatchesDenyConfig(policy, denyConfig);
  return policy;
}

export function main(args = process.argv.slice(2)) {
  const supported = new Set(["--skip-install"]);
  for (const arg of args) {
    if (!supported.has(arg)) throw new Error(`unknown argument: ${arg}`);
  }

  const policy = loadAndValidatePolicy();
  ensureCargoDeny(policy.cargoDenyVersion, args.includes("--skip-install"));

  readFileSync(lockfilePath);
  const workspaceIdentity = parsePackageIdentity(readFileSync(manifestPath, "utf8"));
  const inventory = run("cargo", [
    "deny",
    "--manifest-path", manifestPath,
    "--config", denyConfigPath,
    "--locked",
    "list",
  ], { capture: true });
  assertOnlyWorkspaceIsUnlicensed(inventory, workspaceIdentity);
  console.log(`[cargo-supply-chain] license inventory permits no unlicensed third-party crates (${workspaceIdentity} is the sole private workspace exception)`);

  run("cargo", [
    "deny",
    "--manifest-path", manifestPath,
    "--config", denyConfigPath,
    "--locked",
    "check",
    "advisories",
    "licenses",
    "-D", "warnings",
    "-A", "unlicensed",
    "-A", "no-license-field",
    "--show-stats",
  ]);

  console.log(`[cargo-supply-chain] OK: advisory and license gates passed for ${policy.target}`);
}

const invokedDirectly = process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
if (invokedDirectly) {
  try {
    main();
  } catch (error) {
    console.error(`[cargo-supply-chain] ERROR: ${error.message}`);
    process.exitCode = 1;
  }
}
