use super::unsupported_target;
use super::util::{
    array, encode_segment, f32_value, i64_value, parse_number_from_title, provider_parse_error,
    required_id, string, tracked,
};
use super::{
    AuthConfig, ChapterDto, ChapterIdentity, ChapterSort, ComicFuture, ComicHttpClient,
    ComicHttpConfig, ComicResult, ComicSourceAdapter, HealthTracker, LanguageSource, LibraryDto,
    ProbeDto, ResolveRequest, SearchRequest, SeriesDetailDto, SeriesDto,
};
use crate::domain::{
    ProviderCapability, ProviderManifest, ProviderTrust, ResolvedTarget, ResourceKind,
};
use serde_json::Value;

const PROVIDER_ID: &str = "komga";

pub struct KomgaConnector {
    http: ComicHttpClient,
    health: HealthTracker,
}

impl KomgaConnector {
    pub fn new(base_url: impl Into<String>, auth: AuthConfig) -> ComicResult<Self> {
        Ok(Self {
            http: ComicHttpClient::new(ComicHttpConfig {
                base_url: base_url.into(),
                auth,
            })?,
            health: HealthTracker::new(PROVIDER_ID),
        })
    }

    pub fn parse_libraries(value: &Value) -> Vec<LibraryDto> {
        array(value, &["content", "libraries"])
            .iter()
            .map(|item| LibraryDto {
                id: required_id(item, &["id"]),
                name: string(item, &["name"]).unwrap_or_else(|| "Komga library".to_string()),
                path: string(item, &["root", "path"]),
                kind: Some("komga".to_string()),
            })
            .collect()
    }

    pub fn parse_series(value: &Value) -> Vec<SeriesDto> {
        array(value, &["content", "series"])
            .iter()
            .filter_map(Self::parse_series_one)
            .collect()
    }

    pub fn parse_series_one(item: &Value) -> Option<SeriesDto> {
        let id = required_id(item, &["id"]);
        if id.is_empty() {
            return None;
        }
        let metadata = item.get("metadata").unwrap_or(item);
        Some(SeriesDto {
            id,
            provider_id: PROVIDER_ID.to_string(),
            library_id: string(item, &["libraryId", "library.id"]),
            title: string(metadata, &["title"])
                .or_else(|| string(item, &["name"]))
                .unwrap_or_default(),
            sort_title: string(metadata, &["sortTitle"]),
            summary: string(metadata, &["summary"]),
            cover_url: None,
            language: string(metadata, &["language"]),
            year: i64_value(metadata, &["releaseYear", "year"]).map(|n| n as i32),
        })
    }

    pub fn parse_chapters(series_id: &str, value: &Value) -> Vec<ChapterDto> {
        array(value, &["content", "books"])
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let id = required_id(item, &["id"]);
                if id.is_empty() {
                    return None;
                }
                let metadata = item.get("metadata").unwrap_or(item);
                let title = string(metadata, &["title"])
                    .or_else(|| string(item, &["name"]))
                    .unwrap_or_else(|| format!("Chapter {}", index + 1));
                let volume = f32_value(metadata, &["volumeNumber", "volume"]);
                let chapter = f32_value(metadata, &["number", "chapterNumber", "numberSort"])
                    .or_else(|| parse_number_from_title(&title));
                Some(ChapterDto {
                    identity: ChapterIdentity {
                        provider_id: PROVIDER_ID.to_string(),
                        series_id: series_id.to_string(),
                        volume_id: None,
                        chapter_id: id.clone(),
                        stable_key: format!("{PROVIDER_ID}:{series_id}:{id}"),
                    },
                    title: title.clone(),
                    sort: ChapterSort {
                        volume_number: volume,
                        chapter_number: chapter,
                        ordinal: Some(index as i64),
                        title,
                    },
                    language: string(metadata, &["language"]),
                    language_source: LanguageSource::Provider,
                    page_count: i64_value(item, &["pageCount", "pages"]).map(|n| n as u32),
                    published_at: string(metadata, &["releaseDate", "date"]),
                    file_name: string(item, &["url", "fileName"]),
                })
            })
            .collect()
    }

    pub fn parse_page_urls(
        base: &ComicHttpClient,
        book_id: &str,
        value: &Value,
    ) -> ComicResult<Vec<String>> {
        let mut urls = Vec::new();
        for (index, item) in array(value, &["pages", "content"]).iter().enumerate() {
            let number = i64_value(item, &["number", "pageNumber"]).unwrap_or(index as i64);
            let url = base.endpoint(&format!(
                "/api/v1/books/{}/pages/{number}",
                encode_segment(book_id)
            ))?;
            urls.push(url.to_string());
        }
        if urls.is_empty() {
            return Err(provider_parse_error(
                PROVIDER_ID,
                "resolve",
                "Komga book pages response contained no pages",
            ));
        }
        Ok(urls)
    }

    async fn fetch_libraries(&self) -> ComicResult<Vec<LibraryDto>> {
        let value = self
            .http
            .get_json(PROVIDER_ID, "libraries", "/api/v1/libraries", &[])
            .await?;
        Ok(Self::parse_libraries(&value))
    }
}

