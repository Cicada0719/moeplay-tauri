//! 哔咔(Picacg) HTTP 客户端 — HMAC-SHA256 鉴权 + reqwest
//! 参考: pikapika / PicACG 开源实现

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

// ── 常量 ──────────────────────────────────────────────────────────────────

const API_KEY: &str = "C69BAF41DA5ABD1FFEDC6D2FEA56B";
const SECRET_KEY: &str = "~d}$Q7$eIni=V)9\\RK/P.RM4;9[7|@/CA}b~OW!3?EV`:<>M7pddUBL5n|0/*Cn";
const NONCE: &str = "4ce7a7aa759b40f794d189a88b84aba8";
pub const BASE_URL: &str = "https://picaapi.picacomic.com";

type HmacSha256 = Hmac<Sha256>;

// ── 托管状态 ─────────────────────────────────────────────────────────────

pub struct ComicState {
    pub token: Mutex<String>,
}

impl Default for ComicState {
    fn default() -> Self {
        Self {
            token: Mutex::new(String::new()),
        }
    }
}

// ── 共享 HTTP 客户端（复用连接池，TLS 兼容哔咔镜像）────────────────────

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
            .danger_accept_invalid_certs(crate::http_client::insecure_tls_enabled())
            .build()
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to build custom HTTP client, falling back to default");
                reqwest::Client::new()
            })
    })
}

// ── HMAC 签名 ─────────────────────────────────────────────────────────────

fn unix_ts() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .to_string()
}

pub fn sign(path: &str, timestamp: &str, method: &str) -> Result<String, String> {
    let msg = format!("{}{}{}{}{}", path, timestamp, NONCE, method, API_KEY).to_lowercase();
    let mut mac = HmacSha256::new_from_slice(SECRET_KEY.as_bytes())
        .map_err(|e| format!("HMAC key invalid: {}", e))?;
    mac.update(msg.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn build_headers(
    sig_path: &str,
    method: &str,
    token: &str,
) -> Result<reqwest::header::HeaderMap, String> {
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use std::str::FromStr;

    let ts = unix_ts();
    let sig = sign(sig_path, &ts, method)?;
    let mut map = HeaderMap::new();

    for (k, v) in &[
        ("accept", "application/vnd.picacomic.com.v1+json"),
        ("user-agent", "okhttp/3.8.1"),
        ("content-type", "application/json; charset=UTF-8"),
        ("api-key", API_KEY),
        ("app-build-version", "45"),
        ("app-platform", "android"),
        ("app-uuid", "defaultUuid"),
        ("app-version", "2.2.1.3.3.4"),
        ("nonce", NONCE),
        ("app-channel", "2"),
        ("image-quality", "original"),
    ] {
        if let (Ok(n), Ok(v)) = (HeaderName::from_str(k), HeaderValue::from_str(v)) {
            map.insert(n, v);
        }
    }
    for (k, v) in &[("time", ts.as_str()), ("signature", sig.as_str())] {
        if let (Ok(n), Ok(v)) = (HeaderName::from_str(k), HeaderValue::from_str(v)) {
            map.insert(n, v);
        }
    }
    if !token.is_empty() {
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_str("authorization"),
            HeaderValue::from_str(token),
        ) {
            map.insert(n, v);
        }
    }
    Ok(map)
}

async fn parse_resp(resp: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = resp.status().as_u16();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
        format!(
            "解析响应失败 (HTTP {}): {}",
            status,
            &text[..text.len().min(200)]
        )
    })?;
    match status {
        200 => Ok(json),
        400 => Err(json["message"].as_str().unwrap_or("请求错误").to_string()),
        401 => Err("登录已过期，请重新登录哔咔账号".to_string()),
        _ => Err(format!(
            "服务器返回 HTTP {}: {}",
            status,
            json["message"].as_str().unwrap_or("未知错误")
        )),
    }
}

// ── GET / POST / PUT 辅助 ────────────────────────────────────────────────

pub async fn api_get(
    path: &str,
    token: &str,
    params: &[(&str, String)],
) -> Result<serde_json::Value, String> {
    let mut sorted: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    sorted.sort_by_key(|(k, _)| *k);
    let qs = sorted
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let sig_path = if qs.is_empty() {
        path.to_string()
    } else {
        format!("{}?{}", path, qs)
    };
    let url = format!("{}/{}", BASE_URL, sig_path);
    let resp = client()
        .get(&url)
        .headers(build_headers(&sig_path, "GET", token)?)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;
    parse_resp(resp).await
}

pub async fn api_post(
    path: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/{}", BASE_URL, path);
    let resp = client()
        .post(&url)
        .headers(build_headers(path, "POST", token)?)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;
    parse_resp(resp).await
}

