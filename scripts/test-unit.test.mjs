import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { expect, test } from "vitest";

import {
  createVitestInvocation,
  normalizeWindowsDrive,
  resolveProjectRoot,
  resolveVitestBin,
} from "./test-unit.mjs";

test("normalizes only a lowercase Windows drive letter", () => {
  expect(normalizeWindowsDrive("c:\\repo\\app", "win32")).toBe("C:\\repo\\app");
  expect(normalizeWindowsDrive("C:\\repo\\app", "win32")).toBe("C:\\repo\\app");
  expect(normalizeWindowsDrive("\\\\server\\share", "win32")).toBe("\\\\server\\share");
  expect(normalizeWindowsDrive("c:/repo/app", "linux")).toBe("c:/repo/app");
});

test("resolves the project root from the launcher location, not process.cwd", () => {
  const expectedRoot = normalizeWindowsDrive(process.cwd());
  const scriptUrl = pathToFileURL(path.join(expectedRoot, "scripts", "test-unit.mjs")).href;
  expect(resolveProjectRoot(scriptUrl)).toBe(expectedRoot);
});

test("resolves Vitest from its package-declared local binary", () => {
  const projectRoot = normalizeWindowsDrive(process.cwd());
  const packageJsonUrl = pathToFileURL(
    path.join(projectRoot, "node_modules", "vitest", "package.json"),
  ).href;
  const vitestBin = resolveVitestBin(() => packageJsonUrl);

  expect(path.basename(vitestBin)).toBe("vitest.mjs");
  expect(vitestBin).toMatch(/node_modules[\\/]vitest[\\/]vitest\.mjs$/);
});

test("launches the local Vitest entrypoint through Node without a shell", () => {
  const projectRoot = normalizeWindowsDrive(process.cwd());
  const vitestBin = path.join(projectRoot, "node_modules", "vitest", "vitest.mjs");
  const invocation = createVitestInvocation({
    execPath: "C:\\Program Files\\nodejs\\node.exe",
    projectRoot,
    vitestBin,
  });

  expect(invocation.command).toBe("C:\\Program Files\\nodejs\\node.exe");
  expect(invocation.args).toEqual([
    vitestBin,
    "run",
    "--config",
    path.join(projectRoot, "vitest.config.ts"),
  ]);
  expect(invocation.options).toEqual({
    cwd: projectRoot,
    shell: false,
    stdio: "inherit",
  });
});
