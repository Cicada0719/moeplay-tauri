import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDir, "..");

function readJson(relative) {
  return JSON.parse(fs.readFileSync(path.join(root, relative), "utf8"));
}

function read(relative) {
  return fs.readFileSync(path.join(root, relative), "utf8");
}

function capture(text, regex, label) {
  const match = text.match(regex);
  if (!match) throw new Error(`Unable to read ${label}`);
  return match[1];
}

const pkg = readJson("package.json");
const lock = readJson("package-lock.json");
const tauri = readJson("src-tauri/tauri.conf.json");
const cargoToml = read("src-tauri/Cargo.toml");
const cargoLock = read("src-tauri/Cargo.lock");
const appVersionSource = read("src/lib/app-version.ts");
const settingsPageSource = read("src/lib/components/SettingsPage.svelte");

const versions = new Map([
  ["package.json", pkg.version],
  ["package-lock.json root", lock.version],
  ["package-lock.json packages['']", lock.packages?.[""]?.version],
  ["tauri.conf.json", tauri.version],
  ["Cargo.toml", capture(cargoToml, /^version\s*=\s*"([^"]+)"/m, "Cargo.toml package version")],
  ["Cargo.lock moeplay", capture(cargoLock, /\[\[package\]\]\s*\nname = "moeplay"\s*\nversion = "([^"]+)"/m, "Cargo.lock moeplay version")],
]);

const expected = pkg.version;
const mismatches = [...versions].filter(([, value]) => value !== expected);


function walkFiles(directory, files = []) {
  if (!fs.existsSync(directory)) return files;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) walkFiles(full, files);
    else if (entry.isFile()) files.push(full);
  }
  return files;
}

const staleRuntimeUserAgents = [];
for (const file of walkFiles(path.join(root, "src-tauri", "src"))) {
  if (!/\.rs$/.test(file)) continue;
  const text = fs.readFileSync(file, "utf8");
  for (const match of text.matchAll(/(?:MoePlay|MoeGame|moeplay)\/\d+\.\d+(?:\.\d+)?/g)) {
    staleRuntimeUserAgents.push(`${path.relative(root, file)}:${text.slice(0, match.index).split("\n").length}:${match[0]}`);
  }
}

const explicitExpected = process.argv[2]
  ?? process.env.MOEPLAY_EXPECTED_VERSION
  ?? (process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME?.replace(/^v/, "") : undefined);

if (explicitExpected && expected !== explicitExpected) {
  mismatches.push(["expected version", explicitExpected]);
}

if (staleRuntimeUserAgents.length) {
  for (const value of staleRuntimeUserAgents) mismatches.push(["hard-coded runtime User-Agent", value]);
}

if (!appVersionSource.includes('from "../../package.json"') || !appVersionSource.includes("export const APP_VERSION")) {
  mismatches.push(["settings version source", "src/lib/app-version.ts must derive APP_VERSION from package.json"]);
}
if (!settingsPageSource.includes("v{appVersion}") || /v\d+\.\d+\.\d+/.test(settingsPageSource)) {
  mismatches.push(["settings version display", "SettingsPage must render the synchronized appVersion value without a hard-coded release"]);
}

if (mismatches.length) {
  console.error(`Version consistency check failed. package.json=${expected}`);
  for (const [source, value] of versions) {
    console.error(`  ${source}: ${value ?? "<missing>"}`);
  }
  if (explicitExpected) console.error(`  expected: ${explicitExpected}`);
  process.exit(1);
}

console.log(`Version consistency OK: ${expected}`);
for (const [source, value] of versions) console.log(`  ${source}: ${value}`);
