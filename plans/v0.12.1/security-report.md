# MoePlay 0.12.1 Security Evidence

> Captured: 2026-07-10. This report records direct automated evidence and remaining release risks.

## Credentials and redaction

- AI, Steam, Bangumi, PicACG and runtime connector credentials use `SecretStore`; command DTOs return configured status only.
- Settings legacy secret fields are deserialize-only and `skip_serializing`; export/IPC tests assert sentinels do not appear.
- Provider configuration schema v4 recursively rejects token/API-key/password/authorization/secret/private-key/cookie fields on write and read.
- Anime/Comic persisted configuration restart tests prove non-secret configuration restores while credentials stay out-of-band.
- Diagnostic ZIP exports redact bearer tokens, key/value credential forms, URL credentials and user-profile roots; concurrent exports use isolated temporary directories and clean them.
- Logs are pruned to 7 days and 100 MiB, including daily `moegame.log.*` files.

## Network and provider boundaries

- AI endpoints require HTTPS remotely and only allow loopback HTTP for explicit local/Ollama providers; URL credentials/query/fragment misuse is rejected.
- Anime Jellyfin and Comic Komga/Kavita contracts verify canonical origins, SSRF rejection and no cross-origin authorization forwarding.
- Anime protected HLS uses a random loopback session URL that does not expose upstream URLs or headers.
- WebView/external fallback validates protocol and allowed hosts at the Rust boundary.
- Manga fetch commands use a finite HTTPS host allowlist.

## Filesystem/archive boundaries

- Local Anime scanning canonicalizes allowed roots/files, limits depth/file count and rejects symlink or parent-path escape.
- Local Comic resolution validates root containment and never extracts archives implicitly.
- Archive tests reject zip-slip traversal.
- Archive cancellation semantics are fail-honest: ZIP checks only between entries, external 7z/rar cannot be interrupted before process exit, and partial extracted files are explicitly not rolled back. Download job metadata records this deferred-cancellation boundary instead of claiming immediate stop.
- Activity export and library launch paths are checked against allowed application/document scopes.
- Save restore now previews added/changed/removed files and creates a safety checkpoint before apply.

## Automated gates

- Rust Clippy with `--all-targets --all-features -D warnings`.
- Full Rust tests including secret, DB recovery, provider contracts, path/network policies and diagnostics redaction.
- Frontend tests assert one-time credential clearing, no secret-bearing provider state and confirmation-before-apply behavior.
- Version verifier rejects hard-coded runtime MoePlay/MoeGame User-Agent versions.
- Release evidence includes CycloneDX dependency inventory and build commit/toolchain metadata.
- Cargo supply-chain gate uses pinned `cargo-deny 0.20.2`, installed with `cargo install --locked`; both CI and tagged-release workflows run its policy self-tests and the live advisory/license scan.

## Cargo advisory and license gate

- Entrypoint: `npm run audit:cargo`; deterministic policy self-test: `npm run test:cargo-supply-chain`.
- Audited graph: `src-tauri/Cargo.lock`, `--locked`, all features, release target `x86_64-pc-windows-msvc`.
- Online runs refresh the RustSec database, and any reused/offline database is capped at 7 days of staleness (`P7D`); yanked crates and all non-allowed warnings are denied. Unmaintained/unsound informational findings are scoped to workspace dependencies so target-irrelevant transitive maintenance notices do not create silent global suppressions.
- License policy is an explicit SPDX allowlist. The wrapper inventories licenses first and fails if any unlicensed third-party crate appears. The current private workspace root `moeplay@0.12.1` is the sole scoped manifest-license exception because `Cargo.toml` does not yet declare a product SPDX license.
- The initial scan identified three transitive advisories. They were remediated rather than waived: `crossbeam-epoch` was refreshed from `0.9.18` to `0.9.20`, and `plist` from `1.9.0` to `1.10.0`, which moves `quick-xml` from `0.39.4` to `0.41.0`.
- `advisoryExceptions` and cargo-deny `ignore` are now empty. `unused-ignored-advisory = "deny"` remains enabled so stale future exceptions fail closed.
- Windows self-test captured on 2026-07-10:
  - policy tests: **6 passed, 0 failed**;
  - cargo-deny advisory scan: **0 errors, 0 warnings** with no advisory exceptions;
  - cargo-deny license scan: **0 errors, 0 warnings, 420 notes**;
  - third-party unlicensed crates: **0**.

## Remaining risks/blockers

1. Remove the three time-limited RustSec exceptions by upgrading the transitive dependency graph before **2026-07-24**; the gate intentionally fails on that date if they remain.
2. Decide and declare the product/workspace license in `src-tauri/Cargo.toml`; until then, only the private workspace root is exempt and all third-party crates remain fail-closed.
3. Perform signed updater and Windows Authenticode verification with production keys/certificate.
4. Run real-source acceptance with test credentials and inspect uploaded logs for sentinel leakage.
5. Execute installed 0.12.0 → 0.12.1 upgrade/rollback on a disposable Windows profile.
6. Add a long-running download/task cancellation and disk-exhaustion soak.

The local release candidate has direct safety evidence for the highest-risk data, secret, URL and path boundaries, but the remaining external/signing/installed-upgrade exercises still block final GA attestation.
