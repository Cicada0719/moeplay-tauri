import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const expectedVersion = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;

function walk(directory, matches = []) {
  if (!fs.existsSync(directory)) return matches;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(full, matches);
    else if (entry.isFile() && entry.name === "latest.json") matches.push(full);
  }
  return matches;
}

const explicit = process.argv[2] ? [path.resolve(process.argv[2])] : [];
const candidates = explicit.length
  ? explicit
  : walk(path.join(root, "src-tauri", "target", "release"));

if (!candidates.length) {
  console.error("Updater verification failed: latest.json was not found under src-tauri/target/release");
  process.exit(1);
}

let verified = 0;
for (const file of candidates) {
  const data = JSON.parse(fs.readFileSync(file, "utf8"));
  if (data.version !== expectedVersion) {
    console.error(`${file}: version=${data.version ?? "<missing>"}, expected=${expectedVersion}`);
    process.exitCode = 1;
    continue;
  }
  const platforms = data.platforms && typeof data.platforms === "object"
    ? Object.entries(data.platforms)
    : [];
  if (!platforms.length) {
    console.error(`${file}: platforms is empty`);
    process.exitCode = 1;
    continue;
  }
  for (const [platform, artifact] of platforms) {
    if (!artifact || typeof artifact !== "object") throw new Error(`${file}: invalid platform ${platform}`);
    const { url, signature } = artifact;
    if (typeof url !== "string" || !url.startsWith("https://")) {
      throw new Error(`${file}: ${platform} URL must be HTTPS`);
    }
    if (typeof signature !== "string" || signature.trim().length < 32) {
      throw new Error(`${file}: ${platform} signature is missing or unexpectedly short`);
    }
  }
  console.log(`Updater manifest OK: ${path.relative(root, file)} (${platforms.length} platform entries)`);
  verified += 1;
}

if (process.exitCode) process.exit(process.exitCode);
if (!verified) process.exit(1);
