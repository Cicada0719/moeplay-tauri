# Updater / release readiness

The tag workflow always creates or updates a **draft** GitHub Release. It never publishes automatically.

## Signing gate

- When `TAURI_SIGNING_PRIVATE_KEY` is available, the workflow overlays `bundle.createUpdaterArtifacts=true`, lets Tauri create signed updater archives, uploads `latest.json`, and requires `scripts/verify-updater-artifacts.mjs` to pass.
- When the private-key secret is absent, the workflow deliberately builds installer-only artifacts with updater generation disabled. The draft title/body and job summary mark the candidate as degraded, `latest.json` and `*.sig` files must be absent, and the final workflow step fails so the draft cannot be mistaken for an updater-ready release.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` is passed only to the signed build. It may be empty only when the configured private key is not encrypted.

No private key or real secret belongs in the repository, workflow YAML, fixtures, logs, release evidence, or artifacts.

## Verification commands

```powershell
# Unit coverage for version, HTTPS URL, minisign structure/key ID, and detached-signature matching
node --test scripts/verify-updater-artifacts.node-test.mjs

# Ordinary development checkout: succeeds with an explicit not-generated result
npm run verify:updater

# Signed release workspace: latest.json and matching local updater + .sig are mandatory
node scripts/verify-updater-artifacts.mjs --require --artifacts-dir src-tauri/target/release latest.json

# Unsigned/degraded workspace: updater metadata must not exist
node scripts/verify-updater-artifacts.mjs --expect-absent
```

`verify-release-artifacts.ps1` accepts `-UpdaterMode Auto|Required|Disabled` (or `UPDATER_RELEASE_MODE`) and records the mode/status in `release-manifest.json`.

## Manual promotion gate

Even a fully signed and verified draft must remain draft until an administrator attaches and approves the installed upgrade/rollback report. Signing readiness does not replace the installed-upgrade approval policy.
