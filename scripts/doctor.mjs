import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
const checks = [
  ["Node >= 22", Number(process.versions.node.split(".")[0]) >= 22],
  ["Rust toolchain", spawnSync("cargo", ["--version"], { encoding: "utf8" }).status === 0],
  ["Git", spawnSync("git", ["--version"], { encoding: "utf8" }).status === 0],
  ["npm dependencies", existsSync("node_modules/@tauri-apps/cli/tauri.js")],
];
for (const [label, ok] of checks) console.log(`${ok ? "OK" : "MISSING"} ${label}`);
console.log("Windows: install Visual Studio C++ Build Tools and WebView2. Android: JDK 17, SDK 36, NDK and aarch64-linux-android Rust target.");
console.log("Ordinary development and unsigned builds do not require release signing keys.");
process.exitCode = checks.every(([, ok]) => ok) ? 0 : 1;
