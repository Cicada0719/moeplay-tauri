import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageVersion = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;

export function createUpdaterManifest({ version, artifactPath, signaturePath, repository, publishedAt = new Date().toISOString() }) {
  if (!fs.existsSync(artifactPath)) throw new Error(`Updater artifact is missing: ${artifactPath}`);
  if (!fs.existsSync(signaturePath)) throw new Error(`Updater signature is missing: ${signaturePath}`);
  const signature = fs.readFileSync(signaturePath, "utf8").trim();
  if (!signature) throw new Error(`Updater signature is empty: ${signaturePath}`);
  const assetName = path.basename(artifactPath);
  return {
    version,
    notes: `MoePlay v${version}`,
    pub_date: publishedAt,
    platforms: {
      "windows-x86_64": {
        signature,
        url: `https://github.com/${repository}/releases/download/v${version}/${encodeURIComponent(assetName)}`,
      },
    },
  };
}

export function findUpdaterArtifact(directory, version) {
  const candidates = fs.readdirSync(directory)
    .filter((name) => name.endsWith(".exe") && name.includes(version))
    .sort();
  if (candidates.length !== 1) throw new Error(`Expected one NSIS updater artifact for ${version}, found ${candidates.length}`);
  return path.join(directory, candidates[0]);
}

export function run() {
  const version = process.env.MOEPLAY_RELEASE_VERSION || packageVersion;
  const repository = process.env.GITHUB_REPOSITORY || "Cicada0719/moeplay-tauri";
  const nsisDir = path.resolve(process.env.MOEPLAY_NSIS_DIR || path.join(root, "src-tauri/target/release/bundle/nsis"));
  const output = path.resolve(process.env.MOEPLAY_UPDATER_MANIFEST || path.join(root, "latest.json"));
  const artifactPath = findUpdaterArtifact(nsisDir, version);
  const manifest = createUpdaterManifest({
    version,
    artifactPath,
    signaturePath: `${artifactPath}.sig`,
    repository,
  });
  fs.writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(output);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { run(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
