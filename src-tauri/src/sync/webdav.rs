//! 轻量 WebDAV 客户端（FR-09 / spec §3.4）。
//!
//! - 认证：HTTP Basic，`base64` 手动拼 `Authorization` 头（凭据只进请求头，不落盘、
//!   不进日志）；请求全走 HTTPS（`https_only(true)`），绝不设置
//!   `danger_accept_invalid_certs`。
//! - 超时：连接 10s / 总 60s。
//! - 错误分类：401/403 → `Auth`；超时/DNS/连接 → `Network`；5xx → `Server`；
//!   `PUT` 的 412 → `PutStatus::PreconditionFailed`（供上层乐观并发重试）。

use crate::sync::SyncError;
use base64::Engine;
use reqwest::Method;
use reqwest::StatusCode;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(60);

/// `PUT` 的结果。`PreconditionFailed` 表示 412（远端已被并发修改），
/// 上层据此重拉远端重新合并一次（spec §4.2 step g）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutStatus {
    Ok,
    PreconditionFailed,
}

/// 条件写入策略。
///
/// 同步路径只允许 `IfMatch` 或 `CreateOnly`，保留 `Unconditional` 仅供旧的
/// 低层调用者兼容；新建远端文件必须使用 `If-None-Match: *`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PutCondition {
    Unconditional,
    IfMatch(String),
    CreateOnly,
}

pub struct WebDavClient {
    client: reqwest::Client,
    base_url: String,
    auth_header: String,
}

impl WebDavClient {
    /// 生产构造：强制 HTTPS。
    pub fn new(base_url: &str, username: &str, password: &str) -> Result<Self, SyncError> {
        Self::build(base_url, username, password, true)
    }

    /// 测试构造：允许 http（仅用于本地 wiremock/回环 mock 集成测试）。
    ///
    /// 生产路径一律走 [`WebDavClient::new`]；本构造不会放宽证书校验。
    #[doc(hidden)]
    pub fn new_allow_http(
        base_url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, SyncError> {
        Self::build(base_url, username, password, false)
    }

    fn build(
        base_url: &str,
        username: &str,
        password: &str,
        https_only: bool,
    ) -> Result<Self, SyncError> {
        let base_url = base_url.trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(SyncError::Server("WebDAV 地址不能为空".to_string()));
        }
        let mut builder = reqwest::Client::builder()
            .timeout(TOTAL_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(concat!("moeplay/", env!("CARGO_PKG_VERSION")));
        builder = builder.https_only(https_only);
        let client = builder.build().map_err(map_reqwest_error)?;

        let encoded =
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
        Ok(Self {
            client,
            base_url,
            auth_header: format!("Basic {encoded}"),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_matches('/'))
    }

    /// MKCOL 创建目录；目录已存在（405）时视为成功。
    pub async fn ensure_dir(&self, dir: &str) -> Result<(), SyncError> {
        let response = self
            .client
            .request(mkcol_method(), self.url(dir))
            .header(reqwest::header::AUTHORIZATION, &self.auth_header)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        match response.status() {
            status if status.is_success() => Ok(()),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::CONFLICT => Ok(()),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(SyncError::Auth),
            status if status.is_server_error() => {
                Err(SyncError::Server(format!("创建目录失败: HTTP {status}")))
            }
            status => Err(SyncError::Server(format!("创建目录失败: HTTP {status}"))),
        }
    }

    /// GET；404 → `Ok(None)`；成功返回 `(body, ETag)`。
    pub async fn get(&self, path: &str) -> Result<Option<(Vec<u8>, Option<String>)>, SyncError> {
        let response = self
            .client
            .get(self.url(path))
            .header(reqwest::header::AUTHORIZATION, &self.auth_header)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        match response.status() {
            StatusCode::NOT_FOUND => Ok(None),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(SyncError::Auth),
            status if status.is_success() => {
                let etag = response
                    .headers()
                    .get(reqwest::header::ETAG)
                    .and_then(|value| value.to_str().ok())
                    .map(|value| value.to_string());
                let body = response.bytes().await.map_err(map_reqwest_error)?;
                Ok(Some((body.to_vec(), etag)))
            }
            status if status.is_server_error() => {
                Err(SyncError::Server(format!("下载失败: HTTP {status}")))
            }
            status => Err(SyncError::Server(format!("下载失败: HTTP {status}"))),
        }
    }

    /// 兼容旧调用的 PUT；提供 ETag 时使用 `If-Match`，未提供时保持旧的无条件写入语义。
    /// 新的同步路径必须调用 [`Self::put_with_condition`]，以便新建文件使用
    /// `If-None-Match: *` 防止覆盖；412 → `PreconditionFailed`。
    pub async fn put(
        &self,
        path: &str,
        body: Vec<u8>,
        if_match: Option<&str>,
    ) -> Result<PutStatus, SyncError> {
        let condition = if_match
            .map(|etag| PutCondition::IfMatch(etag.to_string()))
            .unwrap_or(PutCondition::Unconditional);
        self.put_with_condition(path, body, condition).await
    }

    /// 条件 PUT。`CreateOnly` 使用 `If-None-Match: *`，避免覆盖并发创建的文件。
    pub async fn put_with_condition(
        &self,
        path: &str,
        body: Vec<u8>,
        condition: PutCondition,
    ) -> Result<PutStatus, SyncError> {
        let mut request = self
            .client
            .put(self.url(path))
            .header(reqwest::header::AUTHORIZATION, &self.auth_header)
            .body(body);
        match condition {
            PutCondition::Unconditional => {}
            PutCondition::IfMatch(etag) => {
                request = request.header(reqwest::header::IF_MATCH, etag);
            }
            PutCondition::CreateOnly => {
                request = request.header(reqwest::header::IF_NONE_MATCH, "*");
            }
        }
        let response = request.send().await.map_err(map_reqwest_error)?;
        match response.status() {
            StatusCode::PRECONDITION_FAILED => Ok(PutStatus::PreconditionFailed),
            status if status.is_success() => Ok(PutStatus::Ok),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(SyncError::Auth),
            status if status.is_server_error() => {
                Err(SyncError::Server(format!("上传失败: HTTP {status}")))
            }
            status => Err(SyncError::Server(format!("上传失败: HTTP {status}"))),
        }
    }

    /// PROPFIND 探测资源是否存在。
    pub async fn propfind_exists(&self, path: &str) -> Result<bool, SyncError> {
        let response = self
            .client
            .request(propfind_method(), self.url(path))
            .header(reqwest::header::AUTHORIZATION, &self.auth_header)
            .header("Depth", "0")
            .send()
            .await
            .map_err(map_reqwest_error)?;
        match response.status() {
            StatusCode::NOT_FOUND => Ok(false),
            status if status.is_success() => Ok(true),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(SyncError::Auth),
            status => Err(SyncError::Server(format!("探测失败: HTTP {status}"))),
        }
    }
}

fn map_reqwest_error(error: reqwest::Error) -> SyncError {
    if error.is_timeout() {
        SyncError::Network("请求超时".to_string())
    } else if error.is_connect() {
        SyncError::Network(format!("网络连接失败: {error}"))
    } else if error.is_builder() {
        SyncError::Server(format!("请求构建失败: {error}"))
    } else {
        SyncError::Network(format!("网络请求失败: {error}"))
    }
}

/// `reqwest::Method` 未预定义 WebDAV 的 MKCOL，按字节构造（编译期常量，绝不会失败）。
fn mkcol_method() -> Method {
    Method::from_bytes(b"MKCOL").expect("MKCOL is a valid HTTP method")
}

/// `reqwest::Method` 未预定义 WebDAV 的 PROPFIND，按字节构造。
fn propfind_method() -> Method {
    Method::from_bytes(b"PROPFIND").expect("PROPFIND is a valid HTTP method")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_for(server: &MockServer) -> WebDavClient {
        WebDavClient::new_allow_http(&server.uri(), "user", "pass").expect("client builds")
    }

    #[tokio::test]
    async fn get_200_returns_body_and_etag() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/moeplay-sync/history.json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(b"[1,2,3]", "application/json")
                    .insert_header("ETag", "\"abc123\""),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = client_for(&server);
        let (body, etag) = client
            .get("moeplay-sync/history.json")
            .await
            .expect("get succeeds")
            .expect("found");
        assert_eq!(body, vec![b'[', b'1', b',', b'2', b',', b'3', b']']);
        assert_eq!(etag.as_deref(), Some("\"abc123\""));
    }

    #[tokio::test]
    async fn get_404_returns_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/moeplay-sync/history.json"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = client_for(&server);
        assert!(client
            .get("moeplay-sync/history.json")
            .await
            .expect("get succeeds")
            .is_none());
    }

