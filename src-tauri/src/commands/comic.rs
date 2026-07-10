//! 哔咔漫画 Tauri 命令 — 完整功能集

use crate::comic::{self, ComicState};
use crate::secret_store::{SecretKind, SecretStore, SecretStoreError};
use serde::Serialize;
use tauri::State;

const AUTH_STATE_ERROR: &str = "漫画登录状态不可用";
const SECRET_STORE_ERROR: &str = "安全凭据存储操作失败";
const LOGIN_ERROR: &str = "登录失败，请检查账号、密码或网络连接";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicAuthStatus {
    pub configured: bool,
    pub logged_in: bool,
    pub email: Option<String>,
}

fn auth_status(configured: bool, logged_in: bool, email: Option<String>) -> ComicAuthStatus {
    ComicAuthStatus {
        configured,
        logged_in,
        email,
    }
}

fn set_memory_token(state: &ComicState, token: String) -> Result<(), String> {
    let mut guard = state
        .token
        .lock()
        .map_err(|_| AUTH_STATE_ERROR.to_string())?;
    *guard = token;
    Ok(())
}

fn require_token(state: &State<'_, ComicState>) -> Result<String, String> {
    let guard = state
        .token
        .lock()
        .map_err(|_| AUTH_STATE_ERROR.to_string())?;
    if guard.is_empty() {
        return Err("未登录，请先在漫画页面登录你的哔咔账号".to_string());
    }
    Ok(guard.clone())
}

async fn run_secret_store<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, SecretStoreError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|_| SECRET_STORE_ERROR.to_string())?
        .map_err(|_| SECRET_STORE_ERROR.to_string())
}

async fn persist_session(
    token: String,
    email: Option<String>,
    state: &ComicState,
    store: SecretStore,
) -> Result<ComicAuthStatus, String> {
    if token.trim().is_empty() {
        return Err("登录凭据无效".to_string());
    }

    let token_for_store = token.clone();
    run_secret_store(move || store.set(SecretKind::PicacgToken, None, &token_for_store)).await?;
    set_memory_token(state, token)?;
    Ok(auth_status(true, true, email))
}

async fn restore_session(
    state: &ComicState,
    store: SecretStore,
) -> Result<ComicAuthStatus, String> {
    let token = run_secret_store(move || store.get(SecretKind::PicacgToken, None)).await?;
    match token {
        Some(token) if !token.trim().is_empty() => {
            set_memory_token(state, token)?;
            Ok(auth_status(true, true, None))
        }
        _ => {
            set_memory_token(state, String::new())?;
            Ok(auth_status(false, false, None))
        }
    }
}

async fn delete_session(state: &ComicState, store: SecretStore) -> Result<ComicAuthStatus, String> {
    let delete_result = run_secret_store(move || store.delete(SecretKind::PicacgToken, None)).await;
    set_memory_token(state, String::new())?;
    delete_result?;
    Ok(auth_status(false, false, None))
}

fn extract_login_token(response: &serde_json::Value) -> Result<String, String> {
    response["data"]["token"]
        .as_str()
        .filter(|token| !token.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| LOGIN_ERROR.to_string())
}

async fn finish_login(
    response: &serde_json::Value,
    email: String,
    state: &ComicState,
    store: SecretStore,
) -> Result<ComicAuthStatus, String> {
    let token = extract_login_token(response)?;
    persist_session(token, Some(email), state, store).await
}

// ── 认证 ──────────────────────────────────────────────────────────────────

/// One-time migration entry for the legacy localStorage token.
/// The returned status deliberately contains no secret material.
#[tauri::command]
pub async fn comic_set_token(
    token: String,
    state: State<'_, ComicState>,
    store: State<'_, SecretStore>,
) -> Result<ComicAuthStatus, String> {
    persist_session(token, None, state.inner(), store.inner().clone()).await
}

#[tauri::command]
pub async fn comic_restore_session(
    state: State<'_, ComicState>,
    store: State<'_, SecretStore>,
) -> Result<ComicAuthStatus, String> {
    restore_session(state.inner(), store.inner().clone()).await
}

#[tauri::command]
pub async fn comic_logout(
    state: State<'_, ComicState>,
    store: State<'_, SecretStore>,
) -> Result<ComicAuthStatus, String> {
    delete_session(state.inner(), store.inner().clone()).await
}

