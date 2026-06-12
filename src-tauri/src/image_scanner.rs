//! 文件夹图片扫描

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCandidate {
    pub path: String,
    pub kind: String,
    pub score: i32,
    pub size: u64,
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "gif"];

pub fn scan_images(dir: &Path) -> Result<Vec<ImageCandidate>, String> {
    if !dir.is_dir() {
        return Err("目录不存在".to_string());
    }

    let mut images = Vec::new();
    scan_recursive(dir, &mut images, 0)?;
    images.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| b.size.cmp(&a.size)));
    images.truncate(64);
    Ok(images)
}

fn scan_recursive(dir: &Path, images: &mut Vec<ImageCandidate>, depth: u32) -> Result<(), String> {
    if depth > 3 {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            scan_recursive(&path, images, depth + 1)?;
            continue;
        }
        if !is_image(&path) {
            continue;
        }

        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let kind = classify(&path);
        let score = score_image(&path, &kind, size, depth);
        images.push(ImageCandidate {
            path: path.to_string_lossy().to_string(),
            kind,
            score,
            size,
        });
    }

    Ok(())
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn classify(path: &Path) -> String {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ["cover", "package", "poster", "jacket", "封面"]
        .iter()
        .any(|k| name.contains(k))
    {
        "cover".to_string()
    } else if ["background", "wallpaper", "bg", "背景"]
        .iter()
        .any(|k| name.contains(k))
    {
        "background".to_string()
    } else if ["icon", "logo"].iter().any(|k| name.contains(k)) {
        "icon".to_string()
    } else {
        "screenshot".to_string()
    }
}

fn score_image(path: &Path, kind: &str, size: u64, depth: u32) -> i32 {
    let mut score = match kind {
        "cover" => 80,
        "background" => 70,
        "icon" => 55,
        _ => 40,
    };
    if size > 100 * 1024 {
        score += 10;
    }
    if size > 1024 * 1024 {
        score += 5;
    }
    score -= (depth as i32) * 5;
    if path.to_string_lossy().to_lowercase().contains("thumb") {
        score -= 20;
    }
    score
}
