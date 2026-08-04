//! WebDAV 凭据的 keyring 存取（FR-09 / spec §3.5、PRD §4.2 安全）。
//!
//! 密码只进系统安全存储（Windows Credential Manager / macOS Keychain /
//! Linux Secret Service），配置表 / 配置文件绝不落明文。
//!
//! Linux 无 Secret Service 环境：`get_password` 返回 `Ok(None)` 并记 warn 日志
//! （不 panic），`save_password`/`delete_password` 返回错误提示用户。

use crate::sync::SyncError;

const KEYRING_SERVICE: &str = "moeplay-tauri";
const KEYRING_ACCOUNT: &str = "webdav";

/// 保存 WebDAV 授权密码到系统密钥环。
pub fn save_password(password: &str) -> Result<(), SyncError> {
    if password.is_empty() {
        return Err(SyncError::Server("密码不能为空".to_string()));
    }
    let entry = entry()?;
    entry
        .set_password(password)
        .map_err(|error| map_keyring_error("保存凭据失败", error))
}

/// 读取 WebDAV 授权密码。未保存 → `Ok(None)`。
///
/// Linux 无 Secret Service：记 warn 日志并返回 `Ok(None)`，不 panic。
pub fn get_password() -> Result<Option<String>, SyncError> {
    let entry = match entry() {
        Ok(entry) => entry,
        Err(_) => {
            tracing::warn!("WebDAV keyring 初始化失败（可能无系统密钥环）");
            return Ok(None);
        }
    };
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) if cfg!(target_os = "linux") => {
            tracing::warn!("Linux 无法访问 Secret Service，WebDAV 凭据视为未配置");
            Ok(None)
        }
        Err(error) => Err(map_keyring_error("读取凭据失败", error)),
    }
}

/// 删除 WebDAV 授权密码。凭据不存在视为成功（幂等）。
pub fn delete_password() -> Result<(), SyncError> {
    let entry = entry()?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(map_keyring_error("删除凭据失败", error)),
    }
}

fn entry() -> Result<keyring::Entry, SyncError> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|error| map_keyring_error("访问系统密钥环失败", error))
}

/// 把 keyring 错误映射为 `SyncError::Server`（不泄露任何密钥材料）。
fn map_keyring_error(context: &str, error: keyring::Error) -> SyncError {
    SyncError::Server(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 冒烟测试：CI 无系统密钥环时被环境变量门控跳过（spec §6.4）。
    /// 本地三平台手动验证：设置 `MOEPLAY_TEST_KEYRING=1` 后运行。
    #[test]
    #[ignore = "requires an OS credential store; set MOEPLAY_TEST_KEYRING=1 to run manually"]
    fn keyring_save_get_delete_roundtrip() {
        if std::env::var("MOEPLAY_TEST_KEYRING").as_deref() != Ok("1") {
            return;
        }
        let _ = delete_password();
        assert_eq!(get_password().expect("get resolves"), None);

        save_password("s3cret-password").expect("save succeeds");
        assert_eq!(
            get_password().expect("get resolves").as_deref(),
            Some("s3cret-password")
        );

        delete_password().expect("delete succeeds");
        assert_eq!(get_password().expect("get resolves"), None);
    }
}
