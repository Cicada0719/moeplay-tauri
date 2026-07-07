//! 刮削共享工具函数
//!
//! 提供标题匹配置信度计算、HTML 清洗、重试逻辑等被多个数据源复用的函数。

use regex::Regex;
use reqwest::Client;
use tokio::time::{sleep, Duration};

use super::error::ScrapeError;

/// 请求重试配置
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 800;

/// 全局代理设置（由 settings 写入，build_client 读取）
static SCRAPER_PROXY: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

pub fn set_proxy(proxy: Option<String>) {
    *SCRAPER_PROXY.write().unwrap() = proxy;
}

fn apply_proxy(builder: reqwest::ClientBuilder) -> Result<reqwest::ClientBuilder, ScrapeError> {
    let guard = SCRAPER_PROXY.read().unwrap();
    if let Some(url) = guard.as_ref().filter(|u| !u.trim().is_empty()) {
        let proxy = reqwest::Proxy::all(url)
            .map_err(|e| ScrapeError::Network(format!("proxy config error: {e}")))?;
        Ok(builder.proxy(proxy))
    } else {
        Ok(builder)
    }
}

/// 创建带 User-Agent 和超时的 HTTP 客户端
pub fn build_client() -> Result<Client, ScrapeError> {
    let builder = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("MoeGame/1.0 (Galgame Manager)");
    apply_proxy(builder)?
        .build()
        .map_err(|e| ScrapeError::Network(e.to_string()))
}

/// 创建带日语 Accept-Language 的 HTTP 客户端
pub fn build_client_ja() -> Result<Client, ScrapeError> {
    let builder = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("MoeGame/1.0 (Galgame Manager)")
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::ACCEPT_LANGUAGE,
                reqwest::header::HeaderValue::from_static("ja,en;q=0.9,zh;q=0.8"),
            );
            headers
        });
    apply_proxy(builder)?
        .build()
        .map_err(|e| ScrapeError::Network(e.to_string()))
}

/// 带重试的 GET 请求，返回响应字节
///
/// 对网络错误和 5xx 状态码进行重试（最多 `MAX_RETRIES` 次），
/// 4xx 错误（除 429 外）不重试。
pub async fn fetch_with_retry(url: &str) -> Result<reqwest::Response, ScrapeError> {
    let client = build_client()?;
    let mut last_error = String::new();

    for attempt in 1..=MAX_RETRIES {
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(resp);
                }
                last_error = format!("HTTP {}", status.as_u16());
                if status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(ScrapeError::Api {
                        status: status.as_u16(),
                        body: String::new(),
                    });
                }
                if attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
            Err(e) => {
                last_error = e.to_string();
                if attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
        }
    }

    Err(ScrapeError::Network(last_error))
}

/// 带重试的 GET 请求，返回响应文本
pub async fn fetch_text_with_retry(url: &str) -> Result<String, ScrapeError> {
    let resp = fetch_with_retry(url).await?;
    resp.text()
        .await
        .map_err(|e| ScrapeError::Network(e.to_string()))
}

/// 带重试的 GET 请求（使用日语客户端），返回响应文本
pub async fn fetch_text_ja(url: &str) -> Result<String, ScrapeError> {
    let client = build_client_ja()?;
    let mut last_error = String::new();

    for attempt in 1..=MAX_RETRIES {
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return resp
                        .text()
                        .await
                        .map_err(|e| ScrapeError::Network(e.to_string()));
                }
                last_error = format!("HTTP {}", status.as_u16());
                if status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(ScrapeError::Api {
                        status: status.as_u16(),
                        body: String::new(),
                    });
                }
                if attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
            Err(e) => {
                last_error = e.to_string();
                if attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
        }
    }

    Err(ScrapeError::Network(last_error))
}

/// 带重试的 GET 请求，返回响应字节（用于非 UTF-8 编码页面）
pub async fn fetch_bytes_with_retry(url: &str) -> Result<Vec<u8>, ScrapeError> {
    let client = build_client()?;
    let mut last_error = String::new();

    for attempt in 1..=MAX_RETRIES {
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return resp
                        .bytes()
                        .await
                        .map(|b| b.to_vec())
                        .map_err(|e| ScrapeError::Network(e.to_string()));
                }
                last_error = format!("HTTP {}", status.as_u16());
                if status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(ScrapeError::Api {
                        status: status.as_u16(),
                        body: String::new(),
                    });
                }
                if attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
            Err(e) => {
                last_error = e.to_string();
                if attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            }
        }
    }

    Err(ScrapeError::Network(last_error))
}

/// 计算标题匹配置信度 (0.0 ~ 1.0)
///
/// 算法：
/// - 完全匹配 → 1.0
/// - 一方包含另一方 → 0.85
/// - 基于字符覆盖率的近似匹配 → 最多 0.8
pub fn confidence(query: &str, result: &str) -> f64 {
    if query.is_empty() || result.is_empty() {
        return 0.0;
    }

    let q = query.trim().to_lowercase();
    let r = result.trim().to_lowercase();

    if q == r {
        return 1.0;
    }
    if r.contains(&q) || q.contains(&r) {
        return 0.85;
    }

    // 简化版 Jaro-Winkler：基于字符覆盖率
    let shorter = if q.len() <= r.len() { &q } else { &r };
    let longer = if q.len() <= r.len() { &r } else { &q };

    let match_count = shorter.chars().filter(|ch| longer.contains(*ch)).count();
    (match_count as f64) / (longer.len() as f64) * 0.8
}

/// 清理 HTML 标签和实体
pub fn clean_html(html: &str) -> String {
    if html.is_empty() {
        return html.to_string();
    }

    // 移除 HTML 标签
    let re = Regex::new(r"<[^>]+>").unwrap();
    let cleaned = re.replace_all(html, "").trim().to_string();

    // 解码常见 HTML 实体
    let cleaned = cleaned
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'");

    // 解码 Unicode 数字实体 &#xxxxx;
    let re_entity = Regex::new(r"&#(\d+);").unwrap();
    let cleaned = re_entity
        .replace_all(&cleaned, |caps: &regex::Captures| {
            caps.get(1)
                .and_then(|m| m.as_str().parse::<u32>().ok())
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| caps.get(0).unwrap().as_str().to_string())
        })
        .to_string();

    // 解码十六进制实体 &#xXXXX;
    let re_hex = Regex::new(r"&#x([0-9a-fA-F]+);").unwrap();
    let cleaned = re_hex
        .replace_all(&cleaned, |caps: &regex::Captures| {
            caps.get(1)
                .and_then(|m| u32::from_str_radix(m.as_str(), 16).ok())
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| caps.get(0).unwrap().as_str().to_string())
        })
        .to_string();

    cleaned.trim().to_string()
}

/// 从日期字符串提取年份（如 "2024-03-15" → 2024）
pub fn extract_year(date: &str) -> Option<u32> {
    date.get(0..4)?.parse::<u32>().ok()
}

/// 截断字符串
#[allow(dead_code)]
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
