import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { verifyManifest } from "./release-manifest.mjs";
import { runCli as verifyUpdater } from "./verify-updater-artifacts.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const VERSION_RE = /^\d+\.\d+\.\d+$/;
const SHA_RE = /^[a-f0-9]{40}$/i;

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
}

function cargoVersion() {
  const cargo = fs.readFileSync(path.join(root, "src-tauri", "Cargo.toml"), "utf8");
  const match = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  return match?.[1];
}

function cargoLockVersion() {
  const lock = fs.readFileSync(path.join(root, "src-tauri", "Cargo.lock"), "utf8");
  const match = lock.match(/\[\[package\]\]\s*\nname = "moeplay"\s*\nversion = "([^"]+)"/m);
  return match?.[1];
}

/** Read the identity that every release artifact must carry. */
export function readReleaseIdentity() {
  const packageJson = readJson("package.json");
  const lock = readJson("package-lock.json");
  const tauri = readJson("src-tauri/tauri.conf.json");
  return {
    version: packageJson.version,
    files: {
      "package.json": packageJson.version,
      "package-lock.json": lock.version,
      "package-lock.json packages['']": lock.packages?.[""]?.version,
      "src-tauri/tauri.conf.json": tauri.version,
      "src-tauri/Cargo.toml": cargoVersion(),
      "src-tauri/Cargo.lock": cargoLockVersion(),
    },
  };
}

export function validateReleaseIdentity(expectedVersion, identity = readReleaseIdentity()) {
  if (!VERSION_RE.test(expectedVersion)) throw new Error(`Invalid release version: ${expectedVersion}`);
  const mismatches = Object.entries(identity.files).filter(([, value]) => value !== expectedVersion);
  if (mismatches.length) {
    throw new Error(`Release version mismatch: ${mismatches.map(([file, value]) => `${file}=${value ?? "<missing>"}`).join(", ")}`);
  }
  return { version: expectedVersion, files: identity.files };
}

function check(id, status, detail, evidence = undefined) {
  return { id, status, detail, ...(evidence === undefined ? {} : { evidence }) };
}

function readEvidence(file) {
  if (!file) return {};
  const data = JSON.parse(fs.readFileSync(path.resolve(file), "utf8"));
  if (!data || typeof data !== "object" || Array.isArray(data)) throw new Error("Evidence must be a JSON object");
  return data;
}

export function checkRollback(releaseRoot, version, previousVersion) {
  if (!releaseRoot || !previousVersion) {
    return check("rollback", "pending", "未提供发布根目录和前一版本，尚未检查旧版本回滚目录");
  }
  if (!VERSION_RE.test(previousVersion) || previousVersion === version) {
    return check("rollback", "failed", "前一版本必须是不同的语义化版本");
  }
  const base = path.resolve(releaseRoot);
  const paths = [path.join(base, "sites", previousVersion), path.join(base, "downloads", previousVersion)];
  const missing = paths.filter((entry) => !fs.existsSync(entry));
  return missing.length
    ? check("rollback", "failed", `缺少旧版本目录：${missing.join(", ")}`, { previousVersion, paths })
    : check("rollback", "passed", "旧版本站点和下载目录仍可用于回滚", { previousVersion, paths });
}

export function inspectArtifactBatch(directory, expectedVersion, expectedCommit) {
  if (!directory) return check("release-artifacts", "pending", "未提供本地产物目录，尚未绑定 Windows/Android 同批文件");
  const resolved = path.resolve(directory);
  if (!fs.existsSync(resolved)) return check("release-artifacts", "pending", `产物目录不存在：${resolved}`);
  try {
    const manifest = verifyManifest(resolved, { version: expectedVersion, commit: expectedCommit });
    const latest = path.join(resolved, "latest.json");
    let updater = null;
    if (fs.existsSync(latest)) {
      updater = verifyUpdater(["--require", "--artifacts-dir", resolved, latest], { searchRoots: [] });
    }
    return check("release-artifacts", "passed", "发布清单与同批文件的版本、提交 SHA、大小和 SHA-256 一致", {
      directory: resolved,
      commit: manifest.commit,
      assets: manifest.assets.map((asset) => ({ file: asset.file, channel: asset.channel, bytes: asset.size, sha256: asset.sha256 })),
      updater: updater?.status ?? "not-generated",
    });
  } catch (error) {
    return check("release-artifacts", "failed", error.message, { directory: resolved });
  }
}

