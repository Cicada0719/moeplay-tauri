use serde_json::Value;

fn is_allowed_manga_url(url: &str) -> bool {
    match url::Url::parse(url) {
        Ok(parsed) => {
            parsed.scheme() == "https"
                && matches!(
                    parsed.host_str(),
                    Some("api.mangadex.org")
                        | Some("uploads.mangadex.org")
                        | Some("www.dm5.com")
                        | Some("www.1kkk.com")
                )
        }
        Err(_) => false,
    }
}

#[tauri::command]
pub async fn manga_fetch_json(url: String) -> Result<Value, String> {
    if !is_allowed_manga_url(&url) {
        return Err("不允许访问该漫画源地址".into());
    }

    let client = crate::http_client::build_reqwest_client(20, "MoePlay/0.11.9 manga");
    let response = client
        .get(&url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| format!("漫画源请求失败: {e}"))?;
    let status = response.status();

    if !status.is_success() {
        return Err(format!("漫画源返回 HTTP {}", status.as_u16()));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| format!("漫画源响应解析失败: {e}"))
}

#[tauri::command]
pub async fn manga_fetch_text(url: String) -> Result<String, String> {
    if !is_allowed_manga_url(&url) {
        return Err("不允许访问该漫画源地址".into());
    }

    let client = crate::http_client::build_reqwest_client(20, "MoePlay/0.11.9 manga");
    let response = client
        .get(&url)
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .map_err(|e| format!("漫画源请求失败: {e}"))?;
    let status = response.status();

    if !status.is_success() {
        return Err(format!("漫画源返回 HTTP {}", status.as_u16()));
    }

    response
        .text()
        .await
        .map_err(|e| format!("漫画源响应读取失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::is_allowed_manga_url;

    #[test]
    fn only_allows_known_manga_hosts() {
        assert!(is_allowed_manga_url("https://api.mangadex.org/manga"));
        assert!(is_allowed_manga_url("https://uploads.mangadex.org/covers/id/file.jpg"));
        assert!(is_allowed_manga_url("https://www.dm5.com/search?title=onepiece"));
        assert!(is_allowed_manga_url("https://www.1kkk.com/search?title=onepiece"));
        assert!(!is_allowed_manga_url("http://api.mangadex.org/manga"));
        assert!(!is_allowed_manga_url("https://evil.example/manga"));
    }
}
