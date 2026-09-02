import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createLanUpdaterManifest, findUpdaterArtifact, prepareArtifactForSigning, loadConfig } from "./publish-update.mjs";

test("createLanUpdaterManifest builds LAN asset url and stable shape", () => {
  const m = createLanUpdaterManifest({
    version: "0.22.1",
    signature: "dW50cnVzdGVkIGNvbW1lbnQ6IHRlc3Q=",
    serverBaseUrl: "http://192.168.2.88:8788/",
    assetName: "MoeGame_0.22.1_x64-setup.exe",
    notes: "修点什么",
    publishedAt: "2026-09-02T00:00:00.000Z",
  });
  assert.equal(m.platforms["windows-x86_64"].url, "http://192.168.2.88:8788/installers/MoeGame_0.22.1_x64-setup.exe");
  assert.equal(m.version, "0.22.1");
  assert.equal(m.notes, "修点什么");
  assert.equal(m.pub_date, "2026-09-02T00:00:00.000Z");
  assert.match(m.platforms["windows-x86_64"].signature, /^dW50cnVzdGVk/);
});

test("createLanUpdaterManifest rejects bad asset names and empty signature", () => {
  assert.throws(() => createLanUpdaterManifest({ version: "1.0.0", signature: "s", serverBaseUrl: "http://h:1", assetName: "../evil.exe" }), /资源名非法/);
  assert.throws(() => createLanUpdaterManifest({ version: "1.0.0", signature: "", serverBaseUrl: "http://h:1", assetName: "a.exe" }), /签名为空/);
});

test("findUpdaterArtifact prefers canonical name and rejects ambiguity", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-nsis-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  fs.writeFileSync(path.join(dir, "萌游 MoeGame_1.2.3_x64-setup.exe"), "a");
  assert.equal(path.basename(findUpdaterArtifact(dir, "1.2.3")), "萌游 MoeGame_1.2.3_x64-setup.exe");
  const canonical = path.join(dir, "MoeGame_1.2.3_x64-setup.exe");
  fs.writeFileSync(canonical, "b");
  assert.equal(path.basename(findUpdaterArtifact(dir, "1.2.3")), "MoeGame_1.2.3_x64-setup.exe");
  fs.writeFileSync(path.join(dir, "other_1.2.3_x64-setup.exe"), "c");
  assert.equal(path.basename(findUpdaterArtifact(dir, "1.2.3")), "MoeGame_1.2.3_x64-setup.exe");
  const dir2 = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-nsis2-"));
  fs.writeFileSync(path.join(dir2, "a_9.9.9_x64-setup.exe"), "x");
  fs.writeFileSync(path.join(dir2, "b_9.9.9_x64-setup.exe"), "y");
  t.after(() => fs.rmSync(dir2, { recursive: true, force: true }));
  assert.throws(() => findUpdaterArtifact(dir2, "9.9.9"), /恰好一个/);
});

test("prepareArtifactForSigning copies to canonical name and carries existing sig", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-nsis-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const src = path.join(dir, "萌游 MoeGame_2.0.0_x64-setup.exe");
  fs.writeFileSync(src, "payload");
  fs.writeFileSync(`${src}.sig`, "sigtext");
  const artifact = prepareArtifactForSigning(dir, "2.0.0");
  assert.equal(path.basename(artifact), "MoeGame_2.0.0_x64-setup.exe");
  assert.equal(fs.readFileSync(artifact, "utf8"), "payload");
  assert.equal(fs.readFileSync(`${artifact}.sig`, "utf8"), "sigtext");
});

test("loadConfig requires all four fields and normalizes base url", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-cfg-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const file = path.join(dir, "cfg.json");
  fs.writeFileSync(file, JSON.stringify({ privateKeyPath: "k", password: "p", serverBaseUrl: "http://192.168.2.88:8788///", publishToken: "t" }));
  const cfg = loadConfig(file);
  assert.equal(cfg.serverBaseUrl, "http://192.168.2.88:8788");
  fs.writeFileSync(file, JSON.stringify({ privateKeyPath: "k", password: "p", serverBaseUrl: "http://h:1" }));
  assert.throws(() => loadConfig(file), /publishToken/);
  assert.throws(() => loadConfig(path.join(dir, "missing.json")), /发布配置不存在/);
});