#[tauri::command]
pub async fn comic_login(
    email: String,
    password: String,
    state: State<'_, ComicState>,
    store: State<'_, SecretStore>,
) -> Result<ComicAuthStatus, String> {
    let body = serde_json::json!({ "email": &email, "password": &password });
    let response = comic::api_post("auth/sign-in", "", &body)
        .await
        .map_err(|_| LOGIN_ERROR.to_string())?;
    finish_login(&response, email, state.inner(), store.inner().clone()).await
}

#[tauri::command]
pub async fn comic_profile(
    state: State<'_, ComicState>,
) -> Result<comic::ComicUserProfile, String> {
    let token = require_token(&state)?;
    let response = comic::api_get("users/profile", &token, &[]).await?;
    comic::ComicUserProfile::from_value(&response["data"]["user"])
        .ok_or_else(|| "无法解析用户资料".to_string())
}

// ── 分类 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_categories(
    state: State<'_, ComicState>,
) -> Result<Vec<comic::ComicCategory>, String> {
    let token = require_token(&state)?;
    let resp = comic::api_get("categories", &token, &[]).await?;
    let cats = resp["data"]["categories"]
        .as_array()
        .ok_or("invalid categories response")?
        .iter()
        .filter_map(comic::ComicCategory::from_value)
        .collect();
    Ok(cats)
}

// ── 漫画列表 ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_list(
    page: u32,
    category: Option<String>,
    tag: Option<String>,
    sort: Option<String>,
    state: State<'_, ComicState>,
) -> Result<comic::ComicListPage, String> {
    let token = require_token(&state)?;
    let page_s = page.to_string();
    let sort_s = sort.unwrap_or_else(|| "dd".into());

    let mut params: Vec<(&str, String)> = vec![("page", page_s), ("s", sort_s)];
    if let Some(c) = &category {
        params.push(("c", c.clone()));
    }
    if let Some(t) = &tag {
        params.push(("t", t.clone()));
    }

    let resp = comic::api_get("comics", &token, &params).await?;
    Ok(comic::ComicListPage::from_value(&resp["data"]["comics"]))
}

// ── 搜索 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_search(
    keyword: String,
    page: u32,
    sort: Option<String>,
    categories: Option<Vec<String>>,
    state: State<'_, ComicState>,
) -> Result<comic::ComicListPage, String> {
    let token = require_token(&state)?;
    let path = format!("comics/advanced-search?page={}", page);
    let mut body = serde_json::json!({ "keyword": keyword });
    if let Some(s) = sort {
        body["sort"] = serde_json::Value::String(s);
    }
    if let Some(cats) = categories {
        body["categories"] = serde_json::json!(cats);
    }
    let resp = comic::api_post(&path, &token, &body).await?;
    Ok(comic::ComicListPage::from_value(&resp["data"]["comics"]))
}

// ── 详情 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_detail(
    id: String,
    state: State<'_, ComicState>,
) -> Result<comic::ComicDetail, String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}", id);
    let resp = comic::api_get(&path, &token, &[]).await?;
    comic::ComicDetail::from_value(&resp["data"]["comic"])
        .ok_or_else(|| "无法解析漫画详情".to_string())
}

