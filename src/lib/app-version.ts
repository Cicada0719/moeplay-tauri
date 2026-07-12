import packageMetadata from "../../package.json";

// package.json is the frontend source of truth. The release gate verifies it
// matches Cargo.toml and tauri.conf.json before every official build.
export const APP_VERSION = packageMetadata.version;
