import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { createHash, generateKeyPairSync, sign } from "node:crypto";

import { runCli, verifyUpdaterManifest } from "./verify-updater-artifacts.mjs";

const version = "0.12.1";
const keyId = Buffer.from("0102030405060708", "hex");
const keys = generateKeyPairSync("ed25519");
const publicPacket = Buffer.concat([Buffer.from("Ed"), keyId, keys.publicKey.export({format:"der",type:"spki"}).subarray(-32)]);
const publicKey = Buffer.from([
  "untrusted comment: minisign public key",
  publicPacket.toString("base64"),
].join("\n"), "utf8").toString("base64");
const trustedComment = "timestamp:1783699200\tfile:moeplay_0.12.1_x64-setup.nsis.zip";
const signatureBytes = sign(null, createHash("blake2b512").update("signed updater payload").digest(), keys.privateKey);
const signaturePacket = Buffer.concat([Buffer.from("ED"), keyId, signatureBytes]);
const minisignText = [
  "untrusted comment: signature from minisign secret key",
  signaturePacket.toString("base64"),
  `trusted comment: ${trustedComment}`,
  sign(null, Buffer.concat([signatureBytes, Buffer.from(trustedComment)]), keys.privateKey).toString("base64"),
].join("\n");
const signature = Buffer.from(minisignText, "utf8").toString("base64");

function fixture(overrides = {}) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-updater-"));
  const artifactName = "moeplay_0.12.1_x64-setup.nsis.zip";
  const artifact = path.join(directory, artifactName);
  fs.writeFileSync(artifact, "signed updater payload");
  fs.writeFileSync(`${artifact}.sig`, signature);
  const manifest = path.join(directory, "latest.json");
  fs.writeFileSync(manifest, JSON.stringify({
    version,
    notes: "test",
    pub_date: "2026-07-11T00:00:00.000Z",
    platforms: {
      "windows-x86_64-nsis": {
        url: `https://github.com/Cicada0719/moeplay-tauri/releases/download/v${version}/${artifactName}`,
        signature,
        ...overrides,
      },
    },
  }));
  return { directory, manifest, artifact };
}

test("accepts HTTPS metadata whose signature matches the local detached signature", () => {
  const data = fixture();
  const result = verifyUpdaterManifest(data.manifest, { expectedVersion: version, artifactsDir: data.directory, publicKey });
  assert.equal(result.version, version);
  assert.deepEqual(result.verifiedArtifacts, [data.artifact]);
});

test("rejects modified installer bytes even when detached and manifest signatures match", () => {
  const data = fixture(); fs.appendFileSync(data.artifact, "tampered");
  assert.throws(() => verifyUpdaterManifest(data.manifest, { expectedVersion: version, artifactsDir: data.directory, publicKey }), /cryptographic signature/);
});

test("rejects non-HTTPS updater URLs", () => {
  const data = fixture({ url: "http://example.invalid/moeplay_0.12.1_x64-setup.nsis.zip" });
  assert.throws(
    () => verifyUpdaterManifest(data.manifest, { expectedVersion: version, artifactsDir: data.directory, publicKey }),
    /must use HTTPS/,
  );
});

test("rejects version drift", () => {
  const data = fixture();
  assert.throws(
    () => verifyUpdaterManifest(data.manifest, { expectedVersion: "0.12.2", artifactsDir: data.directory, publicKey }),
    /expected=0\.12\.2/,
  );
});

test("rejects signatures made by a key other than the configured updater key", () => {
  const otherPublicPacket = Buffer.concat([Buffer.from("Ed"), Buffer.alloc(8, 3), Buffer.alloc(32, 5)]);
  const otherPublicKey = Buffer.from([
    "untrusted comment: minisign public key",
    otherPublicPacket.toString("base64"),
  ].join("\n"), "utf8").toString("base64");
  const data = fixture();
  assert.throws(
    () => verifyUpdaterManifest(data.manifest, { expectedVersion: version, artifactsDir: data.directory, publicKey: otherPublicKey }),
    /key ID does not match/,
  );
});

test("rejects malformed signature metadata", () => {
  const data = fixture({ signature: "not-a-minisign-signature" });
  assert.throws(
    () => verifyUpdaterManifest(data.manifest, { expectedVersion: version, artifactsDir: data.directory, publicKey }),
    /signature is neither minisign text nor its base64 encoding/,
  );
});

test("requires the URL-named updater artifact to exist locally", () => {
  const data = fixture({ url: `https://example.invalid/missing_${version}.nsis.zip` });
  assert.throws(
    () => verifyUpdaterManifest(data.manifest, { expectedVersion: version, artifactsDir: data.directory, publicKey }),
    /expected exactly one local updater artifact/,
  );
});

test("rejects a manifest signature that differs from the detached signature", () => {
  const otherText = minisignText.replace(minisignText.split("\n")[3], Buffer.alloc(64, 8).toString("base64"));
  const otherSignature = Buffer.from(otherText, "utf8").toString("base64");
  const data = fixture({ signature: otherSignature });
  assert.throws(
    () => verifyUpdaterManifest(data.manifest, { expectedVersion: version, artifactsDir: data.directory, publicKey }),
    /does not match/,
  );
});

test("ordinary workspaces report not-generated while required mode fails", () => {
  const empty = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-no-updater-"));
  assert.equal(runCli([], { searchRoots: [empty] }).status, "not-generated");
  assert.throws(() => runCli(["--require"], { searchRoots: [empty] }), /Signed release mode requires updater artifacts/);
});