// ── 章节 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_chapters(
    id: String,
    state: State<'_, ComicState>,
) -> Result<Vec<comic::ComicChapter>, String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}/eps", id);

    let resp = comic::api_get(&path, &token, &[("page", "1".into())]).await?;
    let eps = &resp["data"]["eps"];
    let total_pages = eps["pages"].as_i64().unwrap_or(1);

    let mut chapters: Vec<comic::ComicChapter> = eps["docs"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(comic::ComicChapter::from_value)
        .collect();

    for p in 2..=total_pages {
        let ps = p.to_string();
        if let Ok(r) = comic::api_get(&path, &token, &[("page", ps)]).await {
            let more: Vec<comic::ComicChapter> = r["data"]["eps"]["docs"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(comic::ComicChapter::from_value)
                .collect();
            chapters.extend(more);
        }
    }

    chapters.sort_by_key(|c| c.order);
    Ok(chapters)
}

// ── 章节图片 ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_chapter_images(
    id: String,
    order: u32,
    state: State<'_, ComicState>,
) -> Result<Vec<comic::ComicImage>, String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}/order/{}/pages", id, order);

    let resp = comic::api_get(&path, &token, &[("page", "1".into())]).await?;
    let pages_data = &resp["data"]["pages"];
    let total_pages = pages_data["pages"].as_i64().unwrap_or(1);

    let mut images: Vec<comic::ComicImage> = pages_data["docs"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(comic::ComicImage::from_value)
        .collect();

    for p in 2..=total_pages {
        let ps = p.to_string();
        if let Ok(r) = comic::api_get(&path, &token, &[("page", ps)]).await {
            let more: Vec<comic::ComicImage> = r["data"]["pages"]["docs"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(comic::ComicImage::from_value)
                .collect();
            images.extend(more);
        }
    }

    Ok(images)
}

// ── 排行榜 ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_ranking(
    time_type: String,
    state: State<'_, ComicState>,
) -> Result<Vec<comic::ComicSummary>, String> {
    let token = require_token(&state)?;
    let resp = comic::api_get(
        "comics/leaderboard",
        &token,
        &[("tt", time_type), ("ct", "VC".into())],
    )
    .await?;
    let list = resp["data"]["comics"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(comic::ComicSummary::from_value)
        .collect();
    Ok(list)
}

// ── 随机本子 ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_random(
    state: State<'_, ComicState>,
) -> Result<Vec<comic::ComicSummary>, String> {
    let token = require_token(&state)?;
    let resp = comic::api_get("comics/random", &token, &[]).await?;
    let list = resp["data"]["comics"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(comic::ComicSummary::from_value)
        .collect();
    Ok(list)
}

// ── 收藏 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_favorites(
    page: u32,
    sort: Option<String>,
    state: State<'_, ComicState>,
) -> Result<comic::ComicListPage, String> {
    let token = require_token(&state)?;
    let mut params = vec![("page", page.to_string())];
    if let Some(s) = sort {
        params.push(("s", s));
    }
    let resp = comic::api_get("users/favourite", &token, &params).await?;
    Ok(comic::ComicListPage::from_value(&resp["data"]["comics"]))
}

#[tauri::command]
pub async fn comic_toggle_favourite(
    id: String,
    state: State<'_, ComicState>,
) -> Result<String, String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}/favourite", id);
    let resp = comic::api_post(&path, &token, &serde_json::json!({})).await?;
    Ok(resp["data"]["action"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}

// ── 点赞 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_like(id: String, state: State<'_, ComicState>) -> Result<String, String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}/like", id);
    let resp = comic::api_post(&path, &token, &serde_json::json!({})).await?;
    Ok(resp["data"]["action"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}

// ── 评论 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_comments(
    id: String,
    page: u32,
    state: State<'_, ComicState>,
) -> Result<comic::CommentsPage, String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}/comments", id);
    let resp = comic::api_get(&path, &token, &[("page", page.to_string())]).await?;
    Ok(comic::CommentsPage::from_value(&resp["data"]["comments"]))
}

#[tauri::command]
pub async fn comic_post_comment(
    id: String,
    content: String,
    state: State<'_, ComicState>,
) -> Result<(), String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}/comments", id);
    comic::api_post(&path, &token, &serde_json::json!({ "content": content })).await?;
    Ok(())
}

