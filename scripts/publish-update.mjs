// 一条命令把 Windows 安装包发布到局域网更新服务器：
//   定位 NSIS 包 →（缺签名则用 tauri signer 补签）→ 生成 latest.json → 原子上传服务器。
// 用法：
//   node scripts/publish-update.mjs [--dir src-tauri/target/release/bundle/nsis] [--version x.y.z]
//        [--notes "文本"] [--notes-file 路径] [--config 路径] [--dry-run]
// 配置文件（默认 %USERPROFILE%\.tauri\moeplay-publish-config.json）：
//   { privateKeyPath, password, serverBaseUrl, publishToken }
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import os from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { canonicalUpdaterAssetName } from "./generate-updater-manifest.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DEFAULT_CONFIG = path.join(os.homedir(), ".tauri", "moeplay-publish-config.json");

export function findUpdaterArtifact(nsisDir, version) {
  const candidates = fs.readdirSync(nsisDir)
    .filter((name) => name.endsWith(".exe") && name.includes(version) && !name.endsWith(".tmp.exe"))
    .sort();
  const canonical = canonicalUpdaterAssetName(version);
  if (candidates.includes(canonical)) return path.join(nsisDir, canonical);
  if (candidates.length !== 1) throw new Error(`期望在 ${nsisDir} 中找到恰好一个包含版本号 ${version} 的安装包，实际 ${candidates.length} 个：${candidates.join(", ")}`);
  return path.join(nsisDir, candidates[0]);
}

export function prepareArtifactForSigning(nsisDir, version) {
  const source = findUpdaterArtifact(nsisDir, version);
  const target = path.join(nsisDir, canonicalUpdaterAssetName(version));
  if (path.resolve(source) !== path.resolve(target)) {
    fs.copyFileSync(source, target);
    if (fs.existsSync(`${source}.sig`)) fs.copyFileSync(`${source}.sig`, `${target}.sig`);
  }
  return target;
}

export function defaultConfigPath() {
  return process.env.MOEPLAY_PUBLISH_CONFIG || DEFAULT_CONFIG;
}

export function loadConfig(configPath = defaultConfigPath()) {
  if (!fs.existsSync(configPath)) {
    throw new Error(`发布配置不存在：${configPath}\n需要字段 privateKeyPath / password / serverBaseUrl / publishToken（publishToken 在服务器安装完成后于 C:\\MoePlayUpdateServer\\server.config.json 中查看）`);
  }
  const cfg = JSON.parse(fs.readFileSync(configPath, "utf8"));
  for (const key of ["privateKeyPath", "password", "serverBaseUrl", "publishToken"]) {
    if (!cfg[key]) throw new Error(`发布配置缺少字段：${key}（${configPath}）`);
  }
  cfg.serverBaseUrl = cfg.serverBaseUrl.replace(/\/+$/, "");
  if (!/^https?:\/\//.test(cfg.serverBaseUrl)) throw new Error(`serverBaseUrl 非法：${cfg.serverBaseUrl}`);
  return cfg;
}

export function createLanUpdaterManifest({ version, signature, serverBaseUrl, assetName, notes, publishedAt = new Date().toISOString() }) {
  if (!/^[A-Za-z0-9._-]+$/.test(assetName)) throw new Error(`资源名非法：${assetName}`);
  if (!signature) throw new Error("签名为空");
  const base = serverBaseUrl.replace(/\/+$/, "");
  return {
    version,
    notes: notes || `MoePlay v${version}`,
    pub_date: publishedAt,
    platforms: {
      "windows-x86_64": {
        signature,
        url: `${base}/installers/${encodeURIComponent(assetName)}`,
      },
    },
  };
}

export function signArtifact(exePath, config) {
  const sigPath = `${exePath}.sig`;
  if (fs.existsSync(sigPath)) return fs.readFileSync(sigPath, "utf8").trim();
  const cli = path.join(root, "node_modules", "@tauri-apps", "cli", "tauri.js");
  const res = spawnSync(process.execPath, [cli, "signer", "sign", exePath, "-f", config.privateKeyPath, "-p", config.password], { encoding: "utf8" });
  if (res.status !== 0) throw new Error(`签名失败：${res.stderr || res.stdout}`);
  if (!fs.existsSync(sigPath)) throw new Error(`签名命令成功但未产出 ${sigPath}`);
  return fs.readFileSync(sigPath, "utf8").trim();
}

function parseArgs(argv) {
  const args = { dir: null, version: null, notes: null, notesFile: null, config: defaultConfigPath(), dryRun: false };
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === "--dir") args.dir = argv[++i];
    else if (a === "--version") args.version = argv[++i];
    else if (a === "--notes") args.notes = argv[++i];
    else if (a === "--notes-file") args.notesFile = argv[++i];
    else if (a === "--config") args.config = argv[++i];
    else if (a === "--dry-run") args.dryRun = true;
    else throw new Error(`未知参数：${a}`);
  }
  return args;
}

async function put(url, token, body, headers = {}) {
  const res = await fetch(url, { method: "PUT", headers: { "X-Publish-Token": token, ...headers }, body });
  if (!res.ok) throw new Error(`上传失败 ${res.status} ${url}：${await res.text()}`);
  return res.text();
}

export async function publish({ dir, version, notes, configPath, dryRun = false }) {
  const pkgVersion = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
  const relDir = dir || path.join(root, "src-tauri", "target", "release", "bundle", "nsis");
  const nsisDir = path.resolve(relDir);
  if (!fs.existsSync(nsisDir)) throw new Error(`找不到 NSIS 产物目录：${nsisDir}`);
  const cfg = loadConfig(configPath);
  const artifact = prepareArtifactForSigning(nsisDir, version);
  const assetName = path.basename(artifact);
  const signature = signArtifact(artifact, cfg);
  const manifest = createLanUpdaterManifest({ version, signature, serverBaseUrl: cfg.serverBaseUrl, assetName, notes });
  if (dryRun) {
    console.log(JSON.stringify({ dryRun: true, assetName, manifest }, null, 2));
    return manifest;
  }
  const base = cfg.serverBaseUrl;
  const exeBytes = fs.readFileSync(artifact);
  process.stdout.write(`上传安装包 ${assetName}（${(exeBytes.length / 1048576).toFixed(1)} MB）… `);
  await put(`${base}/api/upload/installer?name=${encodeURIComponent(assetName)}`, cfg.publishToken, exeBytes, { "Content-Type": "application/octet-stream" });
  process.stdout.write("完成\n上传签名 … ");
  await put(`${base}/api/upload/signature?name=${encodeURIComponent(`${assetName}.sig`)}`, cfg.publishToken, signature, { "Content-Type": "text/plain" });
  process.stdout.write("完成\n发布清单 latest.json … ");
  await put(`${base}/api/upload/manifest`, cfg.publishToken, JSON.stringify(manifest, null, 2), { "Content-Type": "application/json" });
  process.stdout.write("完成\n");
  console.log(`\n✔ v${version} 已发布：${base}/installers/${encodeURIComponent(assetName)}`);
  console.log(`  下载页：${base}/  ·  客户端将在下次「检查更新」时收到提示`);
  return manifest;
}

async function run() {
  const args = parseArgs(process.argv.slice(2));
  const version = args.version || process.env.MOEPLAY_RELEASE_VERSION || JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
  let notes = args.notes;
  if (!notes && args.notesFile) notes = fs.readFileSync(args.notesFile, "utf8").trim();
  await publish({ dir: args.dir, version, notes, configPath: args.config, dryRun: args.dryRun });
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  run().catch((error) => { console.error(`发布失败：${error.message}`); process.exitCode = 1; });
}
