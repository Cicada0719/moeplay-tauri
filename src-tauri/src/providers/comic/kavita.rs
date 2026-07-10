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

const PROVIDER_ID: &str = "kavita";

pub struct KavitaConnector {
    http: ComicHttpClient,
    health: HealthTracker,
}

impl KavitaConnector {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> ComicResult<Self> {
        Ok(Self {
            http: ComicHttpClient::new(ComicHttpConfig {
                base_url: base_url.into(),
                auth: AuthConfig::ApiKey(api_key.into()),
            })?,
            health: HealthTracker::new(PROVIDER_ID),
        })
    }

    pub fn parse_libraries(value: &Value) -> Vec<LibraryDto> {
        array(value, &["libraries", "items"])
            .iter()
            .map(|item| LibraryDto {
                id: required_id(item, &["id", "libraryId"]),
                name: string(item, &["name", "title"])
                    .unwrap_or_else(|| "Kavita library".to_string()),
                path: string(item, &["path", "rootFolder"]),
                kind: string(item, &["type"]).or_else(|| Some("kavita".to_string())),
            })
            .collect()
    }

    pub fn parse_series(value: &Value) -> Vec<SeriesDto> {
        let items = array(value, &["series", "items", "results"]);
        items.iter().filter_map(Self::parse_series_one).collect()
    }

    pub fn parse_series_one(item: &Value) -> Option<SeriesDto> {
        let id = required_id(item, &["id", "seriesId"]);
        if id.is_empty() {
            return None;
        }
        Some(SeriesDto {
            id,
            provider_id: PROVIDER_ID.to_string(),
            library_id: string(item, &["libraryId"])
                .or_else(|| i64_value(item, &["libraryId"]).map(|id| id.to_string())),
            title: string(item, &["name", "title"]).unwrap_or_default(),
            sort_title: string(item, &["sortName"]),
            summary: string(item, &["summary", "description"]),
            cover_url: None,
            language: string(item, &["language", "localizedLanguage"]),
            year: i64_value(item, &["startYear", "year"]).map(|n| n as i32),
        })
    }

    pub fn parse_chapters(series_id: &str, value: &Value) -> Vec<ChapterDto> {
        let mut output = Vec::new();
        for (volume_index, volume) in array(value, &["volumes", "items"]).iter().enumerate() {
            let volume_id = required_id(volume, &["id", "volumeId"]);
            for (chapter_index, item) in array(volume, &["chapters", "items"]).iter().enumerate() {
                let id = required_id(item, &["id", "chapterId"]);
                if id.is_empty() {
                    continue;
                }
                let title = string(item, &["title", "name"])
                    .unwrap_or_else(|| format!("Chapter {}", chapter_index + 1));
                let chapter_number = f32_value(item, &["number", "chapterNumber", "sortNumber"])
                    .or_else(|| parse_number_from_title(&title));
                let volume_number = f32_value(volume, &["number", "volumeNumber"])
                    .or(Some(volume_index as f32 + 1.0));
                output.push(ChapterDto {
                    identity: ChapterIdentity {
                        provider_id: PROVIDER_ID.to_string(),
                        series_id: series_id.to_string(),
                        volume_id: Some(volume_id.clone()),
                        chapter_id: id.clone(),
                        stable_key: format!("{PROVIDER_ID}:{series_id}:{id}"),
                    },
                    title: title.clone(),
                    sort: ChapterSort {
                        volume_number,
                        chapter_number,
                        ordinal: Some((volume_index * 10_000 + chapter_index) as i64),
                        title,
                    },
                    language: string(item, &["language", "localizedLanguage"]),
                    language_source: LanguageSource::Provider,
                    page_count: i64_value(item, &["pages", "pageCount"]).map(|n| n as u32),
                    published_at: string(item, &["releaseDate", "date"]),
                    file_name: string(item, &["fileName", "url"]),
                });
            }
        }
        output
    }

    pub fn parse_page_count(value: &Value) -> Option<u32> {
        if let Some(count) = i64_value(value, &["pageCount", "pages"]) {
            return Some(count as u32);
        }
        Some(array(value, &["pages", "items"]).len() as u32).filter(|count| *count > 0)
    }

    async fn fetch_libraries(&self) -> ComicResult<Vec<LibraryDto>> {
        let value = self
            .http
            .get_json(PROVIDER_ID, "libraries", "/api/Library/libraries", &[])
            .await?;
        Ok(Self::parse_libraries(&value))
    }
}

impl ComicSourceAdapter for KavitaConnector {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest {
            id: PROVIDER_ID.to_string(),
            name: "Kavita".to_string(),
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
                ("queryString", request.query),
                ("includeChapterAndFiles", "false".to_string()),
            ];
            let value = self
                .http
                .get_json(PROVIDER_ID, "search", "/api/Search/search", &query)
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
                    &format!("/api/Series/{}", encode_segment(&series_id)),
                    &[],
                )
                .await?;
            let series = Self::parse_series_one(&value).ok_or_else(|| {
                provider_parse_error(PROVIDER_ID, "detail", "Kavita series response has no id")
            })?;
            Ok(SeriesDetailDto {
                series,
                alternate_titles: array(&value, &["alternateTitles"])
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect(),
                genres: array(&value, &["genres"])
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect(),
                status: string(&value, &["status"]),
                total_chapters: i64_value(&value, &["totalChapters", "chaptersCount"])
                    .map(|n| n as u32),
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
                    "/api/Series/volumes",
                    &[("seriesId", series_id.clone())],
                )
                .await?;
            Ok(Self::parse_chapters(&series_id, &value))
        }))
    }

    fn resolve(&self, request: ResolveRequest) -> ComicFuture<'_, ResolvedTarget> {
        Box::pin(tracked(&self.health, "resolve", async move {
            // Kavita exposes dimensions/count separately and serves individual
            // images through the read-only Reader/image endpoint.
            let dimensions = self
                .http
                .get_json(
                    PROVIDER_ID,
                    "resolve",
                    "/api/Reader/file-dimensions",
                    &[
                        ("chapterId", request.chapter_id.clone()),
                        ("extractPdf", "false".to_string()),
                    ],
                )
                .await?;
            let count = Self::parse_page_count(&dimensions).ok_or_else(|| {
                provider_parse_error(
                    PROVIDER_ID,
                    "resolve",
                    "Kavita chapter has no page dimensions",
                )
            })?;
            let mut pages = Vec::with_capacity(count as usize);
            for index in 0..count {
                let mut url = self.http.endpoint("/api/Reader/image")?;
                url.query_pairs_mut()
                    .append_pair("chapterId", &request.chapter_id)
                    .append_pair("page", &index.to_string())
                    .append_pair("extractPdf", "false");
                let _ = url;
                pages.push(url.to_string());
            }
            let parsed = pages
                .iter()
                .filter_map(|page| page.parse().ok())
                .collect::<Vec<_>>();
            if parsed.len() == pages.len() && parsed.iter().all(|url| self.http.same_origin(url)) {
                Ok(ResolvedTarget::ImagePages {
                    pages,
                    headers: self
                        .http
                        .auth_headers_for(parsed.first().expect("page count > 0")),
                })
            } else {
                Ok(unsupported_target(
                    "Kavita returned a page URL outside the fixed origin",
                    crate::domain::ProviderErrorKind::PolicyBlocked,
                ))
            }
        }))
    }
}
