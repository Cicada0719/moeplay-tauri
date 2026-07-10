# MoePlay 0.12.1 Implementation Status

> Branch: `codex/v0.12.1-foundation`  
> Started: 2026-07-10

## Batch 0 — Baseline & Safety

| ID | Work item | Owner | Status | Evidence |
|---|---|---|---|---|
| P0-CMD | Command/build/capability/frontend contract consistency | Agent Hume | completed | 328/328/328/328 aligned; verifier 4 tests pass |
| P0-DB | Fail-closed SQLite migration/open recovery | Agent Dalton | completed | recovery tests 3 pass; schema migration transactional |
| P0-STATS | Dashboard Rust/TS DTO and aggregation contract | Agent Pascal | completed | TS 4 tests + Rust 2 contract tests; 159 unit pass |
| P0-AI | AI Provider/key exposure/endpoint policy stopgap | Agent Ampere | completed | 16 AI security tests; provider DTO redacted |
| P0-VER | Version/tag/artifact consistency verifier | Integrator | completed | version bumped to 0.12.1; 6 manifests aligned |
| P0-SEC | System SecretStore migration | Integrator + agents | completed | AI/Steam/Bangumi/PicACG moved to OS keyring; sentinel tests pass |
| P0-PICACG | PicACG token localStorage → SecretStore | Agent Avicenna | completed | Rust 4 + frontend 4 tests pass |
| P0-BANGUMI | Bangumi token localStorage → SecretStore | Agent Kuhn | completed | Rust 5 + frontend 5 tests pass |
| P0-SETTINGS | AI/Steam settings plaintext → SecretStore | Agent Huygens | completed | full backend/frontend migration; 190 unit pass |
| P0-REL | Release workflow reuses quality gates and updater upgrade test | Integrator | in progress | draft-until-verified release + updater manifest/signature gate; installed upgrade test pending |

## Batch 1 — Shared Foundation

| ID | Work item | Status |
|---|---|---|
| F1-DOMAIN | Resource / Provider / Progress / Activity / Health contracts | completed foundation; Rust/TS wire contracts and provider health helpers tested |
| F1-MIG | Explicit SQLite migration ledger and golden DB fixtures | completed; schema v5 + provider config/AI result repositories + domain repositories + 20k test |
| F1-JOBS | Persistent BackgroundJob + real cancellation | completed foundation; loopback HTTP pause/Range-resume/cancel verified, archive deferred-cancel boundary explicit |
| F1-UI | PageShell / async states / media patterns / Drawer / motion / a11y foundation | completed; frozen UI-v2 API, nested focus traps, return focus, couch/reduced-motion contracts tested |

## Batch 2 — Four Pillars

| ID | Work item | Status |
|---|---|---|
| LIB | Library import/dedupe/provenance/launch | v2 foundation, commands, preview/apply UI and health panel integrated behind feature flag |
| ACT | Activity dashboard and continue center | v2 service/commands, backfill, continue rail and timeline dashboard integrated with legacy fallback |
| ANIME | Anime provider/orchestrator/internal playback | LocalMedia/Jellyfin main-page integration, secure playback/fallback and persisted config restore completed; TvTFun + aafun live search→roads dual-source acceptance passed |
| COMIC | Comic provider/self-hosted/reader | Local/Komga/Kavita main-page reader integration and persisted config restore completed; Baozi + DM5 + 1kkk public-source live search acceptance passed, self-hosted live acceptance pending |

## Batch 3 — Cross-cutting

| ID | Work item | Status |
|---|---|---|
| AI-V2 | AI Gateway and three product experiences | six-command task orchestration + three change-set commands + Discovery workbench integrated; validated results persist for 7 days across process restart; live provider acceptance pending |
| UI-V2 | Four-pillar UI migration + Big Picture | public API, App route/focus stack, scoped gamepad runtime and Big Picture focus zones complete; four production page migrations in progress |
| JOBS | Download/backup/diagnostics integration | HTTP download pause closes the connection and resumes via validated Range; cancel stops later writes and closes the stream; archive cancel is explicitly deferred/non-rollback; backup/diagnostics executor convergence and long soak pending |

## Batch 4 — Release

| ID | Work item | Status |
|---|---|---|
| PERF | 5k library / 20k activity / media performance | 10k library nightly + 20k activity + bundle budgets automated; media memory and real Tauri startup evidence pending |
| LIVE | Real source acceptance | Anime TvTFun+aafun dual-source and Comic Baozi+DM5+1kkk public-source live checks passed; self-hosted Komga/Kavita and installed-provider acceptance pending |
| UPDATE | Signed 0.12.0 → 0.12.1 updater test | pending |
| ARTIFACT | MSI/NSIS/portable/updater manifest verification | local bundles/hash manifest verified; CycloneDX SBOM + build metadata verified; signed updater artifact pending release key |

## Latest verified build — 2026-07-10

- Version contract: `0.12.1` across package, Cargo, lockfiles and Tauri config.
- Tauri command contract: `328 / 328 / 328 / 328`; 258 literal frontend invocations covered.
- Frontend: `svelte-check` 0 errors/0 warnings; 263 unit tests passed, 1 skipped; 3 Playwright smoke tests passed; production Vite build passed.
- Rust: format and Clippy passed with `--all-targets --all-features -D warnings`; all non-live tests passed, 2 live/environment tests remain ignored by design.
- Download controls: temporary loopback HTTP tests prove pause closes the stream, resume uses an exact validated Range checkpoint, cancel prevents later writes and the server observes disconnect within 2 seconds; archive cancellation remains deferred with no rollback.
- Bundles: release EXE, MSI, NSIS installer and portable ZIP built successfully; `release-manifest.json` contains byte sizes and SHA-256 hashes.
- Local artifacts are unsigned. Signed updater artifacts and the installed `0.12.0 → 0.12.1` upgrade/rollback exercise remain release blockers.
