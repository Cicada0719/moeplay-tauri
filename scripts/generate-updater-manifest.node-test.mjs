import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import assert from "node:assert/strict";
import { createUpdaterManifest, findUpdaterArtifact } from "./generate-updater-manifest.mjs";

test("creates a latest.json entry from a signed NSIS artifact", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-updater-"));
  const artifact = path.join(dir, "MoeGame_1.2.3_x64-setup.exe");
  fs.writeFileSync(artifact, "installer");
  fs.writeFileSync(`${artifact}.sig`, "untrusted comment: signature\nRUSIGNATURE");
  assert.equal(findUpdaterArtifact(dir, "1.2.3"), artifact);
  const manifest = createUpdaterManifest({ version: "1.2.3", artifactPath: artifact, signaturePath: `${artifact}.sig`, repository: "owner/repo", publishedAt: "2026-01-01T00:00:00.000Z" });
  assert.equal(manifest.version, "1.2.3");
  assert.match(manifest.platforms["windows-x86_64"].url, /releases\/download\/v1\.2\.3\/MoeGame_1\.2\.3_x64-setup\.exe$/);
  assert.match(manifest.platforms["windows-x86_64"].signature, /RUSIGNATURE/);
});

test("refuses to publish without exactly one signed updater artifact", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-updater-empty-"));
  assert.throws(() => findUpdaterArtifact(dir, "1.2.3"), /Expected one/);
});
