// 萌游 MoeGame · HTTP 客户端统一构造
//
// 默认强制启用 TLS 证书校验，仅在显式设置 MOEGAME_INSECURE_TLS=1 时关闭。

use std::sync::OnceLock;
use std::time::Duration;

/// 判断是否通过环境变量禁用 TLS 证书校验。
///
/// 首次调用时会读取 `MOEGAME_INSECURE_TLS`，若值为 `1` 则启用不安全模式，
/// 并打印一次警告日志。
pub fn insecure_tls_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let enabled = std::env::var("MOEGAME_INSECURE_TLS")
            .map(|v| v.trim() == "1")
            .unwrap_or(false);
        if enabled {
            tracing::warn!(
                "[security] MOEGAME_INSECURE_TLS=1：已禁用 TLS 证书校验（仅供调试使用）"
            );
        }
        enabled
    })
}

/// 构造一个默认启用 TLS 校验的 reqwest 客户端。
///
/// `timeout_secs` 为整体请求超时，`user_agent` 为 User-Agent 头。
/// 当 `MOEGAME_INSECURE_TLS=1` 时，会调用 `.danger_accept_invalid_certs(true)`。
pub fn build_reqwest_client(timeout_secs: u64, user_agent: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(user_agent)
        .danger_accept_invalid_certs(insecure_tls_enabled())
        .build()
        .unwrap_or_default()
}