/** Probe a public or LAN URL without treating a successful HTML response as a valid download. */
export async function probeDownloadEndpoint(url, fetchImpl = globalThis.fetch) {
  if (!url) return check("download-endpoint", "pending", "未提供公网或局域网地址");
  let parsed;
  try { parsed = new URL(url); } catch { return check("download-endpoint", "failed", `地址无效：${url}`); }
  if (!fetchImpl) return check("download-endpoint", "pending", "运行环境没有 fetch，未执行真实下载检查", { url });
  try {
    const response = await fetchImpl(parsed, {
      method: "GET",
      headers: { Range: "bytes=0-0", "Cache-Control": "no-cache" },
      cache: "no-store",
    });
    const range = response.headers?.get?.("content-range") ?? "";
    const cache = response.headers?.get?.("cache-control") ?? "";
    const isLatest = parsed.pathname.endsWith("latest.json");
    if (!response.ok || response.status !== 206 || !/^bytes\s+0-0\//i.test(range)) {
      return check("download-endpoint", "failed", `下载端点未返回 HTTP 206：${response.status}`, { url, status: response.status, contentRange: range });
    }
    if (isLatest && !/no-store/i.test(cache)) {
      return check("download-endpoint", "failed", "latest.json 缺少 Cache-Control: no-store", { url, cacheControl: cache });
    }
    return check("download-endpoint", "passed", "真实端点支持断点下载并返回正确缓存策略", { url, status: response.status, contentRange: range, cacheControl: cache });
  } catch (error) {
    return check("download-endpoint", "pending", `真实端点暂时不可达：${error.message}`, { url });
  }
}

const REQUIRED_EVIDENCE = [
  ["playback-regression", "播放首帧、持续播放、快进、切集、换源、后台恢复和退出音频回归"],
  ["following-regression", "追番首次基线、增量提醒、未看数、已看和同季隔离回归"],
  ["offline-regression", "离线漫画/小说下载、中断、断网重启、删除、空间统计和续读回归"],
  ["history-regression", "三类历史导入导出、坏记录、重复导入和存储故障回归"],
  ["sync-regression", "WebDAV 条件写入、并发冲突和同步中新增记录回归"],
  ["desktop-install", "Windows 安装与升级冒烟"],
  ["android-upgrade", "Android 正式包冷启动、兼容包覆盖升级和手柄回归"],
];

function evidenceChecks(evidence) {
  return REQUIRED_EVIDENCE.map(([id, description]) => {
    const item = evidence[id];
    if (!item) return check(id, "pending", `${description}：未提供真实执行证据`);
    if (item.status !== "passed" && item.status !== "failed") return check(id, "failed", `${id} 的证据状态必须是 passed 或 failed`);
    if (!item.command || !item.ranAt) return check(id, "failed", `${id} 缺少 command 或 ranAt，不能作为验收证据`);
    return check(id, item.status, item.detail || description, item);
  });
}

export async function buildAcceptanceReport(options = {}) {
  const identity = readReleaseIdentity();
  const version = options.version ?? identity.version;
  const commit = options.commit ?? execFileSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8" }).trim();
  if (!SHA_RE.test(commit)) throw new Error(`Invalid release commit: ${commit}`);
  const checks = [];
  try {
    checks.push(check("version-consistency", "passed", "版本文件一致", validateReleaseIdentity(version, identity)));
  } catch (error) {
    checks.push(check("version-consistency", "failed", error.message, identity));
  }
  checks.push(...evidenceChecks(options.evidence ?? {}));
  checks.push(inspectArtifactBatch(options.artifactsDir, version, commit));
  checks.push(checkRollback(options.releaseRoot, version, options.previousVersion));
  if (options.publicUrl) checks.push(await probeDownloadEndpoint(options.publicUrl, options.fetchImpl));
  else checks.push(check("public-download", "pending", "未提供公网地址，未执行真实下载检查"));
  if (options.lanUrl) checks.push(await probeDownloadEndpoint(options.lanUrl, options.fetchImpl));
  else checks.push(check("lan-download", "pending", "未提供局域网地址，未执行真实下载检查"));
  return {
    schemaVersion: 1,
    product: "MoePlay",
    version,
    commit,
    generatedAt: new Date().toISOString(),
    checks,
    summary: {
      passed: checks.filter((item) => item.status === "passed").length,
      pending: checks.filter((item) => item.status === "pending").length,
      failed: checks.filter((item) => item.status === "failed").length,
    },
  };
}

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--version") options.version = argv[++index];
    else if (arg === "--commit") options.commit = argv[++index];
    else if (arg === "--artifacts-dir") options.artifactsDir = argv[++index];
    else if (arg === "--release-root") options.releaseRoot = argv[++index];
    else if (arg === "--previous-version") options.previousVersion = argv[++index];
    else if (arg === "--evidence") options.evidence = readEvidence(argv[++index]);
    else if (arg === "--public-url") options.publicUrl = argv[++index];
    else if (arg === "--lan-url") options.lanUrl = argv[++index];
    else if (arg === "--report") options.report = path.resolve(argv[++index]);
    else if (arg === "--strict") options.strict = true;
    else throw new Error(`Unknown option: ${arg}`);
  }
  return options;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  buildAcceptanceReport(parseArgs(process.argv.slice(2))).then((report) => {
    if (parseArgs(process.argv.slice(2)).strict && report.summary.pending + report.summary.failed > 0) {
      throw new Error(`验收未完成：${report.summary.failed} failed, ${report.summary.pending} pending`);
    }
    const reportPath = parseArgs(process.argv.slice(2)).report;
    const output = JSON.stringify(report, null, 2) + "\n";
    if (reportPath) {
      fs.mkdirSync(path.dirname(reportPath), { recursive: true });
      fs.writeFileSync(reportPath, output);
    }
    process.stdout.write(output);
  }).catch((error) => {
    console.error(`Acceptance report failed: ${error.message}`);
    process.exitCode = 1;
  });
}
