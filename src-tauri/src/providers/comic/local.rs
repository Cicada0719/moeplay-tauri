use super::util::{parse_number_from_title, tracked};
use super::{
    ChapterDto, ChapterIdentity, ChapterSort, ComicFuture, ComicProviderError, ComicResult,
    ComicSourceAdapter, HealthTracker, LanguageSource, LibraryDto, LocalManifest,
    LocalManifestChapter, LocalManifestSeries, ProbeDto, ResolveRequest, SearchRequest,
    SeriesDetailDto, SeriesDto,
};
use crate::domain::{
    ProviderCapability, ProviderErrorKind, ProviderManifest, ProviderTrust, ResolvedTarget,
    ResourceKind,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif"];

pub struct LocalComicAdapter {
    root: PathBuf,
    manifest_data: LocalManifest,
    provider_id: String,
    health: HealthTracker,
}

impl LocalComicAdapter {
    pub fn new(root: impl AsRef<Path>) -> ComicResult<Self> {
        let root = root.as_ref().canonicalize().map_err(|e| {
            ComicProviderError::InvalidConfig(format!("local comic root is not accessible: {e}"))
        })?;
        if !root.is_dir() {
            return Err(ComicProviderError::InvalidConfig(
                "local comic root must be a directory".to_string(),
            ));
        }
        let manifest_path = root.join(".moeplay-comic.json");
        let manifest_data = if manifest_path.is_file() {
            let raw = fs::read_to_string(&manifest_path)
                .map_err(|e| ComicProviderError::InvalidConfig(e.to_string()))?;
            serde_json::from_str::<LocalManifest>(&raw).map_err(|e| {
                ComicProviderError::InvalidConfig(format!("invalid local comic manifest: {e}"))
            })?
        } else {
            Self::discover_manifest(&root)?
        };
        let provider_id = manifest_data
            .provider_id
            .clone()
            .unwrap_or_else(|| format!("local:{}", hash_path(&root)));
        let adapter = Self {
            root,
            manifest_data,
            provider_id: provider_id.clone(),
            health: HealthTracker::new(provider_id),
        };
        adapter.validate_manifest_paths()?;
        Ok(adapter)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn manifest_data(&self) -> &LocalManifest {
        &self.manifest_data
    }

    pub fn safe_join(&self, relative: &str) -> ComicResult<PathBuf> {
        let relative_path = Path::new(relative);
        if relative_path.is_absolute()
            || relative_path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(ComicProviderError::Security(
                "local path must be relative and cannot contain parent traversal".to_string(),
            ));
        }
        let candidate = self.root.join(relative_path);
        let canonical = candidate.canonicalize().map_err(|e| {
            ComicProviderError::Security(format!("local path is not accessible: {e}"))
        })?;
        if !canonical.starts_with(&self.root) {
            return Err(ComicProviderError::Security(
                "local path escapes configured root".to_string(),
            ));
        }
        Ok(canonical)
    }

    fn validate_manifest_paths(&self) -> ComicResult<()> {
        for series in &self.manifest_data.series {
            let _ = self.safe_join(&series.path)?;
            for chapter in &series.chapters {
                let _ = self.safe_join(&chapter.path)?;
            }
        }
        Ok(())
    }

    fn discover_manifest(root: &Path) -> ComicResult<LocalManifest> {
        let mut chapters = Vec::new();
        for entry in
            fs::read_dir(root).map_err(|e| ComicProviderError::InvalidConfig(e.to_string()))?
        {
            let entry = entry.map_err(|e| ComicProviderError::InvalidConfig(e.to_string()))?;
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("cbz"))
                    .unwrap_or(false)
            {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("chapter.cbz")
                    .to_string();
                chapters.push(LocalManifestChapter {
                    id: file_name.clone(),
                    title: path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&file_name)
                        .to_string(),
                    path: file_name,
                    volume_number: None,
                    chapter_number: parse_number_from_title(
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_default(),
                    ),
                    language: None,
                });
            }
        }
        Ok(LocalManifest {
            version: 1,
            provider_id: None,
            library_name: Some(
                root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Local Comics")
                    .to_string(),
            ),
            series: vec![LocalManifestSeries {
                id: "local-root".to_string(),
                title: root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Local Comics")
                    .to_string(),
                path: ".".to_string(),
                language: None,
                chapters,
            }],
        })
    }

    fn series(&self, id: &str) -> Option<&LocalManifestSeries> {
        self.manifest_data
            .series
            .iter()
            .find(|series| series.id == id)
    }
    fn chapter<'a>(
        &'a self,
        series: &'a LocalManifestSeries,
        id: &str,
    ) -> Option<&'a LocalManifestChapter> {
        series.chapters.iter().find(|chapter| chapter.id == id)
    }

    fn pages_for_path(&self, path: &Path) -> ComicResult<Vec<String>> {
        if path.is_file() {
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("cbz"))
                .unwrap_or(false)
            {
                return Err(ComicProviderError::Provider(crate::domain::ProviderError {
                    kind: ProviderErrorKind::Unsupported,
                    message: "CBZ extraction is intentionally outside the local adapter contract"
                        .to_string(),
                    retryable: false,
                    retry_after_ms: None,
                    provider_id: Some(self.provider_id.clone()),
                    operation: Some("resolve".to_string()),
                }));
            }
            return self.file_url(path).map(|url| vec![url]);
        }
        if !path.is_dir() {
            return Err(ComicProviderError::Security(
                "local chapter path must be a file or directory".to_string(),
            ));
        }
        let mut files = Vec::new();
        for entry in fs::read_dir(path).map_err(|e| ComicProviderError::Security(e.to_string()))? {
            let entry = entry.map_err(|e| ComicProviderError::Security(e.to_string()))?;
            let child = entry
                .path()
                .canonicalize()
                .map_err(|e| ComicProviderError::Security(e.to_string()))?;
            if !child.starts_with(&self.root) {
                return Err(ComicProviderError::Security(
                    "image directory contains a path outside root".to_string(),
                ));
            }
            if child.is_file()
                && child
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| {
                        IMAGE_EXTENSIONS
                            .iter()
                            .any(|allowed| e.eq_ignore_ascii_case(allowed))
                    })
                    .unwrap_or(false)
            {
                files.push(child);
            }
        }
        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        if files.is_empty() {
            return Err(ComicProviderError::Provider(crate::domain::ProviderError {
                kind: ProviderErrorKind::Unsupported,
                message: "image directory contains no supported pages".to_string(),
                retryable: false,
                retry_after_ms: None,
                provider_id: Some(self.provider_id.clone()),
                operation: Some("resolve".to_string()),
            }));
        }
        files.into_iter().map(|path| self.file_url(&path)).collect()
    }

    fn file_url(&self, path: &Path) -> ComicResult<String> {
        Url::from_file_path(path)
            .map(|url| url.to_string())
            .map_err(|_| {
                ComicProviderError::Security("cannot convert local page to a file URL".to_string())
            })
    }
}

