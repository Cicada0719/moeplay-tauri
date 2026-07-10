import { spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";

let cwd = process.cwd();
if (process.platform === "win32" && /^[a-z]:/.test(cwd)) {
  cwd = cwd[0].toUpperCase() + cwd.slice(1);
}

function run(command, args, env = process.env) {
  const result = spawnSync(command, args, {
    cwd,
    env,
    stdio: "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const vitestCli = path.join(cwd, "node_modules", "vitest", "vitest.mjs");

run(cargo, [
  "test",
  "--manifest-path",
  "src-tauri/Cargo.toml",
  "--test",
  "live_anime",
  "--",
  "--ignored",
  "--nocapture",
]);

run(
  process.execPath,
  [vitestCli, "run", "src/lib/sources/liveAcceptance.test.ts"],
  { ...process.env, MOEPLAY_LIVE_TESTS: "1" },
);
