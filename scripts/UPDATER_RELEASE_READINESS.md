# Updater / release readiness

Starting with v0.13.8, every official MoePlay release must support signed automatic updates.
Unsigned or installer-only releases are forbidden.

## Mandatory signing gate

- `TAURI_SIGNING_PRIVATE_KEY` is required for every `v*` tag workflow. If it is missing, the workflow fails before building or publishing a release.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` is passed only to the signed build and may be empty only for an unencrypted private key.
- Official builds enable `bundle.createUpdaterArtifacts=true` through the release-only Tauri config overlay.
- The workflow requires the updater archive, detached signature and `latest.json` to exist and match the public key configured in `src-tauri/tauri.conf.json`.
- The GitHub Release stays draft until every updater verification step passes. The final workflow step then publishes it automatically.
- No private key or password may appear in the repository, logs, fixtures, SBOM, build metadata or release assets.

## Client behavior

Desktop clients use the signed manifest at:

`https://github.com/Cicada0719/moeplay-tauri/releases/latest/download/latest.json`

The application checks this endpoint at startup and through Settings → Check for updates. Tauri verifies the detached signature before installation.

## Verification commands

```powershell
npm run test:updater-policy
node --test scripts/verify-updater-artifacts.node-test.mjs
npm run verify:updater

# Signed release workspace: latest.json and matching updater + .sig are mandatory
node scripts/verify-updater-artifacts.mjs --require --artifacts-dir src-tauri/target/release latest.json
```

The release workflow is the only supported way to publish an official version. Manually uploaded unsigned installers must never be marked as the latest release.
