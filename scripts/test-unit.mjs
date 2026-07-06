import { spawnSync } from "node:child_process";
import process from "node:process";

let cwd = process.cwd();
if (process.platform === "win32" && /^[a-z]:/.test(cwd)) {
  cwd = cwd[0].toUpperCase() + cwd.slice(1);
}

const result = spawnSync(
  process.platform === "win32" ? "npx.cmd" : "npx",
  ["vitest", "run"],
  { stdio: "inherit", cwd, shell: process.platform === "win32" }
);

process.exit(result.status ?? 1);
