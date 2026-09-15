import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { generateManifest, verifyManifest, describeAsset } from "./release-manifest.mjs";

function fixture(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-manifest-test-"));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const version = JSON.parse(fs.readFileSync(new URL("../package.json", import.meta.url))).version;
  for (const suffix of ["setup.exe", "setup.msi", "portable.zip", "arm64-release.apk", "arm64-compat.apk"])
    fs.writeFileSync(path.join(directory, `MoeGame_${version}_${suffix}`), suffix);
  fs.writeFileSync(path.join(directory, "latest.json"), JSON.stringify({ version, platforms: { "windows-x86_64": { signature: "verified-separately" } } }));
  return { directory, version };
}

test("manifest records all channels and verifies the exact batch", t => {
  const { directory } = fixture(t);
  const manifest = generateManifest(directory, { commit: "a".repeat(40), publishedAt: "2026-09-08T00:00:00Z" });
  assert.equal(manifest.assets.length, 5);
  assert.equal(manifest.androidCompatibilityVerified, false);
  assert.deepEqual(verifyManifest(directory), manifest);
  assert.throws(() => verifyManifest(directory, { commit: "b".repeat(40) }), /commit does not match/);
  assert.throws(() => describeAsset(directory, "../outside.exe", manifest.version), /Invalid/);
});

test("rejects tampered assets, absent channels and mismatched updater versions", t => {
  const { directory, version } = fixture(t);
  const manifest = generateManifest(directory, { commit: "b".repeat(40) });
  const asset = path.join(directory, manifest.assets[0].file);
  const original = fs.readFileSync(asset);
  fs.appendFileSync(asset, "tampered");
  assert.throws(() => verifyManifest(directory), /Mismatch/);
  fs.writeFileSync(asset, original);
  fs.writeFileSync(path.join(directory, "release-manifest.json"), JSON.stringify({ ...manifest, assets: manifest.assets.filter(a => a.channel !== "compat") }));
  assert.throws(() => verifyManifest(directory), /Missing compat/);
  fs.writeFileSync(path.join(directory, "release-manifest.json"), JSON.stringify(manifest));
  fs.writeFileSync(path.join(directory, "latest.json"), JSON.stringify({ version: version + "0" }));
  assert.throws(() => verifyManifest(directory), /Missing signed/);
});

test("requires both bound Android coverage reports before claiming data retention", t => {
  const { directory } = fixture(t);
  const manifest = generateManifest(directory, { commit: "c".repeat(40) });
  const apkByChannel = Object.fromEntries(manifest.assets.filter((asset) => asset.platform === "android").map((asset) => [asset.channel, asset.sha256]));
  const report = (channel) => ({
    channel, coverageUpgrade: true, packageName: "com.moeplay.app", versionCode: 24000,
    certificateFingerprint: "b".repeat(64), apkSha256: apkByChannel[channel], device: "test-device",
    dataCheck: { history: true },
  });
  const withEvidence = {
    ...manifest, androidCompatibilityVerified: true,
    androidVerification: { release: report("release"), compat: report("compat") },
  };
  fs.writeFileSync(path.join(directory, "release-manifest.json"), JSON.stringify(withEvidence));
  assert.deepEqual(verifyManifest(directory), withEvidence);
  fs.writeFileSync(path.join(directory, "release-manifest.json"), JSON.stringify({ ...withEvidence, androidVerification: { ...withEvidence.androidVerification, compat: report("release") } }));
  assert.throws(() => verifyManifest(directory), /wrong channel|does not match/);
  fs.writeFileSync(path.join(directory, "release-manifest.json"), JSON.stringify({ ...manifest, androidCompatibilityVerified: true }));
  assert.throws(() => verifyManifest(directory), /without upgrade evidence/);
});

test("generation can bind a frozen source commit explicitly", t => {
  const { directory } = fixture(t);
  const manifest = generateManifest(directory, { commit: "d".repeat(40) });
  assert.equal(manifest.commit, "d".repeat(40));
});
