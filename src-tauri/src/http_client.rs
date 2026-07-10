// 萌游 MoeGame · HTTP 客户端统一构造
//
// 默认强制启用 TLS 证书校验，仅在显式设置 MOEGAME_INSECURE_TLS=1 时关闭。

use std::sync::OnceLock;
use std::time::Duration;

/// Canonical application User-Agent, derived from the package version.
pub fn app_user_agent() -> String {
    format!("MoePlay/{}", env!("CARGO_PKG_VERSION"))
}

/// Canonical application User-Agent with a stable feature label.
pub fn app_user_agent_with_context(context: &str) -> String {
    let context = context.trim();
    if context.is_empty() {
        app_user_agent()
    } else {
        format!("{} {context}", app_user_agent())
    }
}

/// Browser-compatible User-Agent for sources that reject non-browser clients.
pub fn browser_user_agent() -> String {
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0 Safari/537.36 {}",
        app_user_agent()
    )
}

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
pub fn build_reqwest_client(timeout_secs: u64, user_agent: impl AsRef<str>) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(user_agent.as_ref())
        .danger_accept_invalid_certs(insecure_tls_enabled())
        .build()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_user_agents_follow_the_package_version() {
        let expected = format!("MoePlay/{}", env!("CARGO_PKG_VERSION"));
        assert_eq!(app_user_agent(), expected);
        assert_eq!(
            app_user_agent_with_context("manga"),
            format!("{expected} manga")
        );
        assert!(browser_user_agent().ends_with(&expected));
    }
}
