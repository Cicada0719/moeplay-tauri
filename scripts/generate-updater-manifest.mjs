import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageVersion = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;

export function canonicalUpdaterAssetName(version) {
  return `MoeGame_${version}_x64-setup.exe`;
}

export function createUpdaterManifest({ version, artifactPath, signaturePath, repository, publishedAt = new Date().toISOString(), assetName = path.basename(artifactPath) }) {
  if (!fs.existsSync(artifactPath)) throw new Error(`Updater artifact is missing: ${artifactPath}`);
  if (!fs.existsSync(signaturePath)) throw new Error(`Updater signature is missing: ${signaturePath}`);
  const signature = fs.readFileSync(signaturePath, "utf8").trim();
  if (!signature) throw new Error(`Updater signature is empty: ${signaturePath}`);
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
  const canonical = canonicalUpdaterAssetName(version);
  const candidates = fs.readdirSync(directory)
    .filter((name) => name.endsWith(".exe") && name.includes(version) && fs.existsSync(path.join(directory, `${name}.sig`)))
    .sort();
  if (candidates.includes(canonical)) return path.join(directory, canonical);
  if (candidates.length !== 1) throw new Error(`Expected one signed NSIS updater artifact for ${version}, found ${candidates.length}`);
  return path.join(directory, candidates[0]);
}

export function prepareCanonicalUpdaterArtifact(directory, version) {
  const source = findUpdaterArtifact(directory, version);
  const target = path.join(directory, canonicalUpdaterAssetName(version));
  if (path.resolve(source) !== path.resolve(target)) {
    fs.copyFileSync(source, target);
    fs.copyFileSync(`${source}.sig`, `${target}.sig`);
  }
  return target;
}

export function run() {
  const version = process.env.MOEPLAY_RELEASE_VERSION || packageVersion;
  const repository = process.env.GITHUB_REPOSITORY || "Cicada0719/moeplay-tauri";
  const nsisDir = path.resolve(process.env.MOEPLAY_NSIS_DIR || path.join(root, "src-tauri/target/release/bundle/nsis"));
  const output = path.resolve(process.env.MOEPLAY_UPDATER_MANIFEST || path.join(root, "latest.json"));
  const artifactPath = prepareCanonicalUpdaterArtifact(nsisDir, version);
  const manifest = createUpdaterManifest({
    version,
    artifactPath,
    signaturePath: `${artifactPath}.sig`,
    repository,
    assetName: canonicalUpdaterAssetName(version),
  });
  fs.writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(output);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { run(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
