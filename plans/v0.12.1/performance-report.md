# MoePlay 0.12.1 Performance Evidence

> Captured: 2026-07-10 on the local Windows x64 development host.  
> Branch: `codex/v0.12.1-foundation`.

## Database/library budgets

| Scenario | Direct evidence | Budget | Result |
|---|---|---:|---|
| Import 1,000 games | `db_sqlite::tests::benchmark_1000_games` | < 5,000 ms | passed |
| List 1,000 games | same test | < 500 ms | passed |
| Search 1,000 games | same test | < 200 ms | passed |
| Import 5,000 games | `db_sqlite::tests::benchmark_5000_games_release_candidate_budget` | < 15,000 ms | passed locally in the 0.44 s total test run |
| List 5,000 games | same test | < 2,000 ms | passed |
| Search 5,000 games | same test | < 750 ms | passed |
| List 10,000 games | `db_sqlite::tests::benchmark_10000_games_nightly` (`--release --ignored`) | <= 4,000 ms | **47 ms** local |
| Search 10,000 games | same test, 20 distributed queries | P95 <= 150 ms | **66 ms P95** local |
| Single-game update | same test, 100 transactions | P95 <= 50 ms; P99 <= 150 ms | **3 ms P95 / 4 ms P99** local |
| v2 → v5 migration, 10,000 games | same test | <= 30,000 ms | **9 ms** local |
| DB open, no migration | same test, 20 reopens | P95 <= 250 ms | **2 ms P95** local |

The tests use the real SQLite repository and serialized `Game` model, not an in-memory JavaScript approximation. The 10k gate is also scheduled in `.github/workflows/nightly.yml`; local values are reference evidence and CI logs provide runner-specific history. ASCII search now narrows candidates in SQLite before applying the unchanged Rust field whitelist, while non-ASCII search retains the legacy Unicode path.

## Activity budgets

| Scenario | Direct evidence | Budget | Result |
|---|---|---:|---|
| Insert/aggregate/page 20,000 activity events | `repositories::activity::activity_20k_aggregate_and_page_benchmark_stays_bounded` | aggregate + first page <= 800 ms | passed |
| Keyset pagination + scalar summary over 20,000 events | `services::activity::twenty_thousand_events_page_with_keyset_and_scalar_summary` | bounded 100-row pages; exact totals | passed |
| Query plan uses started-at index | `EXPLAIN QUERY PLAN` assertion in repository benchmark | index required | passed |

## Frontend bundle budgets

`npm run verify:bundle-budget` currently reports:

- 22 JavaScript chunks.
- Total JavaScript: 1,844,497 bytes (budget 2,100,000).
- Largest chunk: 869,996 bytes (budget 920,000).
- Anime page: 652,766 bytes (budget 700,000).
- Comic page: 72,892 bytes (budget 100,000).

The verifier and its negative fixtures are required in CI/release. These are regression ceilings, not final optimization targets; the Anime and application-shell chunks remain priority code-splitting work.

## Deterministic HTTP pause/cancel budget

| Scenario | Direct evidence | Budget | Result |
|---|---|---:|---|
| Pause a live HTTP stream | `downloader::tests::real_http_pause_closes_connection_and_resume_uses_range` using a temporary loopback server | acknowledgement and file/network stop < 2 s | passed; file length remains fixed after return |
| Resume from durable checkpoint | same test | exact Range offset and byte-identical final file | passed; `Content-Range` is validated before append |
| Cancel a live HTTP stream | `downloader::tests::real_http_cancel_stops_file_writes_and_drops_connection` | acknowledgement and server-observed disconnect < 2 s | passed; no later file writes |

These tests are deterministic functional budget checks, not a long-duration throughput benchmark. They intentionally avoid public-network variance.

## Comic long-chapter / prefetch memory soak

