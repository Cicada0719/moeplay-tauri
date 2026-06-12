//! 刮削错误类型定义

use std::fmt;

/// 刮削操作可能产生的错误
#[derive(Debug)]
pub enum ScrapeError {
    /// 网络请求失败
    Network(String),
    /// 响应解析失败
    Parse(String),
    /// API 返回错误状态码
    Api { status: u16, body: String },
    /// 请求频率限制
    RateLimited,
    /// 未找到结果
    NotFound,
    /// 配置错误
    Config(String),
}

impl fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "网络错误: {}", msg),
            Self::Parse(msg) => write!(f, "解析错误: {}", msg),
            Self::Api { status, body } => {
                if body.is_empty() {
                    write!(f, "API 错误 (HTTP {})", status)
                } else {
                    write!(f, "API 错误 (HTTP {}): {}", status, body)
                }
            }
            Self::RateLimited => write!(f, "请求过于频繁，请稍后重试"),
            Self::NotFound => write!(f, "未找到结果"),
            Self::Config(msg) => write!(f, "配置错误: {}", msg),
        }
    }
}

impl std::error::Error for ScrapeError {}

/// 兼容旧代码中使用的 `Result<T, String>`
impl From<ScrapeError> for String {
    fn from(e: ScrapeError) -> Self {
        e.to_string()
    }
}
