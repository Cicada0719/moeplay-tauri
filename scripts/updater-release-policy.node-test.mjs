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
  assert.match(workflow, /npm run generate:updater-manifest/);
  assert.match(workflow, /Remove-Item -LiteralPath \.tauri-updater\.conf\.json -Force/);
  assert.ok(
    workflow.indexOf("Generate build metadata") < workflow.indexOf("Generate signed latest.json"),
    "build metadata must be captured before latest.json makes the checkout dirty",
  );
  assert.match(workflow, /gh release upload .*latest\.json/);
  assert.match(workflow, /verify-updater-artifacts\.mjs --require/);
  assert.match(workflow, /gh release edit .*--draft=false/);
  assert.doesNotMatch(workflow, /degraded|installer-only|includeUpdaterJson: false/i);
});

test("desktop clients use the signed latest release endpoint", () => {
  assert.ok(config.plugins?.updater?.pubkey, "updater public key is required");
  assert.deepEqual(config.plugins.updater.endpoints, [
    "https://github.com/Cicada0719/moeplay-tauri/releases/latest/download/latest.json",
  ]);
});
