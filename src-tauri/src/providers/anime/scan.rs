use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{provider_error, AnimeLocalMediaEpisode, AnimeLocalMediaSeries, ProviderResult};
use crate::domain::ProviderErrorKind;

const MAX_SCAN_DEPTH: usize = 10;
const MAX_MEDIA_FILES: usize = 5_000;
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "webm", "avi", "mov", "m4v", "ts", "m2ts", "flv", "wmv",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimeLocalMediaScanResult {
    pub directory: String,
    pub allowed_paths: Vec<String>,
    pub library: Vec<AnimeLocalMediaSeries>,
    pub series_count: usize,
    pub file_count: usize,
    pub skipped_count: usize,
    pub warnings: Vec<String>,
}

pub fn scan_local_media_directory(directory: &Path) -> ProviderResult<AnimeLocalMediaScanResult> {
    let root =
        fs::canonicalize(directory).map_err(|_| scan_error("selected directory is unavailable"))?;
    if !root.is_dir() {
        return Err(scan_error("selected path is not a directory"));
    }

    let mut media_by_directory: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut skipped_count = 0usize;
    let mut warnings = Vec::new();
    collect_media_files(
        &root,
        &root,
        0,
        &mut media_by_directory,
        &mut skipped_count,
        &mut warnings,
    )?;

    let mut library = Vec::with_capacity(media_by_directory.len());
    let mut file_count = 0usize;
    for (series_directory, mut files) in media_by_directory {
        files.sort_by_key(|left| natural_episode_key(left));
        let relative_directory = series_directory
            .strip_prefix(&root)
            .unwrap_or(Path::new(""));
        let title = if relative_directory.as_os_str().is_empty() {
            root.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("本地番剧")
                .to_string()
        } else {
            relative_directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("本地番剧")
                .to_string()
        };
        let series_key = if relative_directory.as_os_str().is_empty() {
            ".".to_string()
        } else {
            relative_directory.to_string_lossy().replace('\\', "/")
        };
        let episodes = files
            .into_iter()
            .map(|path| {
                let relative = path.strip_prefix(&root).unwrap_or(path.as_path());
                let stem = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Episode");
                AnimeLocalMediaEpisode {
                    id: format!("episode-{:016x}", stable_hash(&relative.to_string_lossy())),
                    title: display_episode_title(stem),
                    number: parse_episode_number(stem),
                    path: path.to_string_lossy().into_owned(),
                }
            })
            .collect::<Vec<_>>();
        file_count += episodes.len();
        library.push(AnimeLocalMediaSeries {
            id: format!("series-{:016x}", stable_hash(&series_key)),
            title,
            original_title: None,
            synopsis: Some("从用户选择的本地目录扫描".to_string()),
            artwork_url: None,
            genres: vec!["本地媒体".to_string()],
            episodes,
        });
    }

    if file_count == 0 {
        warnings.push("未找到支持的视频文件".to_string());
    }

    Ok(AnimeLocalMediaScanResult {
        directory: root.to_string_lossy().into_owned(),
        allowed_paths: vec![root.to_string_lossy().into_owned()],
        series_count: library.len(),
        file_count,
        skipped_count,
        library,
        warnings,
    })
}

fn collect_media_files(
    root: &Path,
    directory: &Path,
    depth: usize,
    media_by_directory: &mut BTreeMap<PathBuf, Vec<PathBuf>>,
    skipped_count: &mut usize,
    warnings: &mut Vec<String>,
) -> ProviderResult<()> {
    if depth > MAX_SCAN_DEPTH {
        *skipped_count += 1;
        if !warnings.iter().any(|warning| warning.contains("目录层级")) {
            warnings.push(format!("已跳过超过 {MAX_SCAN_DEPTH} 层的目录"));
        }
        return Ok(());
    }

    let entries =
        fs::read_dir(directory).map_err(|_| scan_error("selected directory could not be read"))?;
    for entry in entries {
        if current_file_count(media_by_directory) >= MAX_MEDIA_FILES {
            if !warnings.iter().any(|warning| warning.contains("文件上限")) {
                warnings.push(format!("扫描达到 {MAX_MEDIA_FILES} 个视频文件上限"));
            }
            return Ok(());
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                *skipped_count += 1;
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => {
                *skipped_count += 1;
                continue;
            }
        };
        if file_type.is_symlink() {
            *skipped_count += 1;
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_media_files(
                root,
                &path,
                depth + 1,
                media_by_directory,
                skipped_count,
                warnings,
            )?;
        } else if file_type.is_file() && is_video_file(&path) {
            let canonical = match fs::canonicalize(&path) {
                Ok(path) if path.starts_with(root) => path,
                _ => {
                    *skipped_count += 1;
                    continue;
                }
            };
            media_by_directory
                .entry(canonical.parent().unwrap_or(root).to_path_buf())
                .or_default()
                .push(canonical);
        }
    }
    Ok(())
}

fn current_file_count(media_by_directory: &BTreeMap<PathBuf, Vec<PathBuf>>) -> usize {
    media_by_directory.values().map(Vec::len).sum()
}

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            VIDEO_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

fn natural_episode_key(path: &Path) -> (u32, String) {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    (
        parse_episode_number(stem).unwrap_or(u32::MAX),
        stem.to_lowercase(),
    )
}

fn parse_episode_number(stem: &str) -> Option<u32> {
    let lower = stem.to_ascii_lowercase();
    for marker in ["episode", "ep", "e"] {
        let mut offset = 0usize;
        while let Some(position) = lower[offset..].find(marker) {
            let start = offset + position + marker.len();
            let digits = lower[start..]
                .chars()
                .skip_while(|character| !character.is_ascii_digit())
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>();
            if let Ok(number) = digits.parse::<u32>() {
                return Some(number);
            }
            offset = start;
            if offset >= lower.len() {
                break;
            }
        }
    }

    lower
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok())
        .next_back()
}

fn display_episode_title(stem: &str) -> String {
    let cleaned = stem.replace(['_', '.'], " ").replace("  ", " ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Episode".to_string()
    } else {
        trimmed.to_string()
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn scan_error(message: &str) -> crate::domain::ProviderError {
    provider_error(
        "local_media",
        "scan",
        ProviderErrorKind::PolicyBlocked,
        message,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_groups_video_files_by_directory_and_orders_episodes() {
        let root =
            std::env::temp_dir().join(format!("moeplay-anime-scan-{}", uuid::Uuid::new_v4()));
        let season = root.join("Fixture Show");
        fs::create_dir_all(&season).unwrap();
        fs::write(season.join("Fixture.Show.E02.mkv"), b"fixture").unwrap();
        fs::write(season.join("Fixture.Show.E01.mp4"), b"fixture").unwrap();
        fs::write(season.join("notes.txt"), b"ignore").unwrap();

        let result = scan_local_media_directory(&root).unwrap();
        assert_eq!(result.series_count, 1);
        assert_eq!(result.file_count, 2);
        assert_eq!(result.library[0].title, "Fixture Show");
        assert_eq!(result.library[0].episodes[0].number, Some(1));
        assert_eq!(result.library[0].episodes[1].number, Some(2));
        assert!(result.library[0]
            .episodes
            .iter()
            .all(|episode| Path::new(&episode.path).is_absolute()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scan_returns_a_friendly_empty_result() {
        let root =
            std::env::temp_dir().join(format!("moeplay-anime-scan-empty-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let result = scan_local_media_directory(&root).unwrap();
        assert_eq!(result.file_count, 0);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("未找到")));
        fs::remove_dir_all(root).unwrap();
    }
}
