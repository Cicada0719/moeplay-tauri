// 萌游 MoeGame · 资源链抓取（M6 生态）
//
// 从 Kungal/TouchGAL 等站点抓取游戏下载/补丁/汉化等资源链接。

use serde::{Deserialize, Serialize};

/// 资源链接
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLink {
    pub label: String,
    pub url: String,
    #[serde(rename = "type")]
    pub kind: String, // magnet, baidu_pan, http_download, patch, translation_patch, official_site
    pub size: Option<String>,
    pub note: Option<String>,
}

/// 从 Kungal API 获取某游戏的详细信息和资源链接。
pub async fn fetch_kungal_resources(game_id: &str) -> Result<Vec<ResourceLink>, String> {
    let detail_url = format!(
        "https://www.kungal.com/api/galgame/detail?galgameId={}",
        game_id
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&detail_url)
        .header("User-Agent", "MoeGame/0.1")
        .send()
        .await
        .map_err(|e| format!("请求 Kungal 失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Kungal 返回 HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析 JSON 失败: {}", e))?;

    let mut links = Vec::new();

    let detail = if json.get("patch").is_some() {
        // Legacy TouchGAL 格式
        &json
    } else {
        &json
    };

    // 提取 patch 信息
    if let Some(patch) = detail.get("patch") {
        if let Some(name) = patch.get("name").and_then(|v| v.as_str()) {
            if let Some(unique_id) = patch.get("uniqueId").and_then(|v| v.as_i64()).or_else(|| {
                patch
                    .get("uniqueId")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            }) {
                links.push(ResourceLink {
                    label: format!("Patch: {}", name),
                    url: format!("https://www.touchgal.io/patch/{}", unique_id),
                    kind: "patch".into(),
                    size: None,
                    note: patch
                        .get("note")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    // 提取下载链接/磁力/网盘
    if let Some(resources) = detail.get("resources").or_else(|| detail.get("downloads")) {
        if let Some(arr) = resources.as_array() {
            for item in arr {
                let label = item
                    .get("name")
                    .or(item.get("label"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("资源下载");
                let url = item
                    .get("url")
                    .or(item.get("link"))
                    .and_then(|v| v.as_str());
                let kind = item
                    .get("type")
                    .or(item.get("kind"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("http_download");
                let size = item.get("size").and_then(|v| v.as_str());

                if let Some(url) = url {
                    links.push(ResourceLink {
                        label: label.to_string(),
                        url: url.to_string(),
                        kind: kind.to_string(),
                        size: size.map(|s| s.to_string()),
                        note: item
                            .get("note")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
        }
    }

    // 提取磁力链接字段
    if let Some(magnet) = detail.get("magnet").or(detail.get("magnetLink")) {
        if let Some(url) = magnet.as_str() {
            if !url.is_empty() {
                links.push(ResourceLink {
                    label: "磁力链接".into(),
                    url: url.to_string(),
                    kind: "magnet".into(),
                    size: None,
                    note: None,
                });
            }
        }
    }

    for field in &[
        "baiduPan",
        "baidu_pan",
        "onedrive",
        "googleDrive",
        "google_drive",
    ] {
        if let Some(link) = detail.get(field) {
            if let Some(url) = link.as_str() {
                if !url.is_empty() {
                    links.push(ResourceLink {
                        label: format!("网盘 ({})", field),
                        url: url.to_string(),
                        kind: "baidu_pan".into(),
                        size: None,
                        note: None,
                    });
                }
            }
        }
    }

    // 官方/Steam 链接
    if let Some(official) = detail.get("official").or(detail.get("officialUrl")) {
        if let Some(url) = official.as_str() {
            if !url.is_empty() {
                links.push(ResourceLink {
                    label: "官方网站".into(),
                    url: url.to_string(),
                    kind: "official_site".into(),
                    size: None,
                    note: None,
                });
            }
        }
    }

    Ok(links)
}