    #[tokio::test]
    async fn get_401_maps_to_auth() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let error = client.get("moeplay-sync/history.json").await.unwrap_err();
        assert!(matches!(error, SyncError::Auth));
    }

    #[tokio::test]
    async fn get_500_maps_to_server() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let error = client.get("moeplay-sync/history.json").await.unwrap_err();
        assert!(matches!(error, SyncError::Server(_)));
    }

    #[tokio::test]
    async fn put_carries_if_match_header_and_412_flagged() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/moeplay-sync/history.json"))
            .and(header("if-match", "\"abc123\""))
            .respond_with(ResponseTemplate::new(412))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_for(&server);
        let status = client
            .put(
                "moeplay-sync/history.json",
                b"data".to_vec(),
                Some("\"abc123\""),
            )
            .await
            .expect("put resolves");
        assert_eq!(status, PutStatus::PreconditionFailed);
    }

    #[tokio::test]
    async fn put_success_returns_ok() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/moeplay-sync/history.json"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let status = client
            .put("moeplay-sync/history.json", b"data".to_vec(), None)
            .await
            .expect("put resolves");
        assert_eq!(status, PutStatus::Ok);
    }

    #[tokio::test]
    async fn create_only_put_carries_if_none_match_star() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/moeplay-sync/history.json"))
            .and(header("if-none-match", "*"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_for(&server);
        let status = client
            .put_with_condition(
                "moeplay-sync/history.json",
                b"data".to_vec(),
                PutCondition::CreateOnly,
            )
            .await
            .expect("create-only put resolves");
        assert_eq!(status, PutStatus::Ok);
    }

    #[tokio::test]
    async fn put_401_maps_to_auth() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let error = client
            .put("moeplay-sync/history.json", b"data".to_vec(), None)
            .await
            .unwrap_err();
        assert!(matches!(error, SyncError::Auth));
    }

    #[tokio::test]
    async fn mkc_ol_existing_dir_405_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("MKCOL"))
            .and(path("/moeplay-sync"))
            .respond_with(ResponseTemplate::new(405))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_for(&server);
        client.ensure_dir("moeplay-sync").await.expect("405 is ok");
    }

    #[tokio::test]
    async fn mkc_ol_new_dir_201_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("MKCOL"))
            .and(path("/moeplay-sync"))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_for(&server);
        client.ensure_dir("moeplay-sync").await.expect("201 is ok");
    }

    #[tokio::test]
    async fn mkc_ol_unauthorized_maps_to_auth() {
        let server = MockServer::start().await;
        Mock::given(method("MKCOL"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let error = client.ensure_dir("moeplay-sync").await.unwrap_err();
        assert!(matches!(error, SyncError::Auth));
    }
}
