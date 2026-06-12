// 统一错误类型 + 公共工具函数

/// 应用统一错误类型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Database(String),
    #[error("网络错误: {0}")]
    Network(String),
    #[error("文件系统错误: {0}")]
    FileSystem(String),
    #[error("刮削错误: {0}")]
    Scrape(String),
    #[error("下载错误: {0}")]
    Download(String),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("未找到: {0}")]
    NotFound(String),
    #[error("无效输入: {0}")]
    InvalidInput(String),
    #[error("权限错误: {0}")]
    Permission(String),
    #[error("序列化错误: {0}")]
    Serialization(String),
    #[error("未知错误: {0}")]
    Unknown(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::FileSystem(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Serialization(e.to_string())
    }
}

/// Tauri 命令返回类型
pub type CommandResult<T> = Result<T, String>;

/// 将 AppError 转换为 Tauri 命令返回的 String
impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}

// ============ 公共工具函数 ============

/// 安全文件名（移除非法字符）
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// 截断字符串
pub fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

/// 截断字符串带省略号
pub fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// 格式化文件大小
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

/// 格式化时长（秒 → 时:分:秒）
pub fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        format!("{}s", seconds % 60)
    }
}

/// 重试包装器（指数退避）
pub async fn retry<F, T, E>(mut f: F, max_retries: u32) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut last_err = None;
    for attempt in 0..=max_retries {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = Some(e);
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
        }
    }
    Err(last_err.unwrap())
}

/// 同步版本的重试
pub fn retry_sync<F, T, E>(mut f: F, max_retries: u32) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut last_err = None;
    for attempt in 0..=max_retries {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = Some(e);
                if attempt < max_retries {
                    std::thread::sleep(std::time::Duration::from_secs(2u64.pow(attempt)));
                }
            }
        }
    }
    Err(last_err.unwrap())
}

/// 唯一文件路径（避免覆盖）
pub fn unique_path(path: &std::path::Path) -> std::path::PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let parent = path.parent().unwrap_or(std::path::Path::new("."));

    for i in 1..1000 {
        let new_path = parent.join(format!("{}_{}{}", stem, i, ext));
        if !new_path.exists() {
            return new_path;
        }
    }
    path.to_path_buf()
}

/// 检查文件是否为可执行文件（Windows: .exe/.bat/.cmd）
pub fn is_executable(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com")
    } else {
        false
    }
}
