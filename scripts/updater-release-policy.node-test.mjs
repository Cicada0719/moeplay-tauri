import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";
import assert from "node:assert/strict";

const root = resolve(import.meta.dirname, "..");
const workflow = readFileSync(resolve(root, ".github/workflows/release.yml"), "utf8");
const config = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));

test("official releases require signed automatic-update artifacts", () => {
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY is required\. Unsigned releases are forbidden\./);
  assert.match(workflow, /createUpdaterArtifacts\":true/);
  assert.match(workflow, /includeUpdaterJson: true/);
  assert.match(workflow, /UPDATER_RELEASE_MODE: Required/);
  assert.match(workflow, /test:visual -- --ignore-snapshots --workers=1/);
  assert.match(workflow, /npm run generate:updater-manifest/);
  assert.match(workflow, /Remove-Item -LiteralPath \.tauri-updater\.conf\.json -Force/);
  assert.match(
    workflow,
    /Remove release-only Tauri metadata[\s\S]*Remove-Item -LiteralPath latest\.json -Force -ErrorAction SilentlyContinue/,
    "tauri-action's generated latest.json must be removed before clean build metadata is captured",
  );
  assert.ok(
    workflow.indexOf("Generate build metadata") < workflow.indexOf("Generate signed latest.json"),
    "build metadata must be captured before latest.json makes the checkout dirty",
  );
  assert.match(workflow, /gh release upload .*latest\.json.*\$updater\.FullName.*\.sig/);
  assert.match(workflow, /verify-updater-artifacts\.mjs --require/);
  assert.match(workflow, /gh release edit .*--draft=false/);
  assert.doesNotMatch(workflow, /degraded|installer-only|includeUpdaterJson: false/i);
});

test("desktop clients use the LAN update server endpoint", () => {
  assert.ok(config.plugins?.updater?.pubkey, "updater public key is required");
  assert.deepEqual(config.plugins.updater.endpoints, [
    "http://192.168.2.88:8788/latest.json",
  ]);
  assert.equal(config.plugins.updater.dangerousInsecureTransportProtocol, true, "LAN server is plain HTTP; package integrity stays guaranteed by minisign signatures");
  assert.equal(config.bundle?.createUpdaterArtifacts, true, "signed updater artifacts must be produced by tauri build");
});
