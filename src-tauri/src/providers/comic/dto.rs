use crate::domain::{ProviderErrorKind, ResolvedTarget};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub library_id: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            library_id: None,
            page: 1,
            page_size: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDto {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeDto {
    pub provider_id: String,
    pub reachable: bool,
    pub authenticated: bool,
    pub server_version: Option<String>,
    pub latency_ms: Option<u64>,
    pub libraries: Vec<LibraryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDto {
    pub id: String,
    pub provider_id: String,
    pub library_id: Option<String>,
    pub title: String,
    pub sort_title: Option<String>,
    pub summary: Option<String>,
    pub cover_url: Option<String>,
    pub language: Option<String>,
    pub year: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetailDto {
    pub series: SeriesDto,
    pub alternate_titles: Vec<String>,
    pub genres: Vec<String>,
    pub status: Option<String>,
    pub total_chapters: Option<u32>,
}

/// The identity is deliberately separate from display/sort fields. A provider
/// may rename a chapter without changing the stable identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterIdentity {
    pub provider_id: String,
    pub series_id: String,
    pub volume_id: Option<String>,
    pub chapter_id: String,
    pub stable_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSort {
    pub volume_number: Option<f32>,
    pub chapter_number: Option<f32>,
    pub ordinal: Option<i64>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageSource {
    Provider,
    Manifest,
    Filename,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterDto {
    pub identity: ChapterIdentity,
    pub title: String,
    pub sort: ChapterSort,
    pub language: Option<String>,
    pub language_source: LanguageSource,
    pub page_count: Option<u32>,
    pub published_at: Option<String>,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRequest {
    pub series_id: String,
    pub chapter_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub index: u32,
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedComic {
    pub request: ResolveRequest,
    pub target: ResolvedTarget,
    pub pages: Vec<PageDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalManifest {
    pub version: u32,
    pub provider_id: Option<String>,
    pub library_name: Option<String>,
    pub series: Vec<LocalManifestSeries>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalManifestSeries {
    pub id: String,
    pub title: String,
    pub path: String,
    pub language: Option<String>,
    pub chapters: Vec<LocalManifestChapter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalManifestChapter {
    pub id: String,
    pub title: String,
    pub path: String,
    pub volume_number: Option<f32>,
    pub chapter_number: Option<f32>,
    pub language: Option<String>,
}

pub fn unsupported_target(reason: impl Into<String>, kind: ProviderErrorKind) -> ResolvedTarget {
    ResolvedTarget::Unsupported {
        reason: reason.into(),
        error_kind: kind,
    }
}
