import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const outputDir = path.join(root, "src-tauri", "target", "release", "bundle");
const output = path.join(outputDir, "build-metadata.json");
function command(file, args) {
  try { return execFileSync(file, args, { cwd: root, encoding: "utf8", windowsHide: true }).trim(); }
  catch { return null; }
}
const metadata = {
  schemaVersion: 1,
  product: "MoePlay",
  version: pkg.version,
  generatedAt: new Date().toISOString(),
  commit: process.env.GITHUB_SHA ?? command("git", ["rev-parse", "HEAD"]),
  ref: process.env.GITHUB_REF_NAME ?? command("git", ["branch", "--show-current"]),
  dirty: Boolean(command("git", ["status", "--porcelain", "--untracked-files=no"])),
  runner: { os: os.platform(), release: os.release(), arch: os.arch(), githubRunner: process.env.RUNNER_OS ?? null },
  toolchain: {
    node: process.version,
    npm: command(process.platform === "win32" ? "npm.cmd" : "npm", ["--version"]),
    rustc: command("rustc", ["--version"]),
    cargo: command("cargo", ["--version"]),
    tauriCli: command(process.platform === "win32" ? "npx.cmd" : "npx", ["tauri", "--version"]),
  },
};
fs.mkdirSync(outputDir, { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(metadata, null, 2)}\n`);
console.log(output);