pub async fn api_put(
    path: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/{}", BASE_URL, path);
    let resp = client()
        .put(&url)
        .headers(build_headers(path, "PUT", token)?)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;
    parse_resp(resp).await
}

// ── 图片 URL 构建 ─────────────────────────────────────────────────────────

pub fn build_image_url(file_server: &str, path: &str) -> String {
    if file_server.is_empty() || path.is_empty() {
        return String::new();
    }
    let base = file_server.trim_end_matches('/');
    let p = path.trim_start_matches('/');
    if base.contains("/static") {
        format!("{}/{}", base, p)
    } else {
        format!("{}/static/{}", base, p)
    }
}

// ── 数据模型（序列化后发往前端）───────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ComicThumb {
    #[serde(rename = "fileServer")]
    pub file_server: String,
    pub path: String,
}

#[derive(Serialize, Clone)]
pub struct ComicSummary {
    pub id: String,
    pub title: String,
    pub author: String,
    pub thumb_url: String,
    pub categories: Vec<String>,
    pub likes_count: i64,
    pub total_views: i64,
    pub eps_count: i64,
    pub finished: bool,
}

impl ComicSummary {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        let thumb = &v["thumb"];
        Some(ComicSummary {
            id: v["_id"].as_str()?.to_string(),
            title: v["title"].as_str().unwrap_or("").to_string(),
            author: v["author"].as_str().unwrap_or("").to_string(),
            thumb_url: build_image_url(
                thumb["fileServer"].as_str().unwrap_or(""),
                thumb["path"].as_str().unwrap_or(""),
            ),
            categories: v["categories"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect(),
            likes_count: v["likesCount"].as_i64().unwrap_or(0),
            total_views: v["totalViews"]
                .as_i64()
                .or_else(|| v["viewsCount"].as_i64())
                .unwrap_or(0),
            eps_count: v["epsCount"].as_i64().unwrap_or(0),
            finished: v["finished"].as_bool().unwrap_or(false),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct ComicDetail {
    pub id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub thumb_url: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub likes_count: i64,
    pub total_views: i64,
    pub eps_count: i64,
    pub pages_count: i64,
    pub finished: bool,
    pub is_liked: bool,
    pub is_favourite: bool,
    pub chinese_team: String,
    pub comments_count: i64,
    pub allow_comment: bool,
    pub updated_at: String,
    pub created_at: String,
}

impl ComicDetail {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        let thumb = &v["thumb"];
        Some(ComicDetail {
            id: v["_id"].as_str()?.to_string(),
            title: v["title"].as_str().unwrap_or("").to_string(),
            author: v["author"].as_str().unwrap_or("").to_string(),
            description: v["description"].as_str().unwrap_or("").to_string(),
            thumb_url: build_image_url(
                thumb["fileServer"].as_str().unwrap_or(""),
                thumb["path"].as_str().unwrap_or(""),
            ),
            categories: v["categories"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect(),
            tags: v["tags"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect(),
            likes_count: v["likesCount"].as_i64().unwrap_or(0),
            total_views: v["totalViews"]
                .as_i64()
                .or_else(|| v["viewsCount"].as_i64())
                .unwrap_or(0),
            eps_count: v["epsCount"].as_i64().unwrap_or(0),
            pages_count: v["pagesCount"].as_i64().unwrap_or(0),
            finished: v["finished"].as_bool().unwrap_or(false),
            is_liked: v["isLiked"].as_bool().unwrap_or(false),
            is_favourite: v["isFavourite"].as_bool().unwrap_or(false),
            chinese_team: v["chineseTeam"].as_str().unwrap_or("").to_string(),
            comments_count: v["commentsCount"].as_i64().unwrap_or(0),
            allow_comment: v["allowComment"].as_bool().unwrap_or(true),
            updated_at: v["updated_at"].as_str().unwrap_or("").to_string(),
            created_at: v["created_at"].as_str().unwrap_or("").to_string(),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct ComicChapter {
    pub id: String,
    pub title: String,
    pub order: i64,
    pub updated_at: String,
}

impl ComicChapter {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        Some(ComicChapter {
            id: v["_id"].as_str()?.to_string(),
            title: v["title"].as_str().unwrap_or("").to_string(),
            order: v["order"].as_i64().unwrap_or(0),
            updated_at: v["updatedAt"]
                .as_str()
                .or_else(|| v["updated_at"].as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct ComicImage {
    pub id: String,
    pub url: String,
}

impl ComicImage {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        let media = &v["media"];
        Some(ComicImage {
            id: v["_id"].as_str()?.to_string(),
            url: build_image_url(
                media["fileServer"].as_str().unwrap_or(""),
                media["path"].as_str().unwrap_or(""),
            ),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct ComicCategory {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumb_url: String,
}

impl ComicCategory {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        if v["isWeb"].as_bool().unwrap_or(false) {
            return None;
        }
        let thumb = &v["thumb"];
        Some(ComicCategory {
            id: v["_id"].as_str().unwrap_or("").to_string(),
            title: v["title"].as_str()?.to_string(),
            description: v["description"].as_str().unwrap_or("").to_string(),
            thumb_url: build_image_url(
                thumb["fileServer"].as_str().unwrap_or(""),
                thumb["path"].as_str().unwrap_or(""),
            ),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct ComicListPage {
    pub docs: Vec<ComicSummary>,
    pub total: i64,
    pub pages: i64,
    pub page: i64,
}

impl ComicListPage {
    pub fn from_value(v: &serde_json::Value) -> Self {
        let docs = v["docs"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(ComicSummary::from_value)
            .collect();
        ComicListPage {
            docs,
            total: v["total"].as_i64().unwrap_or(0),
            pages: v["pages"].as_i64().unwrap_or(1),
            page: v["page"].as_i64().unwrap_or(1),
        }
    }
}

// ── 用户 / 评论 模型 ─────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ComicUserProfile {
    pub id: String,
    pub name: String,
    pub email: String,
    pub avatar_url: String,
    pub title: String,
    pub slogan: String,
    pub level: i64,
    pub exp: i64,
    pub gender: String,
    pub is_punched: bool,
    pub character: String,
    pub created_at: String,
}

impl ComicUserProfile {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        let avatar = &v["avatar"];
        Some(ComicUserProfile {
            id: v["_id"].as_str()?.to_string(),
            name: v["name"].as_str().unwrap_or("").to_string(),
            email: v["email"].as_str().unwrap_or("").to_string(),
            avatar_url: build_image_url(
                avatar["fileServer"].as_str().unwrap_or(""),
                avatar["path"].as_str().unwrap_or(""),
            ),
            title: v["title"].as_str().unwrap_or("").to_string(),
            slogan: v["slogan"].as_str().unwrap_or("").to_string(),
            level: v["level"].as_i64().unwrap_or(0),
            exp: v["exp"].as_i64().unwrap_or(0),
            gender: v["gender"].as_str().unwrap_or("bot").to_string(),
            is_punched: v["isPunched"].as_bool().unwrap_or(false),
            character: v["character"].as_str().unwrap_or("").to_string(),
            created_at: v["created_at"].as_str().unwrap_or("").to_string(),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct ComicCommentUser {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub level: i64,
    pub title: String,
    pub role: String,
    pub character: String,
    pub slogan: String,
}

impl ComicCommentUser {
    pub fn from_value(v: &serde_json::Value) -> Self {
        let avatar = &v["avatar"];
        ComicCommentUser {
            id: v["_id"].as_str().unwrap_or("").to_string(),
            name: v["name"].as_str().unwrap_or("匿名").to_string(),
            avatar_url: build_image_url(
                avatar["fileServer"].as_str().unwrap_or(""),
                avatar["path"].as_str().unwrap_or(""),
            ),
            level: v["level"].as_i64().unwrap_or(0),
            title: v["title"].as_str().unwrap_or("").to_string(),
            role: v["role"].as_str().unwrap_or("").to_string(),
            character: v["character"].as_str().unwrap_or("").to_string(),
            slogan: v["slogan"].as_str().unwrap_or("").to_string(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ComicComment {
    pub id: String,
    pub content: String,
    pub user: ComicCommentUser,
    pub created_at: String,
    pub likes_count: i64,
    pub is_liked: bool,
    pub comments_count: i64,
    pub is_top: bool,
}

impl ComicComment {
    pub fn from_value(v: &serde_json::Value) -> Option<Self> {
        Some(ComicComment {
            id: v["_id"].as_str()?.to_string(),
            content: v["content"].as_str().unwrap_or("").to_string(),
            user: ComicCommentUser::from_value(&v["_user"]),
            created_at: v["created_at"].as_str().unwrap_or("").to_string(),
            likes_count: v["likesCount"].as_i64().unwrap_or(0),
            is_liked: v["isLiked"].as_bool().unwrap_or(false),
            comments_count: v["commentsCount"].as_i64().unwrap_or(0),
            is_top: v["isTop"].as_bool().unwrap_or(false),
        })
    }
}

#[derive(Serialize, Clone)]
pub struct CommentsPage {
    pub docs: Vec<ComicComment>,
    pub total: i64,
    pub pages: i64,
    pub page: i64,
}

impl CommentsPage {
    pub fn from_value(v: &serde_json::Value) -> Self {
        let docs = v["docs"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(ComicComment::from_value)
            .collect();
        CommentsPage {
            docs,
            total: v["total"].as_i64().unwrap_or(0),
            pages: v["pages"].as_i64().unwrap_or(1),
            page: v["page"].as_i64().unwrap_or(1),
        }
    }
}
