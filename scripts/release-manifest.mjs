import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export function describeAsset(directory, file, version) {
  if (path.basename(file) !== file || !file.includes(version)) throw new Error(`Invalid versioned asset: ${file}`);
  const bytes = fs.readFileSync(path.join(directory, file));
  const platform = file.endsWith(".apk") ? "android" : "windows";
  const channel = platform === "android" ? (file.includes("compat") ? "compat" : "release") : file.endsWith(".msi") ? "msi" : file.endsWith(".zip") ? "portable" : "installer";
  return { file, platform, architecture: platform === "android" ? "arm64" : "x64", channel,
    size: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") };
}

export function verifyManifest(directory, expected = {}) {
  const manifest = JSON.parse(fs.readFileSync(path.join(directory, "release-manifest.json"), "utf8"));
  if (manifest.schemaVersion !== 1 || !/^\d+\.\d+\.\d+$/.test(manifest.version) || !/^[a-f0-9]{40}$/.test(manifest.commit)) throw new Error("Invalid release identity");
  if (expected.version && manifest.version !== expected.version) throw new Error("Release version does not match checked-out source");
  if (expected.commit && manifest.commit !== expected.commit) throw new Error("Release commit does not match checked-out source");
  if (!Array.isArray(manifest.assets) || !manifest.assets.length) throw new Error("No release assets");
  const seen = new Set();
  for (const asset of manifest.assets) {
    if (seen.has(asset.file)) throw new Error("Duplicate release asset");
    seen.add(asset.file);
    const actual = describeAsset(directory, asset.file, manifest.version);
    for (const field of ["sha256", "size", "platform", "channel", "architecture"]) if (actual[field] !== asset[field]) throw new Error(`Mismatch ${asset.file}: ${field}`);
  }
  for (const channel of ["installer", "msi", "portable", "release", "compat"]) if (!manifest.assets.some(a => a.channel === channel)) throw new Error(`Missing ${channel} artifact`);
  const latest = JSON.parse(fs.readFileSync(path.join(directory, "latest.json"), "utf8"));
  if (latest.version !== manifest.version || !latest.platforms?.["windows-x86_64"]?.signature) throw new Error("Missing signed Windows update metadata");
  return manifest;
}

export function generateManifest(directory, options = {}) {
  const root = path.resolve(import.meta.dirname, "..");
  const version = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
  const commit = options.commit ?? execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim();
  const files = fs.readdirSync(directory).filter(name => /\.(exe|msi|zip|apk)$/.test(name) && name.includes(version));
  const manifest = { schemaVersion: 1, version, commit, publishedAt: options.publishedAt ?? new Date().toISOString(),
    notes: ["漫画精确续读与跨页大图配对", "小说历史按书整理，章节进度独立保存", "本地阅读备份导入导出", "无界流光视觉更新"],
    androidCompatibilityVerified: options.androidCompatibilityVerified === true,
    assets: files.sort().map(file => describeAsset(directory, file, version)) };
  fs.writeFileSync(path.join(directory, "release-manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  return verifyManifest(directory);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const verify = process.argv.includes("--verify");
    const root = path.resolve(import.meta.dirname, "..");
    const version = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
    const directory = process.argv.find((arg, i) => i > 1 && !arg.startsWith("--")) ?? `artifacts/${version}`;
    const commit = execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim();
    const manifest = verify ? verifyManifest(directory, { version, commit }) : generateManifest(directory);
    console.log(`Verified v${manifest.version}: ${manifest.assets.length} artifacts, commit ${manifest.commit}`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
