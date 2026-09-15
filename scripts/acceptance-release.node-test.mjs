import assert from "node:assert/strict";
import test from "node:test";
import { buildAcceptanceReport, probeDownloadEndpoint, readReleaseIdentity, validateReleaseIdentity } from "./acceptance-release.mjs";

test("release identity is synchronized at v0.24.0", () => {
  const identity = readReleaseIdentity();
  assert.equal(identity.version, "0.24.0");
  assert.deepEqual(Object.values(identity.files), ["0.24.0", "0.24.0", "0.24.0", "0.24.0", "0.24.0", "0.24.0"]);
  assert.doesNotThrow(() => validateReleaseIdentity("0.24.0", identity));
  assert.throws(() => validateReleaseIdentity("0.23.1", identity), /mismatch|Invalid/);
});

test("missing real device and server evidence remains pending", async () => {
  const report = await buildAcceptanceReport({ commit: "a".repeat(40) });
  assert.equal(report.summary.failed, 0);
  assert.ok(report.summary.pending >= 9);
  assert.equal(report.checks.find((item) => item.id === "android-upgrade").status, "pending");
  assert.equal(report.checks.find((item) => item.id === "public-download").status, "pending");
});

test("download probe requires HTTP 206 and no-store for latest metadata", async () => {
  const makeFetch = (status, range, cache) => async () => ({
    status,
    ok: status >= 200 && status < 300,
    headers: { get: (name) => ({ "content-range": range, "cache-control": cache }[name]) },
  });
  assert.equal((await probeDownloadEndpoint("https://example.test/latest.json", makeFetch(200, "", "no-store"))).status, "failed");
  assert.equal((await probeDownloadEndpoint("https://example.test/latest.json", makeFetch(206, "bytes 0-0/10", "max-age=60"))).status, "failed");
  assert.equal((await probeDownloadEndpoint("https://example.test/latest.json", makeFetch(206, "bytes 0-0/10", "no-store"))).status, "passed");
});
