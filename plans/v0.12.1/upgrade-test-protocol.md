# Installed Upgrade / Rollback Protocol — 0.12.0 → 0.12.1

This protocol is a **release blocker**. A tag workflow may build and upload a draft, but it must not publish the release or promote `latest.json` until the signed report is attached.

## Test environment

- Disposable Windows 11 x64 VM or clean local user profile.
- Official 0.12.0 NSIS/MSI installer and the exact 0.12.1 candidate artifacts from `release-manifest.json`.
- Network capture/log collection configured to redact credentials.
- No developer database, `%APPDATA%\moeplay`, keyring entries or installed MoePlay instance present before the run.

## Seed state on 0.12.0

1. Install 0.12.0 and launch it once.
2. Add at least 20 games including one file launch, one Steam URI and one missing executable.
3. Create play sessions, favorite/tag metadata, one save snapshot and one queued/retryable task.
4. Configure AI and one media connector credential in the OS credential manager. Record only configured status and account identifiers, never values.
5. Export a hash/inventory of the database and user-visible state.

## Upgrade exercise

1. Install the exact 0.12.1 candidate over 0.12.0.
2. Launch and wait for schema migration to complete.
3. Confirm schema v5 and migration ledger versions 1–5/checksums.
4. Verify all seeded games, metadata, play sessions, progress, save snapshots and task history.
5. Verify SecretStore configured statuses remain available and no secret appears in SQLite/export/diagnostic ZIP.
6. Open Library v2, Activity v2, Anime Provider v2 and Comic Provider v2; confirm legacy fallback remains usable.
7. Restart twice and confirm provider configurations/tasks recover without duplicated records.
8. Run diagnostics export and inspect it with sentinel scanning.

## Rollback / recovery exercise

- Do **not** open schema v5 directly with an older binary as the normal rollback path.
- Restore the pre-upgrade database backup into a fresh 0.12.0 profile and verify the seeded state.
- Reinstall 0.12.1 and verify forward migration remains idempotent.
- If updater promotion was tested, remove/restore the test `latest.json` immediately after the run.

## Required report fields

```json
{
  "fromVersion": "0.12.0",
  "toVersion": "0.12.1",
  "candidateManifestSha256": "...",
  "environment": "Windows 11 x64 build ...",
  "installed": true,
  "launched": true,
  "schemaVersion": 5,
  "migrationLedgerVerified": true,
  "libraryPreserved": true,
  "activityPreserved": true,
  "saveSnapshotsPreserved": true,
  "secretReferencesPreserved": true,
  "tasksPreserved": true,
  "providerRestartRecoveryVerified": true,
  "diagnosticRedactionVerified": true,
  "rollbackRecoveryVerified": true,
  "updaterSignatureVerified": true,
  "authenticodeDecision": "verified|documented_exception",
  "passed": true,
  "operator": "...",
  "completedAt": "RFC3339"
}
```

Any false/missing field keeps the release draft. The operator must attach logs/screenshots that contain no credential values and the SHA-256 of every installer used.

