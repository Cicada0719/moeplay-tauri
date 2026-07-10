import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const version = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8")).version;
function run(script) {
  const result = spawnSync(process.execPath, [path.join(root, "scripts", script)], { cwd: root, encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr || result.stdout);
}

test("release evidence contains versioned SBOM and toolchain metadata", () => {
  run("generate-sbom.mjs");
  run("generate-build-metadata.mjs");
  const bundle = path.join(root, "src-tauri", "target", "release", "bundle");
  const sbom = JSON.parse(fs.readFileSync(path.join(bundle, "sbom.cdx.json"), "utf8"));
  assert.equal(sbom.bomFormat, "CycloneDX");
  assert.equal(sbom.specVersion, "1.5");
  assert.equal(sbom.metadata.component.version, version);
  assert.ok(sbom.components.length > 100);
  assert.ok(sbom.components.some((component) => component.purl?.startsWith("pkg:cargo/")));
  assert.ok(sbom.components.some((component) => component.purl?.startsWith("pkg:npm/")));
  assert.equal(new Set(sbom.components.map((component) => component.purl)).size, sbom.components.length);

  const metadata = JSON.parse(fs.readFileSync(path.join(bundle, "build-metadata.json"), "utf8"));
  assert.equal(metadata.version, version);
  assert.match(metadata.commit, /^[0-9a-f]{40}$/i);
  assert.match(metadata.toolchain.rustc, /^rustc /);
  assert.match(metadata.toolchain.node, /^v\d+/);
  assert.equal(typeof metadata.dirty, "boolean");
});
