import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";

export function normalizeWindowsDrive(value, platform = process.platform) {
  if (platform === "win32" && /^[a-z]:/.test(value)) {
    return value[0].toUpperCase() + value.slice(1);
  }
  return value;
}

export function resolveProjectRoot(scriptUrl = import.meta.url, platform = process.platform) {
  const scriptsDirectory = path.dirname(fileURLToPath(scriptUrl));
  return normalizeWindowsDrive(path.resolve(scriptsDirectory, ".."), platform);
}

export function resolveVitestBin(resolveSpecifier = import.meta.resolve) {
  const packageJsonPath = fileURLToPath(resolveSpecifier("vitest/package.json"));
  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  const bin = typeof packageJson.bin === "string" ? packageJson.bin : packageJson.bin?.vitest;

  if (!bin) {
    throw new Error(`Vitest package does not declare a CLI binary: ${packageJsonPath}`);
  }

  return path.resolve(path.dirname(packageJsonPath), bin);
}

export function createVitestInvocation({
  execPath = process.execPath,
  projectRoot = resolveProjectRoot(),
  vitestBin = resolveVitestBin(),
} = {}) {
  return {
    command: execPath,
    args: [vitestBin, "run", "--config", path.join(projectRoot, "vitest.config.ts")],
    options: {
      cwd: projectRoot,
      shell: false,
      stdio: "inherit",
    },
  };
}

export function runUnitTests(overrides) {
  const invocation = createVitestInvocation(overrides);
  const result = spawnSync(invocation.command, invocation.args, invocation.options);

  if (result.error) {
    console.error(JSON.stringify({
      event: "frontend-validation-spawn-error",
      command: invocation.command,
      args: invocation.args,
      cwd: invocation.options.cwd,
      code: result.error.code ?? null,
      message: result.error.message,
    }));
    return 1;
  }

  if (result.status === null) {
    console.error(JSON.stringify({
      event: "frontend-validation-process-terminated",
      signal: result.signal ?? null,
      cwd: invocation.options.cwd,
    }));
    return 1;
  }

  return result.status;
}

const isMain = process.argv[1]
  && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;

if (isMain) {
  process.exitCode = runUnitTests();
}