#[tauri::command]
pub async fn comic_comment_like(
    comment_id: String,
    state: State<'_, ComicState>,
) -> Result<String, String> {
    let token = require_token(&state)?;
    let path = format!("comments/{}/like", comment_id);
    let resp = comic::api_post(&path, &token, &serde_json::json!({})).await?;
    Ok(resp["data"]["action"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}

/// 评论的子评论/回复
#[tauri::command]
pub async fn comic_comment_children(
    comment_id: String,
    page: u32,
    state: State<'_, ComicState>,
) -> Result<comic::CommentsPage, String> {
    let token = require_token(&state)?;
    let path = format!("comments/{}/childrens", comment_id);
    let resp = comic::api_get(&path, &token, &[("page", page.to_string())]).await?;
    Ok(comic::CommentsPage::from_value(&resp["data"]["comments"]))
}

// ── 推荐 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_recommendation(
    id: String,
    state: State<'_, ComicState>,
) -> Result<Vec<comic::ComicSummary>, String> {
    let token = require_token(&state)?;
    let path = format!("comics/{}/recommendation", id);
    let resp = comic::api_get(&path, &token, &[]).await?;
    let list = resp["data"]["comics"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(comic::ComicSummary::from_value)
        .collect();
    Ok(list)
}

// ── 打卡 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_punch_in(state: State<'_, ComicState>) -> Result<serde_json::Value, String> {
    let token = require_token(&state)?;
    let resp = comic::api_post("users/punch-in", &token, &serde_json::json!({})).await?;
    Ok(resp["data"].clone())
}

// ── 骑士榜 ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_knight_leaderboard(
    state: State<'_, ComicState>,
) -> Result<Vec<comic::ComicCommentUser>, String> {
    let token = require_token(&state)?;
    let resp = comic::api_get("comics/knight-leaderboard", &token, &[]).await?;
    let list = resp["data"]["users"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(comic::ComicCommentUser::from_value)
        .collect();
    Ok(list)
}

// ── 我的评论 ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_my_comments(
    page: u32,
    state: State<'_, ComicState>,
) -> Result<comic::CommentsPage, String> {
    let token = require_token(&state)?;
    let resp = comic::api_get("users/my-comments", &token, &[("page", page.to_string())]).await?;
    Ok(comic::CommentsPage::from_value(&resp["data"]["comments"]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::{BackendError, SecretBackend};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MemoryBackend {
        values: Mutex<HashMap<(String, String), String>>,
    }

    impl SecretBackend for MemoryBackend {
        fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), BackendError> {
            self.values
                .lock()
                .expect("memory backend lock")
                .insert((service.to_owned(), account.to_owned()), secret.to_owned());
            Ok(())
        }

        fn get(&self, service: &str, account: &str) -> Result<String, BackendError> {
            self.values
                .lock()
                .expect("memory backend lock")
                .get(&(service.to_owned(), account.to_owned()))
                .cloned()
                .ok_or(BackendError::Missing)
        }

        fn delete(&self, service: &str, account: &str) -> Result<(), BackendError> {
            match self
                .values
                .lock()
                .expect("memory backend lock")
                .remove(&(service.to_owned(), account.to_owned()))
            {
                Some(_) => Ok(()),
                None => Err(BackendError::Missing),
            }
        }
    }

    fn memory_store() -> SecretStore {
        SecretStore::with_backend(Arc::new(MemoryBackend::default()))
    }

    fn memory_token(state: &ComicState) -> String {
        state.token.lock().expect("comic state lock").clone()
    }

    #[tokio::test]
    async fn login_result_does_not_leak_token() {
        let state = ComicState::default();
        let store = memory_store();
        let secret = "picacg-login-secret";
        let response = serde_json::json!({ "data": { "token": secret } });

        let status = finish_login(
            &response,
            "reader@example.com".to_string(),
            &state,
            store.clone(),
        )
        .await
        .expect("finish login");

        assert!(status.configured);
        assert!(status.logged_in);
        assert_eq!(status.email.as_deref(), Some("reader@example.com"));
        assert_eq!(memory_token(&state), secret);
        assert_eq!(
            store
                .get(SecretKind::PicacgToken, None)
                .expect("stored login token")
                .as_deref(),
            Some(secret)
        );

        let wire = serde_json::to_string(&status).expect("serialize login status");
        assert!(!wire.contains(secret));
        assert!(!wire.contains("token"));
    }

    #[tokio::test]
    async fn restore_loads_secret_store_token_into_memory() {
        let state = ComicState::default();
        let store = memory_store();
        store
            .set(SecretKind::PicacgToken, None, "restored-secret")
            .expect("seed secret store");

        let status = restore_session(&state, store)
            .await
            .expect("restore session");

        assert_eq!(status, auth_status(true, true, None));
        assert_eq!(memory_token(&state), "restored-secret");
    }

    #[tokio::test]
    async fn logout_deletes_secret_store_and_clears_memory() {
        let state = ComicState::default();
        let store = memory_store();
        persist_session("logout-secret".to_string(), None, &state, store.clone())
            .await
            .expect("seed session");

        let status = delete_session(&state, store.clone())
            .await
            .expect("delete session");

        assert_eq!(status, auth_status(false, false, None));
        assert_eq!(memory_token(&state), "");
        assert_eq!(
            store
                .get(SecretKind::PicacgToken, None)
                .expect("read deleted secret"),
            None
        );
    }

    #[tokio::test]
    async fn legacy_migration_persists_token_without_returning_it() {
        let state = ComicState::default();
        let store = memory_store();
        let legacy_secret = "legacy-local-storage-secret";

        let status = persist_session(legacy_secret.to_string(), None, &state, store.clone())
            .await
            .expect("migrate legacy token");

        assert_eq!(status, auth_status(true, true, None));
        assert_eq!(memory_token(&state), legacy_secret);
        assert_eq!(
            store
                .get(SecretKind::PicacgToken, None)
                .expect("read migrated token")
                .as_deref(),
            Some(legacy_secret)
        );
        let wire = serde_json::to_string(&status).expect("serialize migration status");
        assert!(!wire.contains(legacy_secret));
        assert!(!wire.contains("token"));
    }
}
