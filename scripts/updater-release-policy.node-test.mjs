import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";
import assert from "node:assert/strict";
const root = resolve(import.meta.dirname, "..");
const read = file => readFileSync(resolve(root, file), "utf8");
const config = JSON.parse(read("src-tauri/tauri.conf.json"));
const official = JSON.parse(read("src-tauri/tauri.official.conf.json"));
test("Fork builds need no signing secrets; official builds retain signed HTTPS updates", () => {
  assert.equal(config.bundle.createUpdaterArtifacts, false);
  assert.deepEqual(config.plugins.updater, { pubkey: "", endpoints: [] });
  assert.equal(official.bundle.createUpdaterArtifacts, true);
  assert.ok(official.plugins.updater.pubkey);
  assert.deepEqual(official.plugins.updater.endpoints, [
    "https://moeplay.sgy0719.top/latest.json",
    "https://github.com/sgyxyx-prog/moeplay-tauri/releases/latest/download/latest.json",
  ]);
  assert.ok(official.plugins.updater.endpoints.every(url => url.startsWith("https://")));
  assert.equal(official.plugins.updater.dangerousInsecureTransportProtocol, undefined);
});
test("CI audits are read-only and screenshots are compared", () => {
  const ci = read(".github/workflows/ci.yml");
  assert.match(ci, /contents: read/);
  assert.doesNotMatch(ci, /ignore-snapshots|git push|audit fix/);
  assert.equal(JSON.parse(read("package.json")).scripts["audit:dependencies"], "npm audit --audit-level=high");
});
test("release workflow verifies uploaded local artifacts without rebuilding or replacing attachments", () => {
  const workflow = read(".github/workflows/release.yml");
  assert.match(workflow, /release-manifest.mjs artifacts\/verify --verify/);
  assert.match(workflow, /verify-updater-artifacts.mjs --require/);
  assert.doesNotMatch(workflow, /tauri-action|--clobber|gh release upload|contents: write/);
});