`npm.cmd run test:comic-memory-soak` now starts an isolated Vite server and Chrome process, injects deterministic Tauri/provider mocks, opens the real `ComicReader`, traverses a 180-page chapter through its eager/lazy prefetch path, and repeats four open/scroll/close cycles. It records Chrome DevTools JS heap/DOM counters and the complete browser process-tree RSS, then exits non-zero when a threshold fails. Optional `--json` and `--markdown` paths preserve machine-readable evidence.

Default run captured on 2026-07-10:

| Check | Budget | Result |
|---|---:|---:|
| Initial eager window | exactly 3 images | **3** |
| Initial image requests before traversal | <= 18 | **12** |
| Full long chapter traversal | 180 / 180 requests | **180 / 180** |
| Closed-state JS heap growth, cycle 1 -> 4 | <= 24 MiB | **+0.24 MiB** |
| Closed-state browser-tree RSS growth, cycle 1 -> 4 | <= 160 MiB | **-7.72 MiB** |
| Peak browser-tree RSS over detail baseline | <= 512 MiB | **+64.09 MiB** |
| Closed-state DOM node drift | <= 500 | **0** |

The deterministic soak passed. These are repeatable regression ceilings around the actual frontend reader and browser loading behavior; they are not a substitute for a multi-hour manual OS-pressure profile with real high-resolution pages.

## Windows real Tauri startup/RSS measurement harness

`scripts/measure-tauri-startup.ps1` accepts `-ExePath` and `-Repeat` (default 10), creates a fresh process for every iteration, and writes JSON plus Markdown. Definitions are explicit:

- **Cold start:** script-created process to its first visible top-level window. It is process-cold; Windows filesystem/GPU/WebView2 caches are not flushed or falsely described as cold.
- **First content:** first real `PrintWindow` frame meeting pixel-diversity thresholds. A probe timeout remains missing evidence rather than being replaced with process/window timing.
- **Idle RSS:** median working set for the Tauri root and descendants discovered from that exact script-created PID during the final half of the idle interval.
- **Cleanup safety:** the script records PID + creation identity and stops only that owned process tree; it never kills by executable name.

Example:

```powershell
npm.cmd run measure:tauri-startup -- -ExePath .\src-tauri\target\release\moeplay.exe -Repeat 10
```

The native window/RSS/output path was self-tested with a one-repeat `mspaint.exe` probe (cold window 532.97 ms, first content 580.42 ms, idle RSS 47.67 MiB, current monitor DPI 125%). **That probe validates the measurement machinery only and is not MoePlay performance evidence.** A direct attempt against the local 0.12.1 release executable correctly produced a failed/missing-evidence record because an installed MoePlay instance was already running and the new app process exited through the single-instance path before creating its own window. The existing user process was not terminated or included in the measurements.

### Windows scale evidence status

The startup script records the actual DPI of the measured window but does not change or emulate Windows display scale. Therefore:

- 125% MoePlay layout/readability/interaction evidence: **still requires a real interactive MoePlay run and manual sign-off**.
- 150% MoePlay layout/readability/interaction evidence: **still requires a real interactive MoePlay run and manual sign-off**.
- The 125% `mspaint.exe` self-test above must not be cited as MoePlay scale acceptance.

## Remaining performance evidence

- A 10-repeat real MoePlay Tauri run with the installed instance closed, preserving cold-window, first-content P50/P95 and idle RSS JSON/Markdown evidence.
- Separate real interactive MoePlay evidence and manual sign-off at both Windows 125% and 150% scale; no synthetic or unrelated-app result is accepted.
- Multi-hour comic soak with real high-resolution pages under OS memory pressure, beyond the deterministic regression gate above.
- Multi-hour download retry/pause/cancel and process-recovery soak under constrained disk/network conditions; the deterministic loopback behavior is now covered.

These remaining items prevent claiming the complete performance section of the MASTER PLAN, but the 5k/10k library, 20k activity, bundle-budget and deterministic ComicReader memory gates now have direct automated evidence.
