//! 哔咔漫画 Tauri 命令 — 完整功能集

use crate::comic::{self, ComicState};
use tauri::State;

fn require_token(state: &State<'_, ComicState>) -> Result<String, String> {
    let guard = state.token.lock().map_err(|e| e.to_string())?;
    if guard.is_empty() {
        return Err("未登录，请先在漫画页面登录你的哔咔账号".to_string());
    }
    Ok(guard.clone())
}

// ── 认证 ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn comic_set_token(
    token: String,
    state: State<'_, ComicState>,
) -> Result<(), String> {
    let mut guard = state.token.lock().map_err(|e| e.to_string())?;
    *guard = token;
    Ok(())
}

#[tauri::command]
pub async fn comic_login(
    email: String,
    password: String,
    state: State<'_, ComicState>,
) -> Result<String, String> {
    let body = serde_json::json!({ "email": email, "password": password });
    let resp = comic::api_post("auth/sign-in", "", &body).await?;
    let token = resp["data"]["token"]
        .as_str()
        .ok_or_else(|| "登录响应中未找到 token".to_string())?
        .to_string();
    {
        let mut guard = state.token.lock().map_err(|e| e.to_string())?;
        *guard = token.clone();
    }
    Ok(token)
}

#[tauri::command]
pub async fn comic_profile(
    state: State<'_, ComicState>,
) -> Result<comic::ComicUserProfile, String> {
    let token = require_token(&state)?;
    let resp = comic::api_get("users/profile", &token, &[]).await?;
    comic::ComicUserProfile::from_value(&resp["data"]["user"])
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

    let mut params: Vec<(&str, String)> = vec![
        ("page", page_s),
        ("s", sort_s),
    ];
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
pub async fn comic_like(
    id: String,
    state: State<'_, ComicState>,
) -> Result<String, String> {
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
pub async fn comic_punch_in(
    state: State<'_, ComicState>,
) -> Result<serde_json::Value, String> {
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
