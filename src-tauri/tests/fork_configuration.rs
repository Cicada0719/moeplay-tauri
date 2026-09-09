#![cfg(desktop)]

#[test]
fn unsigned_fork_configuration_initializes_the_updater_without_secrets() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let updater: tauri_plugin_updater::Config =
        serde_json::from_value(config["plugins"]["updater"].clone()).unwrap();
    assert!(updater.pubkey.is_empty());
    assert!(updater.endpoints.is_empty());
    assert_eq!(config["bundle"]["createUpdaterArtifacts"], false);
}
