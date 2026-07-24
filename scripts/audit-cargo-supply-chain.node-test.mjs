import assert from "node:assert/strict";
import test from "node:test";

import {
  advisoryIdsFromDenyConfig,
  assertOnlyWorkspaceIsUnlicensed,
  assertPolicyMatchesDenyConfig,
  parseCargoDenyVersion,
  parseIsoDate,
  parsePackageIdentity,
  validatePolicy,
} from "./audit-cargo-supply-chain.mjs";

const validPolicy = {
  schemaVersion: 1,
  cargoDenyVersion: "0.20.2",
  target: "x86_64-pc-windows-msvc",
  advisoryExceptions: [
    {
      id: "RUSTSEC-2026-0204",
      expiresOn: "2026-07-24",
      reason: "A sufficiently detailed temporary transitive dependency remediation reason.",
    },
  ],
};

test("policy accepts exact versions and unexpired, justified advisory exceptions", () => {
  assert.equal(validatePolicy(validPolicy, new Date("2026-07-10T12:00:00Z")), validPolicy);
});

test("policy fails closed on the exception expiry date", () => {
  assert.throws(
    () => validatePolicy(validPolicy, new Date("2026-07-24T00:00:00Z")),
    /expired on 2026-07-24/,
  );
});

test("policy rejects duplicate advisory IDs and invalid dates", () => {
  // 钉住 now：不随真实日历变化——夹具的 expiresOn(2026-07-24) 一到当天，
  // 不过期的前置校验会先抛"已过期"，永远测不到"重复 id"这条路径。
  assert.throws(
    () => validatePolicy({ ...validPolicy, advisoryExceptions: [...validPolicy.advisoryExceptions, ...validPolicy.advisoryExceptions] }, new Date("2026-07-10T12:00:00Z")),
    /duplicate RustSec advisory exception/,
  );
  assert.throws(() => parseIsoDate("2026-02-30"), /valid calendar date/);
});

test("cargo-deny.toml ignores must exactly match the expiring policy", () => {
  const config = '[graph]\ntargets = ["x86_64-pc-windows-msvc"]\n[advisories]\nignore = [{ id = "RUSTSEC-2026-0204", reason = "temporary" }]\n';
  assert.deepEqual(advisoryIdsFromDenyConfig(config), ["RUSTSEC-2026-0204"]);
  assert.doesNotThrow(() => assertPolicyMatchesDenyConfig(validPolicy, config));
  assert.throws(() => assertPolicyMatchesDenyConfig(validPolicy, config.replace("0204", "9999")), /do not match policy/);
});

test("tool version and Cargo package identity parsers are strict", () => {
  assert.equal(parseCargoDenyVersion("cargo-deny 0.20.2\n"), "0.20.2");
  assert.equal(parseCargoDenyVersion("cargo-deny latest\n"), null);
  assert.equal(parsePackageIdentity('[package]\nname = "moeplay"\nversion = "0.12.1"\n\n[dependencies]\n'), "moeplay@0.12.1");
});

test("only the private workspace package may be unlicensed", () => {
  assert.doesNotThrow(() => assertOnlyWorkspaceIsUnlicensed("MIT (1): serde@1.0.0\nUnlicensed (1): moeplay@0.12.1\n", "moeplay@0.12.1"));
  assert.throws(
    () => assertOnlyWorkspaceIsUnlicensed("Unlicensed (2): moeplay@0.12.1, mystery@1.0.0\n", "moeplay@0.12.1"),
    /only the private workspace crate/,
  );
});
