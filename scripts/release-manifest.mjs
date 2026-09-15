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

function androidVerificationFor(manifest) {
  return manifest.androidVerification ?? manifest.androidUpgradeReports ?? manifest.androidReports ?? null;
}

function verifyAndroidVerification(manifest) {
  if (manifest.androidCompatibilityVerified !== true) return;
  const verification = androidVerificationFor(manifest);
  if (!verification || typeof verification !== "object") throw new Error("Android compatibility is marked verified without upgrade evidence");
  for (const channel of ["release", "compat"]) {
    const evidence = verification[channel];
    if (!evidence || typeof evidence !== "object") throw new Error(`Missing Android ${channel} upgrade evidence`);
    const report = evidence.report && typeof evidence.report === "object" ? evidence.report : evidence;
    if (report.channel !== channel && report.variant !== channel) throw new Error(`Android ${channel} evidence has the wrong channel`);
    if (report.coverageUpgrade !== true && report.coveragePassed !== true) throw new Error(`Android ${channel} coverage was not successful`);
    const apkSha256 = report.apkSha256 ?? report.apkSHA256 ?? evidence.apkSha256;
    if (!/^[0-9a-f]{64}$/i.test(apkSha256 ?? "")) throw new Error(`Android ${channel} evidence is missing APK SHA-256`);
    const asset = manifest.assets.find((entry) => entry.channel === channel && entry.platform === "android");
    if (!asset || asset.sha256.toLowerCase() !== apkSha256.toLowerCase()) throw new Error(`Android ${channel} evidence does not match the release APK`);
    const packageName = report.packageName ?? report.package;
    if (typeof packageName !== "string" || !packageName.trim()) throw new Error(`Android ${channel} evidence is missing package name`);
    if (!Number.isInteger(report.versionCode) && !Number.isInteger(report.toVersionCode)) throw new Error(`Android ${channel} evidence is missing versionCode`);
    const certificate = report.certificateFingerprint ?? report.certificateSha256 ?? report.certFingerprint;
    if (!/^(?:[0-9a-f]{2}:){15}[0-9a-f]{2}$|^[0-9a-f]{64}$/i.test(certificate ?? "")) throw new Error(`Android ${channel} evidence is missing certificate fingerprint`);
    if (!report.device && !evidence.device) throw new Error(`Android ${channel} evidence is missing device`);
    const checks = report.dataCheck ?? report.dataChecks;
    if (!(report.dataCheck === true || report.dataPreserved === true || (checks && typeof checks === "object" && Object.keys(checks).length > 0 && Object.values(checks).every((check) => check === true || check?.passed === true)))) throw new Error(`Android ${channel} evidence is missing data checks`);
  }
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
  verifyAndroidVerification(manifest);
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
    notes: options.notes ?? ["电脑端使用系统输入法和实体键盘，不再弹出应用虚拟键盘", "修复隐藏视频解析窗口发声和退出后残留音频", "以实际视频帧检测黑屏与卡流，支持重试和换源", "修复快速切源旧请求覆盖与 Provider v2 切源", "画质增强失败自动恢复原始视频，便携包补齐内置规则", "保留 v0.23 漫画精确续读、小说按书历史和阅读备份"],
    androidCompatibilityVerified: options.androidCompatibilityVerified === true,
    assets: files.sort().map(file => describeAsset(directory, file, version)) };
  if (options.androidVerification ?? options.androidUpgradeReports ?? options.androidReports) {
    manifest.androidVerification = options.androidVerification ?? options.androidUpgradeReports ?? options.androidReports;
  }
  fs.writeFileSync(path.join(directory, "release-manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  return verifyManifest(directory);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const verify = process.argv.includes("--verify");
    const root = path.resolve(import.meta.dirname, "..");
    const version = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
    let directory;
    let explicitCommit;
    for (let index = 2; index < process.argv.length; index += 1) {
      const arg = process.argv[index];
      if (arg === "--verify") continue;
      if (arg === "--commit") {
        explicitCommit = process.argv[++index];
        if (explicitCommit === undefined || explicitCommit.startsWith("-")) throw new Error("--commit requires a 40-character Git SHA");
        continue;
      }
      if (arg.startsWith("-")) throw new Error(`Unknown option: ${arg}`);
      if (directory) throw new Error("Only one artifact directory may be supplied");
      directory = arg;
    }
    directory ??= `artifacts/${version}`;
    const commit = explicitCommit
      ?? execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim();
    if (explicitCommit !== undefined && !/^[a-f0-9]{40}$/i.test(explicitCommit)) throw new Error("--commit must be a 40-character Git SHA");
    const manifest = verify ? verifyManifest(directory, { version, commit }) : generateManifest(directory, { commit });
    console.log(`Verified v${manifest.version}: ${manifest.assets.length} artifacts, commit ${manifest.commit}`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
