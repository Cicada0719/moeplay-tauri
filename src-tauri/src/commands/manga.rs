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
                        | Some("cn.baozimhcn.com")
                        | Some("cn.dzmanga.com")
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

    let client = crate::http_client::build_reqwest_client(
        20,
        crate::http_client::app_user_agent_with_context("manga"),
    );
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

    let parsed = url::Url::parse(&url).map_err(|e| format!("漫画源地址解析失败: {e}"))?;
    let is_baozi = matches!(
        parsed.host_str(),
        Some("cn.baozimhcn.com") | Some("cn.dzmanga.com")
    );
    let user_agent = if is_baozi {
        crate::http_client::browser_user_agent()
    } else {
        crate::http_client::app_user_agent_with_context("manga")
    };
    let client = crate::http_client::build_reqwest_client(20, user_agent);
    let attempts = if is_baozi { 3 } else { 1 };
    let mut last_error = String::new();

    for attempt in 0..attempts {
        let mut request = client.get(&url).header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        );
        if is_baozi {
            request = request.header(reqwest::header::REFERER, "https://cn.baozimhcn.com/");
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    last_error = format!("漫画源返回 HTTP {}", status.as_u16());
                } else {
                    match response.text().await {
                        Ok(text) => return Ok(text),
                        Err(error) => last_error = format!("漫画源响应读取失败: {error}"),
                    }
                }
            }
            Err(error) => last_error = format!("漫画源请求失败: {error}"),
        }
        if attempt + 1 < attempts {
            tokio::time::sleep(std::time::Duration::from_millis(300 * (attempt + 1) as u64)).await;
        }
    }

    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::is_allowed_manga_url;

    #[test]
    fn only_allows_known_manga_hosts() {
        assert!(is_allowed_manga_url("https://api.mangadex.org/manga"));
        assert!(is_allowed_manga_url(
            "https://uploads.mangadex.org/covers/id/file.jpg"
        ));
        assert!(is_allowed_manga_url(
            "https://www.dm5.com/search?title=onepiece"
        ));
        assert!(is_allowed_manga_url(
            "https://www.1kkk.com/search?title=onepiece"
        ));
        assert!(is_allowed_manga_url(
            "https://cn.baozimhcn.com/search?q=onepiece"
        ));
        assert!(is_allowed_manga_url(
            "https://cn.dzmanga.com/comic/chapter/id/0_1_1.html"
        ));
        assert!(!is_allowed_manga_url("http://api.mangadex.org/manga"));
        assert!(!is_allowed_manga_url("https://evil.example/manga"));
    }
}
