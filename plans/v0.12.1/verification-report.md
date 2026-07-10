# MoePlay 0.12.1 Verification Report

> Verification date: 2026-07-10  
> Branch: `codex/v0.12.1-foundation`  
> Result: local release candidate builds successfully; full MASTER PLAN is not yet complete.

## Quality gates

| Gate | Evidence | Result |
|---|---|---|
| Version consistency | `npm run verify:versions` | passed; all six version sources are `0.12.1` |
| Tauri command contract | `npm run verify:commands` | passed; handler/build/capability/permissions are `328/328/328/328`, 258 literal frontend invocations checked |
| Verifier self-tests | `npm run test:command-contract` | 4 passed |
| Svelte/TypeScript | `npm run check` | 0 errors, 0 warnings |
| Frontend unit | `npm run test:unit` | 263 passed, 1 skipped |
| Frontend browser smoke | `npm run test:visual` | 3 passed |
| Frontend production build | `npm run build` | passed |
| Rust formatting | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | passed |
| Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | passed |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features` | passed; live/environment tests remain explicitly ignored |
| Deterministic HTTP download controls | `cargo test --manifest-path src-tauri/Cargo.toml downloader::tests -- --nocapture` | passed; 8 tests including real loopback HTTP pause/Range-resume, cancel/disconnect and archive limitation |
| Archive cancellation contract | `cargo test --manifest-path src-tauri/Cargo.toml archive::tests -- --nocapture` | passed; 9 tests, cancellation granularity is explicit |
| Tauri Windows build | `npm run tauri -- build --config src-tauri/tauri.ci.conf.json --ci` | EXE, MSI and NSIS produced |
| Portable and hashes | `npm run package:portable`; `npm run verify:artifacts` | passed |

## Local artifacts

| Kind | File | Bytes | SHA-256 |
|---|---|---:|---|
| EXE | `src-tauri/target/release/moeplay.exe` | 40,979,968 | `DAD7280F80D91F1557091277D1B32FFAD2760A28D77B32FE0AD8B8A8E924FE6E` |
| MSI | `src-tauri/target/release/bundle/msi/萌游 MoeGame_0.12.1_x64_zh-CN.msi` | 15,478,784 | `0C847B20988D428458660B4650F1867AC8772E48571F365589FBA85752EB7932` |
| NSIS | `src-tauri/target/release/bundle/nsis/萌游 MoeGame_0.12.1_x64-setup.exe` | 11,563,902 | `E36499E5D65E3794D27B3E9AD9EA0EDC4B17953160E660DF2CB8893D993D4C55` |
| Portable ZIP | `src-tauri/target/release/bundle/portable/moeplay_0.12.1_x64-portable.zip` | 15,184,805 | `CD0C562F400DD25F08E71D01C4ED1D1295688AC89F496022C851F7F8654DD86C` |
| CycloneDX SBOM | `src-tauri/target/release/bundle/sbom.cdx.json` | 264,908 | `15134A083BDA6F0AF1C531EB90CF94852965BCC8C8D9805E1A95C15F7F15A7F3` |
| Build metadata | `src-tauri/target/release/bundle/build-metadata.json` | 526 | `85016942CFF9799CFC2968DEF831EED5C369EA56D8D2F20E7B01209B2D61D76D` |

The machine has no release signing key configured, so these local artifacts are unsigned. The tag release workflow remains responsible for Tauri updater signatures and `latest.json`.

> Artifact freshness: the bundle hashes above were captured before the HTTP download control changes documented below. Rebuild EXE/MSI/NSIS/portable before distributing this worktree.

## Real HTTP download control evidence

The download tests start a temporary `127.0.0.1` TCP HTTP server and do not use the public internet.

- **Pause:** the downloader stops committing chunks behind a write barrier, checkpoints the on-disk byte length, drops the active HTTP response, and returns only after the worker has stopped. The test asserts no file growth after pause and a control acknowledgement below two seconds.
- **Resume:** a new request uses `Range: bytes=<checkpoint>-`; `206 Partial Content` is accepted only when `Content-Range` starts at the exact checkpoint. The reconstructed file is compared byte-for-byte with the fixture.
- **Cancel:** cancellation wakes pending connect/read/rate-limit waits, prevents any later chunk write, drops the HTTP response, and leaves the task terminal/non-resumable. The server must observe the connection closing within two seconds and the file must remain unchanged after cancel returns.
- **Archive boundary:** pause is rejected once extraction/import begins. Cancel is recorded, but ZIP can observe it only between entries and a single entry copy remains synchronous; 7z/rar extraction uses a blocking external process and cannot stop until that process exits. Partial extracted files are not rolled back. Persistent job metadata records `archiveCancellationDeferred` and the limitation text.

Current integration note: the focused downloader/archive suites passed after these changes, and the latest production-code `cargo check` plus `cargo clippy --lib --all-features -D warnings` pass. A later `--all-targets --all-features` test rerun is temporarily blocked by an unrelated concurrent edit in `src-tauri/src/db_sqlite.rs` where the new 10k benchmark prints four variables outside their scope. This download task did not modify that shared file; rerun the complete suite after its owner finishes the benchmark edit.

## Proven foundations

- P0 command/version/database/dashboard/AI endpoint/SecretStore fixes are implemented and covered by direct tests.
- SQLite schema v5, migration ledger, Activity/Progress/ProviderHealth/BackgroundJob/AI-result repositories and persistent TaskQueue are compiled and tested.
- Activity v2 and Library v2 service/UI foundations are integrated with fallback paths.
- Anime/Comic Provider v2 are integrated into their main pages, with internal playback/reader flows and persisted non-secret configuration restored after restart.
- AI Gateway contracts/schema/governance plus the six-command structured-task runtime and three-command validated change-set apply/undo boundary are integrated; the Discovery workbench now uses literal `invokeCmd` calls and the unified start/status/result/cancel contract.
- Redacted diagnostics/log retention, save-restore preview, SBOM and build metadata evidence are implemented.
- Anime and Comic provider registries, command APIs, origin-bound secrets and contract fixtures are integrated and tested.
- UI v2 primitives, semantic tokens and reduced-motion initialization are integrated.

## Remaining release and plan gaps

1. Run a signed tag build with the real updater private key and verify `latest.json` plus updater signatures.
2. Perform an installed `0.12.0 → 0.12.1` upgrade and rollback exercise on a disposable Windows profile while preserving a populated library/database.
3. Anime live dual-source acceptance passes with TvTFun and aafun. Comic public-source live acceptance passes with Baozi, DM5 and 1kkk; Komga/Kavita self-hosted acceptance still requires reachable user services.
4. Complete real-source acceptance and production hardening of the newly integrated Anime/Comic Provider v2 flows.
5. Complete remaining AI live-provider production acceptance.
6. Complete remaining UI v2/Big Picture migration, backup/diagnostics executor convergence, long-duration constrained-network download soak, and the full performance/security evidence matrix required by the MASTER PLAN.

These gaps mean the local build is usable as a development release candidate, but they prevent claiming the entire 0.12.1 → 0.13.0 plan complete.