impl ComicSourceAdapter for LocalComicAdapter {
    fn manifest(&self) -> ProviderManifest {
        ProviderManifest {
            id: self.provider_id.clone(),
            name: self
                .manifest_data
                .library_name
                .clone()
                .unwrap_or_else(|| "Local Comics".to_string()),
            resource_kinds: vec![ResourceKind::Comic],
            capabilities: vec![
                ProviderCapability::Probe,
                ProviderCapability::Search,
                ProviderCapability::Detail,
                ProviderCapability::Children,
                ProviderCapability::Resolve,
            ],
            trust: ProviderTrust::UserConfigured,
            version: "batch2".to_string(),
            enabled: true,
            requires_auth: false,
            allowed_hosts: Vec::new(),
        }
    }

    fn health(&self, operation: &str) -> crate::domain::ProviderHealth {
        self.health.snapshot(operation)
    }

    fn probe(&self) -> ComicFuture<'_, ProbeDto> {
        Box::pin(tracked(&self.health, "probe", async move {
            Ok(ProbeDto {
                provider_id: self.provider_id.clone(),
                reachable: true,
                authenticated: true,
                server_version: Some("local".to_string()),
                latency_ms: Some(0),
                libraries: self.libraries().await?,
            })
        }))
    }

    fn libraries(&self) -> ComicFuture<'_, Vec<LibraryDto>> {
        Box::pin(tracked(&self.health, "libraries", async move {
            Ok(vec![LibraryDto {
                id: "local-library".to_string(),
                name: self
                    .manifest_data
                    .library_name
                    .clone()
                    .unwrap_or_else(|| "Local Comics".to_string()),
                path: Some(self.root.to_string_lossy().to_string()),
                kind: Some("local".to_string()),
            }])
        }))
    }

    fn search(&self, request: SearchRequest) -> ComicFuture<'_, Vec<SeriesDto>> {
        Box::pin(tracked(&self.health, "search", async move {
            let query = request.query.to_lowercase();
            Ok(self
                .manifest_data
                .series
                .iter()
                .filter(|series| query.is_empty() || series.title.to_lowercase().contains(&query))
                .map(|series| SeriesDto {
                    id: series.id.clone(),
                    provider_id: self.provider_id.clone(),
                    library_id: Some("local-library".to_string()),
                    title: series.title.clone(),
                    sort_title: Some(series.title.clone()),
                    summary: None,
                    cover_url: None,
                    language: series.language.clone(),
                    year: None,
                })
                .collect())
        }))
    }

    fn detail(&self, series_id: String) -> ComicFuture<'_, SeriesDetailDto> {
        Box::pin(tracked(&self.health, "detail", async move {
            let series = self.series(&series_id).ok_or_else(|| {
                ComicProviderError::InvalidConfig("local series not found".to_string())
            })?;
            Ok(SeriesDetailDto {
                series: SeriesDto {
                    id: series.id.clone(),
                    provider_id: self.provider_id.clone(),
                    library_id: Some("local-library".to_string()),
                    title: series.title.clone(),
                    sort_title: Some(series.title.clone()),
                    summary: None,
                    cover_url: None,
                    language: series.language.clone(),
                    year: None,
                },
                alternate_titles: Vec::new(),
                genres: Vec::new(),
                status: None,
                total_chapters: Some(series.chapters.len() as u32),
            })
        }))
    }

    fn chapters(&self, series_id: String) -> ComicFuture<'_, Vec<ChapterDto>> {
        Box::pin(tracked(&self.health, "chapters", async move {
            let series = self.series(&series_id).ok_or_else(|| {
                ComicProviderError::InvalidConfig("local series not found".to_string())
            })?;
            Ok(series
                .chapters
                .iter()
                .enumerate()
                .map(|(index, chapter)| ChapterDto {
                    identity: ChapterIdentity {
                        provider_id: self.provider_id.clone(),
                        series_id: series.id.clone(),
                        volume_id: None,
                        chapter_id: chapter.id.clone(),
                        stable_key: format!("{}:{}:{}", self.provider_id, series.id, chapter.id),
                    },
                    title: chapter.title.clone(),
                    sort: ChapterSort {
                        volume_number: chapter.volume_number,
                        chapter_number: chapter
                            .chapter_number
                            .or_else(|| parse_number_from_title(&chapter.title)),
                        ordinal: Some(index as i64),
                        title: chapter.title.clone(),
                    },
                    language: chapter.language.clone().or_else(|| series.language.clone()),
                    language_source: if chapter.language.is_some() || series.language.is_some() {
                        LanguageSource::Manifest
                    } else {
                        LanguageSource::Unknown
                    },
                    page_count: None,
                    published_at: None,
                    file_name: Some(chapter.path.clone()),
                })
                .collect())
        }))
    }

    fn resolve(&self, request: ResolveRequest) -> ComicFuture<'_, ResolvedTarget> {
        Box::pin(tracked(&self.health, "resolve", async move {
            let series = self.series(&request.series_id).ok_or_else(|| {
                ComicProviderError::InvalidConfig("local series not found".to_string())
            })?;
            let chapter = self.chapter(series, &request.chapter_id).ok_or_else(|| {
                ComicProviderError::InvalidConfig("local chapter not found".to_string())
            })?;
            let path = self.safe_join(&chapter.path)?;
            match self.pages_for_path(&path) {
                Ok(pages) => Ok(ResolvedTarget::ImagePages {
                    pages,
                    headers: Vec::new(),
                }),
                Err(ComicProviderError::Provider(error))
                    if error.kind == ProviderErrorKind::Unsupported =>
                {
                    Ok(ResolvedTarget::Unsupported {
                        reason: error.message,
                        error_kind: error.kind,
                    })
                }
                Err(error) => Err(error),
            }
        }))
    }
}

fn hash_path(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}
