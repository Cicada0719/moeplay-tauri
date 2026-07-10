import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { inspectBundle } from "./verify-bundle-budget.mjs";

function fixture(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moeplay-bundle-budget-"));
  for (const [name, bytes] of Object.entries(files)) fs.writeFileSync(path.join(dir, name), Buffer.alloc(bytes));
  return dir;
}

test("accepts chunks within explicit budgets", () => {
  const dir = fixture({ "index-a.js": 80, "AnimePage-a.js": 60, "ComicPage-a.js": 20 });
  const report = inspectBundle(dir, { totalJavaScriptBytes: 200, largestChunkBytes: 100, animeChunkBytes: 70, comicChunkBytes: 30 });
  assert.deepEqual(report.failures, []);
  fs.rmSync(dir, { recursive: true, force: true });
});

test("reports every exceeded budget", () => {
  const dir = fixture({ "index-a.js": 110, "AnimePage-a.js": 80, "ComicPage-a.js": 40 });
  const report = inspectBundle(dir, { totalJavaScriptBytes: 200, largestChunkBytes: 100, animeChunkBytes: 70, comicChunkBytes: 30 });
  assert.equal(report.failures.length, 4);
  fs.rmSync(dir, { recursive: true, force: true });
});