impl ComicSourceAdapter for KomgaConnector {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest {
            id: PROVIDER_ID.to_string(),
            name: "Komga".to_string(),
            resource_kinds: vec![ResourceKind::Comic],
            capabilities: vec![
                ProviderCapability::Probe,
                ProviderCapability::Search,
                ProviderCapability::Detail,
                ProviderCapability::Children,
                ProviderCapability::Resolve,
                ProviderCapability::ProgressRead,
            ],
            trust: ProviderTrust::SelfHosted,
            version: "batch2".to_string(),
            enabled: true,
            requires_auth: true,
            allowed_hosts: vec![self
                .http
                .base_url()
                .host_str()
                .unwrap_or_default()
                .to_string()],
        }
    }

    fn health(&self, operation: &str) -> crate::domain::ProviderHealth {
        self.health.snapshot(operation)
    }

    fn probe(&self) -> ComicFuture<'_, ProbeDto> {
        Box::pin(tracked(&self.health, "probe", async move {
            let libraries = self.fetch_libraries().await?;
            Ok(ProbeDto {
                provider_id: PROVIDER_ID.to_string(),
                reachable: true,
                authenticated: true,
                server_version: None,
                latency_ms: None,
                libraries,
            })
        }))
    }

    fn libraries(&self) -> ComicFuture<'_, Vec<LibraryDto>> {
        Box::pin(tracked(&self.health, "libraries", self.fetch_libraries()))
    }

    fn search(&self, request: SearchRequest) -> ComicFuture<'_, Vec<SeriesDto>> {
        Box::pin(tracked(&self.health, "search", async move {
            let query = vec![
                ("search", request.query),
                ("page", request.page.saturating_sub(1).to_string()),
                ("size", request.page_size.clamp(1, 100).to_string()),
            ];
            let value = self
                .http
                .get_json(PROVIDER_ID, "search", "/api/v1/series", &query)
                .await?;
            Ok(Self::parse_series(&value))
        }))
    }

    fn detail(&self, series_id: String) -> ComicFuture<'_, SeriesDetailDto> {
        Box::pin(tracked(&self.health, "detail", async move {
            let value = self
                .http
                .get_json(
                    PROVIDER_ID,
                    "detail",
                    &format!("/api/v1/series/{}", encode_segment(&series_id)),
                    &[],
                )
                .await?;
            let series = Self::parse_series_one(&value).ok_or_else(|| {
                provider_parse_error(PROVIDER_ID, "detail", "Komga series response has no id")
            })?;
            let metadata = value.get("metadata").unwrap_or(&value);
            Ok(SeriesDetailDto {
                series,
                alternate_titles: Vec::new(),
                genres: array(metadata, &["genres"])
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect(),
                status: string(metadata, &["status"]),
                total_chapters: i64_value(&value, &["booksCount", "bookCount"]).map(|n| n as u32),
            })
        }))
    }

    fn chapters(&self, series_id: String) -> ComicFuture<'_, Vec<ChapterDto>> {
        Box::pin(tracked(&self.health, "chapters", async move {
            let value = self
                .http
                .get_json(
                    PROVIDER_ID,
                    "chapters",
                    &format!("/api/v1/series/{}/books", encode_segment(&series_id)),
                    &[("sort", "metadata.numberSort,asc".to_string())],
                )
                .await?;
            Ok(Self::parse_chapters(&series_id, &value))
        }))
    }

    fn resolve(&self, request: ResolveRequest) -> ComicFuture<'_, ResolvedTarget> {
        Box::pin(tracked(&self.health, "resolve", async move {
            let value = self
                .http
                .get_json(
                    PROVIDER_ID,
                    "resolve",
                    &format!(
                        "/api/v1/books/{}/pages",
                        encode_segment(&request.chapter_id)
                    ),
                    &[],
                )
                .await?;
            let pages = Self::parse_page_urls(&self.http, &request.chapter_id, &value)?;
            let headers = pages
                .first()
                .and_then(|url| url.parse().ok())
                .map(|url| self.http.auth_headers_for(&url))
                .unwrap_or_default();
            if pages.iter().all(|url| {
                url.parse()
                    .ok()
                    .map(|parsed| self.http.same_origin(&parsed))
                    .unwrap_or(false)
            }) {
                Ok(ResolvedTarget::ImagePages { pages, headers })
            } else {
                Ok(unsupported_target(
                    "Komga returned a page URL outside the fixed origin",
                    crate::domain::ProviderErrorKind::PolicyBlocked,
                ))
            }
        }))
    }
}
