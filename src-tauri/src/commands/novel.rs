use ::scraper::{Html, Selector};
use futures_util::{stream, StreamExt};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const GUTENDEX_API: &str = "https://gutendex.com";
const BIQUGE_BASE: &str = "https://www.biquge.com.tw";
const X80_BASE: &str = "https://www.80xsw.com";
const WIKISOURCE_API: &str = "https://zh.wikisource.org/w/api.php";
const INTERNET_ARCHIVE_API: &str = "https://archive.org/advancedsearch.php";
const INTERNET_ARCHIVE_METADATA: &str = "https://archive.org/metadata";
const INTERNET_ARCHIVE_DOWNLOAD: &str = "https://archive.org/download";
const OPEN_LIBRARY_BASE: &str = "https://openlibrary.org";
const STANDARD_EBOOKS_BASE: &str = "https://standardebooks.org";
const MAX_CHAPTERS: usize = 2_000;
const MAX_CATALOG_PAGES: usize = 30;
const MAX_CHAPTER_PAGES: usize = 24;
const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelBook {
    pub id: String,
    pub source: String,
    pub title: String,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub cover_url: Option<String>,
    pub language: Option<String>,
    pub subjects: Vec<String>,
    pub public_domain: bool,
    pub source_url: String,
    pub download_url: Option<String>,
    pub download_format: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelChapter {
    pub id: String,
    pub title: String,
    pub order: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelDetail {
    pub book: NovelBook,
    pub chapters: Vec<NovelChapter>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NovelChapterContent {
    pub book_id: String,
    pub source: String,
    pub chapter: NovelChapter,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct GutendexList {
    #[serde(default)]
    results: Vec<GutendexBook>,
}

#[derive(Debug, Clone, Deserialize)]
struct GutendexPerson {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GutendexBook {
    id: u64,
    title: String,
    #[serde(default)]
    authors: Vec<GutendexPerson>,
    #[serde(default)]
    summaries: Vec<String>,
    #[serde(default)]
    subjects: Vec<String>,
    #[serde(default)]
    bookshelves: Vec<String>,
    #[serde(default)]
    languages: Vec<String>,
    copyright: Option<bool>,
    #[serde(default)]
    formats: HashMap<String, String>,
}

#[tauri::command]
pub async fn novel_search(source: String, query: String) -> Result<Vec<NovelBook>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("请输入小说名或作者".to_string());
    }
    if query.chars().count() > 120 {
        return Err("搜索关键词过长".to_string());
    }

    match source.trim().to_ascii_lowercase().as_str() {
        "biquge" => search_biquge(query).await,
        "x80" => search_x80(query).await,
        "internetarchive" => search_internet_archive(query).await,
        "openlibrary" => search_open_library(query).await,
        "standardebooks" => search_standard_ebooks(query).await,
        "gutenberg" => search_gutenberg(query).await,
        "wikisource" => search_wikisource(query).await,
        "all" | "" => {
            let (biquge, x80, archive, open_library, standard_ebooks, gutenberg, wikisource) = tokio::join!(
                search_biquge(query),
                search_x80(query),
                search_internet_archive(query),
                search_open_library(query),
                search_standard_ebooks(query),
                search_gutenberg(query),
                search_wikisource(query)
            );
            let mut books = Vec::new();
            let mut errors = Vec::new();
            for result in [
                biquge,
                x80,
                archive,
                open_library,
                standard_ebooks,
                gutenberg,
                wikisource,
            ] {
                match result {
                    Ok(mut items) => books.append(&mut items),
                    Err(error) => errors.push(error),
                }
            }
            if books.is_empty() && !errors.is_empty() {
                Err(errors.join("；"))
            } else {
                Ok(books)
            }
        }
        _ => Err("不支持的小说源".to_string()),
    }
}

#[tauri::command]
pub async fn novel_detail(source: String, book_id: String) -> Result<NovelDetail, String> {
    match source.trim().to_ascii_lowercase().as_str() {
        "biquge" => detail_biquge(&book_id).await,
        "x80" => detail_x80(&book_id).await,
        "internetarchive" => detail_internet_archive(&book_id).await,
        "openlibrary" => detail_open_library(&book_id).await,
        "standardebooks" => detail_standard_ebooks(&book_id).await,
        "gutenberg" => detail_gutenberg(&book_id).await,
        "wikisource" => detail_wikisource(&book_id).await,
        _ => Err("不支持的小说源".to_string()),
    }
}

#[tauri::command]
pub async fn novel_read_chapter(
    source: String,
    book_id: String,
    chapter_id: String,
) -> Result<NovelChapterContent, String> {
    match source.trim().to_ascii_lowercase().as_str() {
        "biquge" => read_biquge(&book_id, &chapter_id).await,
        "x80" => read_x80(&book_id, &chapter_id).await,
        "internetarchive" => read_internet_archive(&book_id, &chapter_id).await,
        "openlibrary" => read_open_library(&book_id, &chapter_id).await,
        "standardebooks" => {
            Err("Standard Ebooks 提供公开 EPUB 下载，不提供应用内纯文本正文".to_string())
        }
        "gutenberg" => read_gutenberg(&book_id, &chapter_id).await,
        "wikisource" => read_wikisource(&book_id, &chapter_id).await,
        _ => Err("不支持的小说源".to_string()),
    }
}

async fn search_biquge(query: &str) -> Result<Vec<NovelBook>, String> {
    let html = fetch_html_response(
        novel_client(30)
            .post(format!("{BIQUGE_BASE}/search.html"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Referer", format!("{BIQUGE_BASE}/"))
            .body(format!("s={}", urlencoding::encode(query))),
        "笔趣阁搜索",
    )
    .await?;
    let books = parse_biquge_search(&html);
    if books.is_empty() {
        tracing::warn!(query, "Biquge search returned no parseable results");
    }
    Ok(books)
}

async fn detail_biquge(book_id: &str) -> Result<NovelDetail, String> {
    let book_id = validate_biquge_book_id(book_id)?;
    let source_url = format!("{BIQUGE_BASE}/{book_id}/");
    let info_html = fetch_html(&source_url, 30, "笔趣阁作品详情").await?;
    let book = parse_biquge_book_detail(&book_id, &info_html)?;

    let first_catalog_url = format!("{BIQUGE_BASE}/{book_id}/ml0.html");
    let first_catalog = fetch_html(&first_catalog_url, 30, "笔趣阁章节目录")
        .await
        .unwrap_or_else(|_| info_html.clone());
    let mut catalog_pages = parse_biquge_catalog_pages(&book_id, &first_catalog);
    if catalog_pages.is_empty() {
        catalog_pages.push((0, format!("/{book_id}/ml0.html")));
    }
    catalog_pages.sort_by_key(|(page, _)| *page);
    catalog_pages.dedup_by(|left, right| left.1 == right.1);
    catalog_pages.truncate(MAX_CATALOG_PAGES);

    let fetched_pages = stream::iter(catalog_pages.into_iter().map(|(page, path)| async move {
        let url = absolute_biquge_url(&path);
        (page, fetch_html(&url, 30, "笔趣阁章节目录").await)
    }))
    .buffer_unordered(4)
    .collect::<Vec<_>>()
    .await;

    let mut fetched_pages = fetched_pages;
    fetched_pages.sort_by_key(|(page, _)| *page);
    let mut seen = HashSet::new();
    let mut chapters = Vec::new();
    for (_, page) in fetched_pages {
        let Ok(page) = page else { continue };
        for (id, title) in parse_biquge_chapters(&book_id, &page) {
            if seen.insert(id.clone()) {
                chapters.push(NovelChapter {
                    id,
                    title,
                    order: chapters.len() + 1,
                });
                if chapters.len() >= MAX_CHAPTERS {
                    break;
                }
            }
        }
        if chapters.len() >= MAX_CHAPTERS {
            break;
        }
    }
    if chapters.is_empty() {
        for (id, title) in parse_biquge_chapters(&book_id, &info_html) {
            if seen.insert(id.clone()) {
                chapters.push(NovelChapter {
                    id,
                    title,
                    order: chapters.len() + 1,
                });
            }
        }
    }
    if chapters.is_empty() {
        return Err("笔趣阁未返回可读取的章节目录".to_string());
    }

    Ok(NovelDetail { book, chapters })
}

async fn read_biquge(book_id: &str, chapter_id: &str) -> Result<NovelChapterContent, String> {
    let book_id = validate_biquge_book_id(book_id)?;
    let chapter_id = validate_biquge_chapter_id(chapter_id)?;
    let chapter_stem = chapter_id.trim_end_matches(".html").to_string();
    let mut next_path = format!("/{book_id}/{chapter_id}");
    let mut visited = HashSet::new();
    let mut parts = Vec::new();
    let mut chapter_title = String::new();

    for _ in 0..MAX_CHAPTER_PAGES {
        if !visited.insert(next_path.clone()) {
            break;
        }
        let html = fetch_html(&absolute_biquge_url(&next_path), 30, "笔趣阁章节正文").await?;
        if chapter_title.is_empty() {
            chapter_title = parse_biquge_chapter_title(&html)
                .unwrap_or_else(|| chapter_id.trim_end_matches(".html").to_string());
        }
        let content = parse_biquge_chapter_content(&html);
        if !content.is_empty() {
            parts.push(content);
        }
        let Some(candidate) = parse_biquge_next_page(&html) else {
            break;
        };
        let expected_prefix = format!("/{book_id}/{chapter_stem}_");
        if !candidate.starts_with(&expected_prefix) || !candidate.ends_with(".html") {
            break;
        }
        next_path = candidate;
    }

    let content = normalize_text(&parts.join("\n\n"));
    if content.is_empty() {
        return Err("笔趣阁章节正文为空".to_string());
    }

    Ok(NovelChapterContent {
        book_id: book_id.clone(),
        source: "biquge".to_string(),
        chapter: NovelChapter {
            id: chapter_id.clone(),
            title: chapter_title,
            order: 1,
        },
        content,
    })
}

async fn fetch_html(url: &str, timeout_secs: u64, label: &str) -> Result<String, String> {
    fetch_html_response(novel_client(timeout_secs).get(url), label).await
}

async fn fetch_html_response(
    request: reqwest::RequestBuilder,
    label: &str,
) -> Result<String, String> {
    let bytes = request
        .send()
        .await
        .map_err(|error| format!("{label}失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("{label}失败: {error}"))?
        .bytes()
        .await
        .map_err(|error| format!("{label}读取失败: {error}"))?;
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(format!("{label}响应超过 8 MiB 安全上限"));
    }
    Ok(decode_text(&bytes))
}

fn parse_biquge_search(html: &str) -> Vec<NovelBook> {
    let document = Html::parse_document(html);
    let item_selector =
        Selector::parse(".sort_list li").expect("valid Biquge search item selector");
    let link_selector = Selector::parse(".s2 a").expect("valid Biquge search link selector");
    let author_selector = Selector::parse(".s5").expect("valid Biquge author selector");
    document
        .select(&item_selector)
        .filter_map(|item| {
            let link = item.select(&link_selector).next()?;
            let href = link.value().attr("href")?;
            let id = biquge_book_id_from_href(href)?;
            let title = normalize_text(&link.text().collect::<Vec<_>>().join(" "));
            if title.is_empty() {
                return None;
            }
            let author = item
                .select(&author_selector)
                .next()
                .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
                .filter(|value| !value.is_empty());
            Some(NovelBook {
                id: id.clone(),
                source: "biquge".to_string(),
                title,
                author,
                summary: None,
                cover_url: None,
                language: Some("zh".to_string()),
                subjects: vec!["中文网文".to_string()],
                public_domain: false,
                source_url: format!("{BIQUGE_BASE}/{id}/"),
                download_url: None,
                download_format: None,
            })
        })
        .take(24)
        .collect()
}

fn parse_biquge_book_detail(book_id: &str, html: &str) -> Result<NovelBook, String> {
    let document = Html::parse_document(html);
    let title = select_text(&document, ".book-info h1")
        .or_else(|| meta_content(&document, "meta[property='og:title']"))
        .ok_or_else(|| "笔趣阁作品标题为空".to_string())?;
    let author = select_text(&document, ".book-info .author")
        .map(|value| value.trim_end_matches('著').trim().to_string())
        .filter(|value| !value.is_empty());
    let summary = meta_content(&document, "meta[name='description']");
    let cover_url = document
        .select(&Selector::parse(".book-img img").expect("valid Biquge cover selector"))
        .next()
        .and_then(|node| node.value().attr("src"))
        .map(absolute_biquge_url);
    let tag_selector = Selector::parse(".book-info .tag span").expect("valid Biquge tag selector");
    let subjects = document
        .select(&tag_selector)
        .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty())
        .take(8)
        .collect();
    Ok(NovelBook {
        id: book_id.to_string(),
        source: "biquge".to_string(),
        title,
        author,
        summary,
        cover_url,
        language: Some("zh".to_string()),
        subjects,
        public_domain: false,
        source_url: format!("{BIQUGE_BASE}/{book_id}/"),
        download_url: None,
        download_format: None,
    })
}

fn parse_biquge_catalog_pages(book_id: &str, html: &str) -> Vec<(usize, String)> {
    let document = Html::parse_document(html);
    let selector =
        Selector::parse("select option, .page_num a").expect("valid Biquge catalog page selector");
    let pattern = Regex::new(&format!(r"^/{}/ml(\d+)\.html$", regex::escape(book_id)))
        .expect("valid Biquge catalog regex");
    let mut pages = document
        .select(&selector)
        .filter_map(|node| {
            node.value()
                .attr("value")
                .or_else(|| node.value().attr("href"))
        })
        .filter_map(|path| {
            let page = pattern
                .captures(path)?
                .get(1)?
                .as_str()
                .parse::<usize>()
                .ok()?;
            (page <= MAX_CATALOG_PAGES).then(|| (page, path.to_string()))
        })
        .collect::<Vec<_>>();
    pages.sort_by_key(|(page, _)| *page);
    pages.dedup_by_key(|(page, _)| *page);
    pages
}

fn parse_biquge_chapters(book_id: &str, html: &str) -> Vec<(String, String)> {
    let document = Html::parse_document(html);
    let selector =
        Selector::parse(".chapter-list a, .section-list a").expect("valid Biquge chapter selector");
    document
        .select(&selector)
        .filter_map(|link| {
            let href = link.value().attr("href")?;
            let prefix = format!("/{book_id}/");
            let id = href.strip_prefix(&prefix)?;
            if !is_biquge_chapter_id(id) {
                return None;
            }
            let title = normalize_text(&link.text().collect::<Vec<_>>().join(" "));
            (!title.is_empty()).then(|| (id.to_string(), title))
        })
        .collect()
}

fn parse_biquge_chapter_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let title = select_text(&document, "#chaptername, .chaptername")?;
    let page_suffix = Regex::new(r"[（(]第\d+页[）)]$").expect("valid Biquge page suffix regex");
    Some(page_suffix.replace(&title, "").trim().to_string())
}

fn parse_biquge_chapter_content(html: &str) -> String {
    let document = Html::parse_document(html);
    let paragraph_selector =
        Selector::parse("#txt p, .txt p").expect("valid Biquge paragraph selector");
    let mut paragraphs = document
        .select(&paragraph_selector)
        .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|line| !line.is_empty() && !is_biquge_boilerplate(line))
        .collect::<Vec<_>>();
    if paragraphs.is_empty() {
        let container_selector = Selector::parse("#txt, .txt").expect("valid Biquge text selector");
        if let Some(container) = document.select(&container_selector).next() {
            paragraphs = container
                .text()
                .map(str::trim)
                .filter(|line| {
                    !line.is_empty()
                        && !line.contains("document.writeln")
                        && !is_biquge_boilerplate(line)
                })
                .map(str::to_string)
                .collect();
        }
    }
    if paragraphs.is_empty() {
        paragraphs = parse_biquge_base64_paragraphs(html);
    }
    normalize_text(&paragraphs.join("\n\n"))
}

fn parse_biquge_base64_paragraphs(html: &str) -> Vec<String> {
    let script_pattern = Regex::new(
        r#"document\.writeln\s*\(\s*uvbrpleo\.drhunkab\s*\(\s*['\"]([A-Za-z0-9+/=\s]+)['\"]\s*\)\s*\)"#,
    )
    .expect("valid Biquge Base64 script regex");
    let paragraph_selector = Selector::parse("p").expect("valid Biquge decoded paragraph selector");
    script_pattern
        .captures_iter(html)
        .filter_map(|captures| captures.get(1))
        .filter_map(|encoded| decode_base64_html(encoded.as_str()))
        .flat_map(|decoded| {
            let document = Html::parse_fragment(&decoded);
            document
                .select(&paragraph_selector)
                .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
                .filter(|line| !line.is_empty() && !is_biquge_boilerplate(line))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn decode_base64_html(value: &str) -> Option<String> {
    let value = value.split_whitespace().collect::<String>();
    if value.is_empty() || value.len() > MAX_TEXT_BYTES * 2 || value.len() % 4 != 0 {
        return None;
    }
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    for chunk in value.as_bytes().chunks_exact(4) {
        let mut values = [0u8; 4];
        let mut padding = 0usize;
        for (index, byte) in chunk.iter().copied().enumerate() {
            values[index] = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' if index >= 2 => {
                    padding += 1;
                    0
                }
                _ => return None,
            };
        }
        if padding > 2 || (padding > 0 && chunk[3] != b'=') || (padding == 2 && chunk[2] != b'=') {
            return None;
        }
        let triple = ((values[0] as u32) << 18)
            | ((values[1] as u32) << 12)
            | ((values[2] as u32) << 6)
            | values[3] as u32;
        output.push((triple >> 16) as u8);
        if padding < 2 {
            output.push((triple >> 8) as u8);
        }
        if padding == 0 {
            output.push(triple as u8);
        }
        if output.len() > MAX_TEXT_BYTES {
            return None;
        }
    }
    String::from_utf8(output).ok()
}

fn parse_biquge_next_page(html: &str) -> Option<String> {
    let pattern =
        Regex::new(r#"var\s+hhekgsv=['"]([^'"]+)['"]"#).expect("valid Biquge next-page regex");
    pattern
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
}

fn select_text(document: &Html, selector: &str) -> Option<String> {
    document
        .select(&Selector::parse(selector).ok()?)
        .next()
        .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty())
}

fn meta_content(document: &Html, selector: &str) -> Option<String> {
    document
        .select(&Selector::parse(selector).ok()?)
        .next()
        .and_then(|node| node.value().attr("content"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn validate_biquge_book_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 48
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric());
    valid
        .then(|| value.to_ascii_lowercase())
        .ok_or_else(|| "笔趣阁作品 ID 无效".to_string())
}

fn validate_biquge_chapter_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    is_biquge_chapter_id(value)
        .then(|| value.to_string())
        .ok_or_else(|| "笔趣阁章节 ID 无效".to_string())
}

fn is_biquge_chapter_id(value: &str) -> bool {
    value.len() <= 96
        && value.ends_with(".html")
        && !value.starts_with("ml")
        && !value.starts_with("dx")
        && value
            .trim_end_matches(".html")
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn biquge_book_id_from_href(href: &str) -> Option<String> {
    let path = url::Url::parse(BIQUGE_BASE).ok()?.join(href).ok()?;
    let mut segments = path.path_segments()?;
    validate_biquge_book_id(segments.next()?).ok()
}

fn absolute_biquge_url(path: &str) -> String {
    url::Url::parse(BIQUGE_BASE)
        .and_then(|base| base.join(path))
        .map(|url| url.to_string())
        .unwrap_or_else(|_| format!("{BIQUGE_BASE}/"))
}

fn is_biquge_boilerplate(line: &str) -> bool {
    let normalized = line.trim();
    normalized.contains("一秒记住")
        || normalized.contains("请勿开启浏览器阅读模式")
        || normalized.contains("www.biquge.com.tw")
}

async fn search_x80(query: &str) -> Result<Vec<NovelBook>, String> {
    let url = format!("{X80_BASE}/search.php?q={}&p=1", urlencoding::encode(query));
    let html = fetch_html(&url, 30, "80 小说网搜索").await?;
    let books = parse_x80_search(&html);
    if books.is_empty() {
        tracing::warn!(query, "80x search returned no parseable results");
    }
    Ok(books)
}

async fn detail_x80(book_id: &str) -> Result<NovelDetail, String> {
    let book_id = validate_x80_book_id(book_id)?;
    let source_url = format!("{X80_BASE}/{book_id}/");
    let detail_html = fetch_html(&source_url, 30, "80 小说网作品详情").await?;
    let book = parse_x80_book_detail(&book_id, &detail_html)?;

    let first_catalog_url = format!("{X80_BASE}/{book_id}/index_1.html");
    let first_catalog = fetch_html(&first_catalog_url, 30, "80 小说网章节目录")
        .await
        .unwrap_or_else(|_| detail_html.clone());
    let mut catalog_pages = parse_x80_catalog_pages(&book_id, &first_catalog);
    if !catalog_pages.iter().any(|(page, _)| *page == 1) {
        catalog_pages.push((1, format!("/{book_id}/index_1.html")));
    }
    catalog_pages.sort_by_key(|(page, _)| *page);
    catalog_pages.dedup_by_key(|(page, _)| *page);
    catalog_pages.truncate(MAX_CATALOG_PAGES);

    let fetched_pages = stream::iter(catalog_pages.into_iter().map(|(page, path)| async move {
        let url = absolute_x80_url(&path);
        (page, fetch_html(&url, 30, "80 小说网章节目录").await)
    }))
    .buffer_unordered(4)
    .collect::<Vec<_>>()
    .await;

    let mut fetched_pages = fetched_pages;
    fetched_pages.sort_by_key(|(page, _)| *page);
    let mut seen = HashSet::new();
    let mut chapters = Vec::new();
    for (_, page) in fetched_pages {
        let Ok(page) = page else { continue };
        for (id, title) in parse_x80_chapters(&book_id, &page) {
            if seen.insert(id.clone()) {
                chapters.push(NovelChapter {
                    id,
                    title,
                    order: chapters.len() + 1,
                });
                if chapters.len() >= MAX_CHAPTERS {
                    break;
                }
            }
        }
        if chapters.len() >= MAX_CHAPTERS {
            break;
        }
    }
    if chapters.is_empty() {
        return Err("80 小说网未返回可读取的章节目录".to_string());
    }

    Ok(NovelDetail { book, chapters })
}

async fn read_x80(book_id: &str, chapter_id: &str) -> Result<NovelChapterContent, String> {
    let book_id = validate_x80_book_id(book_id)?;
    let chapter_id = validate_x80_chapter_id(chapter_id)?;
    let mut next_chapter_id = chapter_id.clone();
    let mut page_number = 1usize;
    let mut visited = HashSet::new();
    let mut parts = Vec::new();
    let mut chapter_title = String::new();

    for _ in 0..MAX_CHAPTER_PAGES {
        if !visited.insert(next_chapter_id.clone()) {
            break;
        }
        let url = format!("{X80_BASE}/{book_id}/{next_chapter_id}");
        let html = match fetch_html(&url, 30, "80 小说网章节正文").await {
            Ok(html) => html,
            Err(error) if parts.is_empty() => return Err(error),
            Err(_) => break,
        };
        if chapter_title.is_empty() {
            chapter_title = parse_x80_chapter_title(&html)
                .unwrap_or_else(|| chapter_id.trim_end_matches(".html").to_string());
        }
        let content = parse_x80_chapter_content(&html);
        if !content.is_empty() {
            parts.push(content);
        }
        let Some(candidate) = parse_x80_next_page(&html, &book_id, &chapter_id, page_number) else {
            break;
        };
        next_chapter_id = candidate;
        page_number += 1;
    }

    let content = normalize_text(&parts.join("\n\n"));
    if content.is_empty() {
        return Err("80 小说网章节正文为空".to_string());
    }

    Ok(NovelChapterContent {
        book_id: book_id.clone(),
        source: "x80".to_string(),
        chapter: NovelChapter {
            id: chapter_id,
            title: chapter_title,
            order: 1,
        },
        content,
    })
}

fn parse_x80_search(html: &str) -> Vec<NovelBook> {
    let document = Html::parse_document(html);
    let item_selector =
        Selector::parse("div.col-12.col-md-6 > dl").expect("valid 80x search item selector");
    let title_selector = Selector::parse("dd h3 a").expect("valid 80x search title selector");
    let author_selector =
        Selector::parse("dd.book_other span").expect("valid 80x search author selector");
    let cover_selector = Selector::parse("dt img").expect("valid 80x search cover selector");
    document
        .select(&item_selector)
        .filter_map(|item| {
            let title_link = item.select(&title_selector).next()?;
            let id = x80_book_id_from_href(title_link.value().attr("href")?)?;
            let raw_title = normalize_text(&title_link.text().collect::<Vec<_>>().join(" "));
            let title = strip_x80_category_prefix(&raw_title);
            if title.is_empty() {
                return None;
            }
            let author = item
                .select(&author_selector)
                .next()
                .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
                .filter(|value| !value.is_empty());
            let cover_url = item
                .select(&cover_selector)
                .next()
                .and_then(|node| node.value().attr("src"))
                .map(absolute_x80_url);
            let subjects = x80_category_from_title(&raw_title).into_iter().collect();
            Some(NovelBook {
                id: id.clone(),
                source: "x80".to_string(),
                title,
                author,
                summary: None,
                cover_url,
                language: Some("zh".to_string()),
                subjects,
                public_domain: false,
                source_url: format!("{X80_BASE}/{id}/"),
                download_url: None,
                download_format: None,
            })
        })
        .take(24)
        .collect()
}

fn parse_x80_book_detail(book_id: &str, html: &str) -> Result<NovelBook, String> {
    let document = Html::parse_document(html);
    let title = meta_content(&document, "meta[property='og:novel:book_name']")
        .or_else(|| select_text(&document, ".book_info h1, h1"))
        .or_else(|| meta_content(&document, "meta[property='og:title']"))
        .map(|value| value.trim_end_matches("最新章节").trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "80 小说网作品标题为空".to_string())?;
    let author = meta_content(&document, "meta[property='og:novel:author']")
        .or_else(|| select_text(&document, ".book_info .options a"))
        .filter(|value| !value.is_empty());
    let summary = meta_content(&document, "meta[property='og:description']");
    let cover_url =
        meta_content(&document, "meta[property='og:image']").map(|url| absolute_x80_url(&url));
    let subjects = meta_content(&document, "meta[property='og:novel:category']")
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect();
    Ok(NovelBook {
        id: book_id.to_string(),
        source: "x80".to_string(),
        title,
        author,
        summary,
        cover_url,
        language: Some("zh".to_string()),
        subjects,
        public_domain: false,
        source_url: format!("{X80_BASE}/{book_id}/"),
        download_url: None,
        download_format: None,
    })
}

fn parse_x80_catalog_pages(book_id: &str, html: &str) -> Vec<(usize, String)> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(".pages a[href], a[href*='index_']")
        .expect("valid 80x catalog page selector");
    let mut pages = document
        .select(&selector)
        .filter_map(|link| link.value().attr("href"))
        .filter_map(|href| x80_catalog_page_from_href(book_id, href))
        .filter(|(page, _)| *page >= 1 && *page <= MAX_CATALOG_PAGES)
        .collect::<Vec<_>>();
    pages.sort_by_key(|(page, _)| *page);
    pages.dedup_by_key(|(page, _)| *page);
    pages
}

fn parse_x80_chapters(book_id: &str, html: &str) -> Vec<(String, String)> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(".book_list.book_list2 a").expect("valid 80x chapter selector");
    document
        .select(&selector)
        .filter_map(|link| {
            let id = x80_chapter_id_from_href(book_id, link.value().attr("href")?)?;
            let title = normalize_text(&link.text().collect::<Vec<_>>().join(" "));
            (!title.is_empty()).then_some((id, title))
        })
        .collect()
}

fn parse_x80_chapter_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let title = select_text(&document, "h1")?;
    let page_suffix = Regex::new(r"\s*[?(]???\s*\(?\d+\s*/\s*\d+\)?\s*?[?)]?\s*$")
        .expect("valid 80x page suffix regex");
    Some(page_suffix.replace(&title, "").trim().to_string())
}

fn parse_x80_chapter_content(html: &str) -> String {
    let document = Html::parse_document(html);
    let paragraph_selector =
        Selector::parse("article p").expect("valid 80x article paragraph selector");
    let mut paragraphs = document
        .select(&paragraph_selector)
        .map(|node| normalize_text(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|line| !line.is_empty() && !is_x80_boilerplate(line))
        .collect::<Vec<_>>();
    if paragraphs.is_empty() {
        let article_selector = Selector::parse("article").expect("valid 80x article selector");
        if let Some(article) = document.select(&article_selector).next() {
            paragraphs = article
                .text()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !is_x80_boilerplate(line))
                .map(str::to_string)
                .collect();
        }
    }
    normalize_text(&paragraphs.join("\n\n"))
}

fn parse_x80_next_page(
    html: &str,
    book_id: &str,
    chapter_id: &str,
    current_page: usize,
) -> Option<String> {
    let document = Html::parse_document(html);
    let next_selector = Selector::parse("#next[href]").expect("valid 80x next-page selector");
    let href = document
        .select(&next_selector)
        .next()?
        .value()
        .attr("href")?;
    let candidate = x80_chapter_id_from_href(book_id, href)?;
    let chapter_stem = chapter_id.trim_end_matches(".html");
    let expected = format!("{chapter_stem}_{}.html", current_page + 1);
    (candidate == expected).then_some(candidate)
}

fn validate_x80_book_id(value: &str) -> Result<String, String> {
    let segments = value
        .trim()
        .trim_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    let valid = segments.len() == 2
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment.len() <= 12
                && segment.chars().all(|character| character.is_ascii_digit())
        });
    valid
        .then(|| format!("{}/{}", segments[0], segments[1]))
        .ok_or_else(|| "80 小说网作品 ID 无效".to_string())
}

fn validate_x80_chapter_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    let valid = value.len() <= 40
        && value.ends_with(".html")
        && value
            .trim_end_matches(".html")
            .split_once('_')
            .map(|(stem, page)| {
                !stem.is_empty()
                    && stem.chars().all(|character| character.is_ascii_digit())
                    && page.parse::<usize>().is_ok_and(|page| page >= 2)
            })
            .unwrap_or_else(|| {
                !value.trim_end_matches(".html").is_empty()
                    && value
                        .trim_end_matches(".html")
                        .chars()
                        .all(|character| character.is_ascii_digit())
            });
    valid
        .then(|| value.to_string())
        .ok_or_else(|| "80 小说网章节 ID 无效".to_string())
}

fn x80_path(value: &str) -> Option<String> {
    let url = url::Url::parse(X80_BASE).ok()?.join(value).ok()?;
    (url.scheme() == "https" && url.host_str() == Some("www.80xsw.com"))
        .then(|| url.path().to_string())
}

fn x80_book_id_from_href(href: &str) -> Option<String> {
    let path = x80_path(href)?;
    let segments = path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    (segments.len() == 2)
        .then(|| format!("{}/{}", segments[0], segments[1]))
        .and_then(|id| validate_x80_book_id(&id).ok())
}

fn x80_catalog_page_from_href(book_id: &str, href: &str) -> Option<(usize, String)> {
    let path = x80_path(href)?;
    let pattern = Regex::new(&format!(r"^/{}/index_(\d+)\.html$", regex::escape(book_id)))
        .expect("valid 80x catalog page regex");
    let page = pattern
        .captures(&path)?
        .get(1)?
        .as_str()
        .parse::<usize>()
        .ok()?;
    Some((page, path))
}

fn x80_chapter_id_from_href(book_id: &str, href: &str) -> Option<String> {
    let path = x80_path(href)?;
    let prefix = format!("/{book_id}/");
    let chapter_id = path.strip_prefix(&prefix)?;
    (!chapter_id.contains('/'))
        .then(|| validate_x80_chapter_id(chapter_id).ok())
        .flatten()
}

fn absolute_x80_url(path: &str) -> String {
    let Ok(base) = url::Url::parse(X80_BASE) else {
        return format!("{X80_BASE}/");
    };
    let Ok(url) = base.join(path) else {
        return format!("{X80_BASE}/");
    };
    if url.scheme() == "https" && url.host_str() == Some("www.80xsw.com") {
        url.to_string()
    } else {
        format!("{X80_BASE}/")
    }
}

fn x80_category_from_title(title: &str) -> Option<String> {
    title
        .strip_prefix('[')
        .and_then(|value| value.split_once(']'))
        .map(|(category, _)| normalize_text(category))
        .filter(|category| !category.is_empty())
}

fn strip_x80_category_prefix(title: &str) -> String {
    title
        .strip_prefix('[')
        .and_then(|value| value.split_once(']').map(|(_, title)| title))
        .map(normalize_text)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| title.to_string())
}

fn is_x80_boilerplate(line: &str) -> bool {
    let line = line.trim();
    line.contains("80小说网") || line.contains("请收藏本站") || line.contains("手机用户请浏览")
}

async fn search_internet_archive(query: &str) -> Result<Vec<NovelBook>, String> {
    let escaped = query.replace('\\', "\\\\").replace('"', "\\\"");
    let archive_query = format!(
        "(title:(\"{escaped}\") OR creator:(\"{escaped}\")) AND mediatype:texts AND collection:opensource"
    );
    let url = format!(
        "{INTERNET_ARCHIVE_API}?q={}&fl%5B%5D=identifier&fl%5B%5D=title&fl%5B%5D=creator&fl%5B%5D=description&fl%5B%5D=language&fl%5B%5D=subject&fl%5B%5D=licenseurl&fl%5B%5D=rights&rows=20&page=1&output=json",
        urlencoding::encode(&archive_query)
    );
    let payload = fetch_json(&url, 35).await?;
    Ok(parse_internet_archive_search(&payload))
}

fn parse_internet_archive_search(payload: &Value) -> Vec<NovelBook> {
    payload
        .pointer("/response/docs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|doc| {
            let id = doc.get("identifier")?.as_str()?.trim();
            validate_archive_identifier(id).ok()?;
            let title = value_first_string(doc.get("title"))?;
            let subjects = value_strings(doc.get("subject"))
                .into_iter()
                .take(8)
                .collect::<Vec<_>>();
            Some(NovelBook {
                id: id.to_string(),
                source: "internetarchive".to_string(),
                title,
                author: value_first_string(doc.get("creator")),
                summary: value_first_string(doc.get("description")),
                cover_url: Some(format!("https://archive.org/services/img/{id}")),
                language: value_first_string(doc.get("language")),
                subjects,
                public_domain: archive_metadata_is_public_domain(doc),
                source_url: format!("https://archive.org/details/{id}"),
                download_url: None,
                download_format: None,
            })
        })
        .take(20)
        .collect()
}

async fn detail_internet_archive(book_id: &str) -> Result<NovelDetail, String> {
    let identifier = validate_archive_identifier(book_id)?;
    let payload = fetch_json(&format!("{INTERNET_ARCHIVE_METADATA}/{identifier}"), 45).await?;
    archive_detail_from_metadata(&identifier, &payload, "internetarchive", None, None)
}

async fn read_internet_archive(
    book_id: &str,
    chapter_id: &str,
) -> Result<NovelChapterContent, String> {
    let identifier = validate_archive_identifier(book_id)?;
    read_archive_text("internetarchive", book_id, &identifier, chapter_id).await
}

async fn search_open_library(query: &str) -> Result<Vec<NovelBook>, String> {
    let url = format!(
        "{OPEN_LIBRARY_BASE}/search.json?q={}&limit=20&fields=key,title,author_name,cover_i,language,subject,first_publish_year,public_scan_b,ia",
        urlencoding::encode(query)
    );
    let payload = fetch_json(&url, 35).await?;
    Ok(parse_open_library_search(&payload))
}

fn parse_open_library_search(payload: &Value) -> Vec<NovelBook> {
    payload
        .get("docs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|doc| {
            if doc.get("public_scan_b").and_then(Value::as_bool) != Some(true) {
                return None;
            }
            let work_key = doc.get("key")?.as_str()?;
            let work_id = validate_open_library_work_key(work_key).ok()?;
            let archive_id = value_strings(doc.get("ia")).into_iter().next()?;
            validate_archive_identifier(&archive_id).ok()?;
            let title = doc.get("title")?.as_str()?.trim().to_string();
            if title.is_empty() {
                return None;
            }
            let year = doc.get("first_publish_year").and_then(Value::as_i64);
            Some(NovelBook {
                id: format!("{work_id}|{archive_id}"),
                source: "openlibrary".to_string(),
                title,
                author: value_strings(doc.get("author_name")).into_iter().next(),
                summary: year.map(|value| {
                    format!("首次出版于 {value} 年；Open Library 标记为可公开扫描阅读。")
                }),
                cover_url: doc
                    .get("cover_i")
                    .and_then(Value::as_i64)
                    .map(|cover| format!("https://covers.openlibrary.org/b/id/{cover}-M.jpg")),
                language: value_strings(doc.get("language")).into_iter().next(),
                subjects: value_strings(doc.get("subject"))
                    .into_iter()
                    .take(8)
                    .collect(),
                public_domain: false,
                source_url: format!("{OPEN_LIBRARY_BASE}/works/{work_id}"),
                download_url: None,
                download_format: None,
            })
        })
        .take(20)
        .collect()
}

async fn detail_open_library(book_id: &str) -> Result<NovelDetail, String> {
    let (work_id, archive_id) = parse_open_library_id(book_id)?;
    let work_payload = fetch_json(&format!("{OPEN_LIBRARY_BASE}/works/{work_id}.json"), 35).await?;
    let archive_payload =
        fetch_json(&format!("{INTERNET_ARCHIVE_METADATA}/{archive_id}"), 45).await?;
    let title = work_payload
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Open Library 作品缺少标题".to_string())?
        .to_string();
    let description = match work_payload.get("description") {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Object(value)) => value
            .get("value")
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    };
    let cover_url = work_payload
        .get("covers")
        .and_then(Value::as_array)
        .and_then(|covers| covers.first())
        .and_then(Value::as_i64)
        .map(|cover| format!("https://covers.openlibrary.org/b/id/{cover}-L.jpg"));
    let subjects = value_strings(work_payload.get("subjects"))
        .into_iter()
        .take(10)
        .collect::<Vec<_>>();
    archive_detail_from_metadata(
        &archive_id,
        &archive_payload,
        "openlibrary",
        Some((book_id.to_string(), title, description, cover_url, subjects)),
        Some(format!("{OPEN_LIBRARY_BASE}/works/{work_id}")),
    )
}

async fn read_open_library(book_id: &str, chapter_id: &str) -> Result<NovelChapterContent, String> {
    let (_, archive_id) = parse_open_library_id(book_id)?;
    read_archive_text("openlibrary", book_id, &archive_id, chapter_id).await
}

async fn search_standard_ebooks(query: &str) -> Result<Vec<NovelBook>, String> {
    let url = format!(
        "{STANDARD_EBOOKS_BASE}/ebooks?query={}",
        urlencoding::encode(query)
    );
    let html = fetch_html(&url, 35, "Standard Ebooks 搜索").await?;
    Ok(parse_standard_ebooks_search(&html))
}

fn parse_standard_ebooks_search(html: &str) -> Vec<NovelBook> {
    let document = Html::parse_document(html);
    let item_selector = Selector::parse("main.ebooks li[typeof='schema:Book']").unwrap();
    let link_selector = Selector::parse("a[property='schema:url']").unwrap();
    let title_selector = Selector::parse("[property='schema:name']").unwrap();
    let author_selector =
        Selector::parse("[property='schema:author'] [property='schema:name']").unwrap();
    let image_selector = Selector::parse("img[property='schema:image']").unwrap();

    document
        .select(&item_selector)
        .filter_map(|item| {
            let href = item
                .select(&link_selector)
                .filter_map(|link| link.value().attr("href"))
                .find(|href| href.starts_with("/ebooks/") && href.matches('/').count() >= 3)?;
            let id = validate_standard_ebooks_id(href.strip_prefix("/ebooks/")?).ok()?;
            let title = item
                .select(&title_selector)
                .map(|node| node.text().collect::<String>().trim().to_string())
                .find(|value| !value.is_empty())?;
            let author = item
                .select(&author_selector)
                .map(|node| node.text().collect::<String>().trim().to_string())
                .find(|value| !value.is_empty());
            let cover_url = item
                .select(&image_selector)
                .filter_map(|image| image.value().attr("src"))
                .next()
                .map(absolute_standard_ebooks_url);
            Some(NovelBook {
                id: id.clone(),
                source: "standardebooks".to_string(),
                title,
                author,
                summary: Some(
                    "Standard Ebooks 精校制作的公共领域电子书，可在详情页下载兼容 EPUB。"
                        .to_string(),
                ),
                cover_url,
                language: Some("en".to_string()),
                subjects: vec!["Standard Ebooks".to_string(), "精校 EPUB".to_string()],
                public_domain: true,
                source_url: format!("{STANDARD_EBOOKS_BASE}/ebooks/{id}"),
                download_url: None,
                download_format: None,
            })
        })
        .take(20)
        .collect()
}

async fn detail_standard_ebooks(book_id: &str) -> Result<NovelDetail, String> {
    let id = validate_standard_ebooks_id(book_id)?;
    let source_url = format!("{STANDARD_EBOOKS_BASE}/ebooks/{id}");
    let html = fetch_html(&source_url, 35, "Standard Ebooks 详情").await?;
    parse_standard_ebooks_detail(&id, &source_url, &html)
}

fn parse_standard_ebooks_detail(
    id: &str,
    source_url: &str,
    html: &str,
) -> Result<NovelDetail, String> {
    let document = Html::parse_document(html);
    let title = select_text(&document, "h1[property='schema:name']")
        .ok_or_else(|| "Standard Ebooks 作品缺少标题".to_string())?;
    let author = select_text(
        &document,
        "a[property='schema:author'] [property='schema:name']",
    );
    let summary = meta_content(&document, "meta[property='schema:description']")
        .or_else(|| meta_content(&document, "meta[name='description']"));
    let cover_url = meta_content(&document, "meta[property='og:image']");
    let subject_selector = Selector::parse("a[href^='/subjects/']").unwrap();
    let mut subjects = document
        .select(&subject_selector)
        .map(|node| node.text().collect::<String>().trim().to_string())
        .filter(|value| !value.is_empty())
        .take(8)
        .collect::<Vec<_>>();
    if subjects.is_empty() {
        subjects.push("Standard Ebooks".to_string());
    }
    let download_selector =
        Selector::parse("a[property='schema:contentUrl'][href$='.epub']").unwrap();
    let epub_links = document
        .select(&download_selector)
        .filter_map(|link| link.value().attr("href"))
        .collect::<Vec<_>>();
    let download_url = epub_links
        .iter()
        .copied()
        .find(|href| !href.ends_with(".kepub.epub") && !href.contains("_advanced.epub"))
        .or_else(|| epub_links.first().copied())
        .map(absolute_standard_ebooks_url);

    Ok(NovelDetail {
        book: NovelBook {
            id: id.to_string(),
            source: "standardebooks".to_string(),
            title,
            author,
            summary,
            cover_url,
            language: Some("en".to_string()),
            subjects,
            public_domain: true,
            source_url: source_url.to_string(),
            download_format: download_url.as_ref().map(|_| "EPUB".to_string()),
            download_url,
        },
        // Standard Ebooks publishes downloadable EPUB files rather than a stable
        // plain-text chapter API, so the app intentionally exposes download only.
        chapters: Vec::new(),
    })
}

type ArchiveBookOverride = (String, String, Option<String>, Option<String>, Vec<String>);

fn archive_detail_from_metadata(
    archive_id: &str,
    payload: &Value,
    source: &str,
    override_book: Option<ArchiveBookOverride>,
    source_url_override: Option<String>,
) -> Result<NovelDetail, String> {
    let metadata = payload
        .get("metadata")
        .ok_or_else(|| "Internet Archive 未返回作品元数据".to_string())?;
    let (download_file, text_file) = archive_file_choices(payload);
    let (book_id, title, summary, cover_url, subjects) = override_book.unwrap_or_else(|| {
        (
            archive_id.to_string(),
            value_first_string(metadata.get("title")).unwrap_or_else(|| archive_id.to_string()),
            value_first_string(metadata.get("description")),
            Some(format!("https://archive.org/services/img/{archive_id}")),
            value_strings(metadata.get("subject"))
                .into_iter()
                .take(10)
                .collect(),
        )
    });
    let download_url = download_file.as_ref().map(|(name, _)| {
        format!(
            "{INTERNET_ARCHIVE_DOWNLOAD}/{archive_id}/{}",
            urlencoding::encode(name)
        )
    });
    let download_format = download_file.as_ref().map(|(_, format)| format.clone());
    let chapters = text_file
        .into_iter()
        .map(|name| NovelChapter {
            id: name,
            title: "全文（Internet Archive 纯文本）".to_string(),
            order: 1,
        })
        .collect();
    Ok(NovelDetail {
        book: NovelBook {
            id: book_id,
            source: source.to_string(),
            title,
            author: value_first_string(metadata.get("creator")),
            summary,
            cover_url,
            language: value_first_string(metadata.get("language")),
            subjects,
            public_domain: archive_metadata_is_public_domain(metadata),
            source_url: source_url_override
                .unwrap_or_else(|| format!("https://archive.org/details/{archive_id}")),
            download_url,
            download_format,
        },
        chapters,
    })
}

fn archive_file_choices(payload: &Value) -> (Option<(String, String)>, Option<String>) {
    let files = payload
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let names = files
        .iter()
        .filter_map(|file| file.get("name").and_then(Value::as_str))
        .filter(|name| !name.contains('/') && !name.contains('\\') && !name.contains(".."))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let preferred = [(".epub", "EPUB"), (".pdf", "PDF"), (".txt", "TXT")];
    let download = preferred.iter().find_map(|(extension, format)| {
        names
            .iter()
            .find(|name| name.to_ascii_lowercase().ends_with(extension))
            .map(|name| (name.clone(), (*format).to_string()))
    });
    let text = names
        .iter()
        .find(|name| {
            let lower = name.to_ascii_lowercase();
            lower.ends_with("_djvu.txt") || lower.ends_with(".txt")
        })
        .cloned();
    (download, text)
}

async fn read_archive_text(
    source: &str,
    book_id: &str,
    archive_id: &str,
    chapter_id: &str,
) -> Result<NovelChapterContent, String> {
    let filename = validate_archive_text_filename(chapter_id)?;
    let url = format!(
        "{INTERNET_ARCHIVE_DOWNLOAD}/{archive_id}/{}",
        urlencoding::encode(&filename)
    );
    let response = novel_client(60)
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("Internet Archive 正文请求失败: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Internet Archive 正文返回 HTTP {}",
            response.status()
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("读取 Internet Archive 正文失败: {error}"))?;
    if bytes.len() > MAX_TEXT_BYTES {
        return Err("Internet Archive 正文超过应用内阅读大小限制，请改用下载文件".to_string());
    }
    let content = normalize_text(&decode_text(&bytes));
    if content.is_empty() {
        return Err("Internet Archive 未返回可阅读正文".to_string());
    }
    Ok(NovelChapterContent {
        book_id: book_id.to_string(),
        source: source.to_string(),
        chapter: NovelChapter {
            id: filename,
            title: "全文（Internet Archive 纯文本）".to_string(),
            order: 1,
        },
        content,
    })
}

fn validate_archive_identifier(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let valid = !trimmed.is_empty()
        && trimmed.len() <= 160
        && trimmed
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character));
    valid
        .then(|| trimmed.to_string())
        .ok_or_else(|| "无效的 Internet Archive 作品编号".to_string())
}

fn validate_archive_text_filename(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    let valid = !trimmed.is_empty()
        && trimmed.len() <= 240
        && !trimmed.contains('/')
        && !trimmed.contains('\\')
        && !trimmed.contains("..")
        && lower.ends_with(".txt");
    valid
        .then(|| trimmed.to_string())
        .ok_or_else(|| "无效的 Internet Archive 正文文件".to_string())
}

fn validate_open_library_work_key(value: &str) -> Result<String, String> {
    let work_id = value.trim().strip_prefix("/works/").unwrap_or(value.trim());
    let digits = work_id
        .strip_prefix("OL")
        .and_then(|value| value.strip_suffix('W'));
    let valid = work_id.starts_with("OL")
        && work_id.ends_with('W')
        && work_id.len() <= 32
        && digits.is_some_and(|digits| {
            !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
        });
    valid
        .then(|| work_id.to_string())
        .ok_or_else(|| "无效的 Open Library 作品编号".to_string())
}

fn parse_open_library_id(value: &str) -> Result<(String, String), String> {
    let (work_id, archive_id) = value
        .split_once('|')
        .ok_or_else(|| "Open Library 作品编号缺少公开扫描信息".to_string())?;
    Ok((
        validate_open_library_work_key(work_id)?,
        validate_archive_identifier(archive_id)?,
    ))
}

fn validate_standard_ebooks_id(value: &str) -> Result<String, String> {
    let trimmed = value.trim().trim_matches('/');
    let valid = !trimmed.is_empty()
        && trimmed.len() <= 180
        && trimmed.split('/').count() >= 2
        && trimmed.split('/').all(|segment| {
            !segment.is_empty()
                && segment.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                })
        });
    valid
        .then(|| trimmed.to_string())
        .ok_or_else(|| "无效的 Standard Ebooks 作品编号".to_string())
}

fn absolute_standard_ebooks_url(path: &str) -> String {
    if path.starts_with("https://") {
        path.to_string()
    } else {
        format!("{STANDARD_EBOOKS_BASE}/{}", path.trim_start_matches('/'))
    }
}

fn value_strings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(value)) => vec![value.trim().to_string()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn value_first_string(value: Option<&Value>) -> Option<String> {
    value_strings(value)
        .into_iter()
        .find(|value| !value.is_empty())
}

fn archive_metadata_is_public_domain(metadata: &Value) -> bool {
    ["rights", "licenseurl"]
        .into_iter()
        .filter_map(|key| value_first_string(metadata.get(key)))
        .any(|value| {
            let lower = value.to_ascii_lowercase();
            let explicitly_not_public_domain = lower.contains("not public domain")
                || lower.contains("no public domain")
                || lower.contains("not in the public domain");
            !explicitly_not_public_domain
                && (lower.contains("public domain")
                    || lower.contains("publicdomain")
                    || lower.contains("creativecommons.org/publicdomain/zero"))
        })
}

async fn search_gutenberg(query: &str) -> Result<Vec<NovelBook>, String> {
    let url = format!(
        "{GUTENDEX_API}/books/?search={}&copyright=false",
        urlencoding::encode(query)
    );
    let payload = novel_client(35)
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Gutendex 搜索失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Gutendex 搜索失败: {error}"))?
        .json::<GutendexList>()
        .await
        .map_err(|error| format!("Gutendex 响应解析失败: {error}"))?;

    Ok(payload
        .results
        .into_iter()
        .filter(|book| book.copyright != Some(true))
        .take(24)
        .map(gutendex_book_to_dto)
        .collect())
}

async fn detail_gutenberg(book_id: &str) -> Result<NovelDetail, String> {
    let id = parse_numeric_id(book_id)?;
    let book = fetch_gutenberg_book(id).await?;
    Ok(NovelDetail {
        book: gutendex_book_to_dto(book),
        chapters: vec![NovelChapter {
            id: "full".to_string(),
            title: "全文".to_string(),
            order: 1,
        }],
    })
}

async fn read_gutenberg(book_id: &str, chapter_id: &str) -> Result<NovelChapterContent, String> {
    if chapter_id != "full" {
        return Err("Project Gutenberg 当前仅提供全文阅读".to_string());
    }
    let id = parse_numeric_id(book_id)?;
    let book = fetch_gutenberg_book(id).await?;
    let (text_url, is_html) = preferred_gutenberg_text(&book.formats)
        .ok_or_else(|| "该书没有可读取的纯文本或 HTML 格式".to_string())?;
    let bytes = novel_client(45)
        .get(text_url)
        .send()
        .await
        .map_err(|error| format!("正文下载失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("正文下载失败: {error}"))?
        .bytes()
        .await
        .map_err(|error| format!("正文读取失败: {error}"))?;
    if bytes.len() > MAX_TEXT_BYTES {
        return Err("正文超过 8 MiB 安全上限，请改用 EPUB 下载".to_string());
    }
    let decoded = decode_text(&bytes);
    let content = if is_html {
        html_to_plain_text(&decoded)
    } else {
        clean_gutenberg_markers(&decoded)
    };
    if content.trim().is_empty() {
        return Err("正文为空".to_string());
    }

    Ok(NovelChapterContent {
        book_id: id.to_string(),
        source: "gutenberg".to_string(),
        chapter: NovelChapter {
            id: "full".to_string(),
            title: "全文".to_string(),
            order: 1,
        },
        content,
    })
}

async fn fetch_gutenberg_book(id: u64) -> Result<GutendexBook, String> {
    novel_client(35)
        .get(format!("{GUTENDEX_API}/books/{id}"))
        .send()
        .await
        .map_err(|error| format!("Gutendex 详情请求失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Gutendex 详情请求失败: {error}"))?
        .json::<GutendexBook>()
        .await
        .map_err(|error| format!("Gutendex 详情解析失败: {error}"))
}

fn gutendex_book_to_dto(book: GutendexBook) -> NovelBook {
    let author = (!book.authors.is_empty()).then(|| {
        book.authors
            .iter()
            .map(|person| person.name.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    });
    let cover_url = book.formats.get("image/jpeg").cloned();
    let download_url = book
        .formats
        .get("application/epub+zip")
        .cloned()
        .or_else(|| {
            book.formats
                .iter()
                .find(|(mime, _)| mime.starts_with("application/epub"))
                .map(|(_, url)| url.clone())
        });
    let mut subjects = book.subjects;
    subjects.extend(book.bookshelves);
    subjects.sort();
    subjects.dedup();
    subjects.truncate(8);

    NovelBook {
        id: book.id.to_string(),
        source: "gutenberg".to_string(),
        title: book.title,
        author,
        summary: book
            .summaries
            .into_iter()
            .find(|value| !value.trim().is_empty()),
        cover_url,
        language: (!book.languages.is_empty()).then(|| book.languages.join(" / ")),
        subjects,
        public_domain: book.copyright == Some(false),
        source_url: format!("https://www.gutenberg.org/ebooks/{}", book.id),
        download_format: download_url.as_ref().map(|_| "EPUB".to_string()),
        download_url,
    }
}

async fn search_wikisource(query: &str) -> Result<Vec<NovelBook>, String> {
    let url = format!(
        "{WIKISOURCE_API}?action=query&generator=search&gsrsearch={}&gsrnamespace=0&gsrlimit=20&prop=extracts%7Cpageimages&exintro=1&explaintext=1&piprop=thumbnail&pithumbsize=360&format=json&formatversion=2&origin=*",
        urlencoding::encode(query)
    );
    let payload = fetch_json(&url, 30).await?;
    let pages = payload
        .pointer("/query/pages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(pages
        .iter()
        .filter_map(wikisource_page_to_book)
        .take(20)
        .collect())
}

async fn detail_wikisource(book_id: &str) -> Result<NovelDetail, String> {
    let page_id = parse_numeric_id(book_id)?;
    let url = format!(
        "{WIKISOURCE_API}?action=parse&pageid={page_id}&prop=links%7Cdisplaytitle&format=json&formatversion=2&origin=*"
    );
    let payload = fetch_json(&url, 30).await?;
    let parse = payload
        .get("parse")
        .ok_or_else(|| mediawiki_error(&payload, "维基文库未返回作品详情"))?;
    let title = parse
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "维基文库作品缺少标题".to_string())?;
    let display_title = parse
        .get("displaytitle")
        .and_then(Value::as_str)
        .map(crate::scraper::utils::clean_html)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| title.clone());
    let mut chapters = parse
        .get("links")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|link| {
            let namespace = link.get("ns").and_then(Value::as_i64).unwrap_or(-1);
            let chapter_title = link.get("title").and_then(Value::as_str)?;
            if namespace != 0
                || chapter_title == title
                || !looks_like_chapter(&title, chapter_title)
            {
                return None;
            }
            Some(chapter_title.to_string())
        })
        .take(MAX_CHAPTERS)
        .enumerate()
        .map(|(index, chapter_title)| NovelChapter {
            id: chapter_title.clone(),
            title: chapter_display_title(&title, &chapter_title),
            order: index + 1,
        })
        .collect::<Vec<_>>();
    if chapters.is_empty() {
        chapters.push(NovelChapter {
            id: title.clone(),
            title: "全文".to_string(),
            order: 1,
        });
    }

    Ok(NovelDetail {
        book: NovelBook {
            id: page_id.to_string(),
            source: "wikisource".to_string(),
            title: display_title,
            author: None,
            summary: Some("来自中文维基文库的自由文本。".to_string()),
            cover_url: None,
            language: Some("zh".to_string()),
            subjects: vec!["维基文库".to_string(), "自由文本".to_string()],
            // Wikisource mixes public-domain works with freely licensed pages.
            // Keep the stricter flag false unless the upstream API exposes a
            // per-page public-domain declaration.
            public_domain: false,
            source_url: format!(
                "https://zh.wikisource.org/wiki/{}",
                urlencoding::encode(&title)
            ),
            download_url: None,
            download_format: None,
        },
        chapters,
    })
}

async fn read_wikisource(
    book_id: &str,
    chapter_title: &str,
) -> Result<NovelChapterContent, String> {
    let page_id = parse_numeric_id(book_id)?;
    let chapter_title = chapter_title.trim();
    if chapter_title.is_empty() || chapter_title.chars().count() > 240 {
        return Err("维基文库章节标识无效".to_string());
    }
    let url = format!(
        "{WIKISOURCE_API}?action=parse&page={}&prop=text%7Cdisplaytitle&format=json&formatversion=2&origin=*",
        urlencoding::encode(chapter_title)
    );
    let payload = fetch_json(&url, 35).await?;
    let parse = payload
        .get("parse")
        .ok_or_else(|| mediawiki_error(&payload, "维基文库未返回章节正文"))?;
    let html = parse
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "维基文库章节正文为空".to_string())?;
    if html.len() > MAX_TEXT_BYTES {
        return Err("章节超过 8 MiB 安全上限".to_string());
    }
    let content = html_to_plain_text(html);
    if content.trim().is_empty() {
        return Err("维基文库章节正文为空".to_string());
    }
    let display_title = parse
        .get("displaytitle")
        .and_then(Value::as_str)
        .map(crate::scraper::utils::clean_html)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| chapter_title.to_string());

    Ok(NovelChapterContent {
        book_id: page_id.to_string(),
        source: "wikisource".to_string(),
        chapter: NovelChapter {
            id: chapter_title.to_string(),
            title: display_title,
            order: 1,
        },
        content,
    })
}

fn wikisource_page_to_book(page: &Value) -> Option<NovelBook> {
    let id = page.get("pageid")?.as_u64()?;
    let title = page.get("title")?.as_str()?.trim();
    if title.is_empty() {
        return None;
    }
    let summary = page
        .get("extract")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| crate::scraper::utils::truncate(value, 300));
    let cover_url = page
        .pointer("/thumbnail/source")
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(NovelBook {
        id: id.to_string(),
        source: "wikisource".to_string(),
        title: title.to_string(),
        author: None,
        summary,
        cover_url,
        language: Some("zh".to_string()),
        subjects: vec!["维基文库".to_string()],
        public_domain: false,
        source_url: format!(
            "https://zh.wikisource.org/wiki/{}",
            urlencoding::encode(title)
        ),
        download_url: None,
        download_format: None,
    })
}

async fn fetch_json(url: &str, timeout_secs: u64) -> Result<Value, String> {
    novel_client(timeout_secs)
        .get(url)
        .send()
        .await
        .map_err(|error| format!("小说源请求失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("小说源请求失败: {error}"))?
        .json::<Value>()
        .await
        .map_err(|error| format!("小说源响应解析失败: {error}"))
}

fn novel_client(timeout_secs: u64) -> reqwest::Client {
    // Some public Chinese web sources close rustls HTTP/2 handshakes before sending a
    // response. Use a browser-shaped UA and HTTP/1.1 without weakening certificate checks.
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .http1_only()
        .user_agent(crate::http_client::browser_user_agent())
        .danger_accept_invalid_certs(crate::http_client::insecure_tls_enabled())
        .build()
        .unwrap_or_default()
}

fn parse_numeric_id(value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| "作品 ID 无效".to_string())
}

fn preferred_gutenberg_text(formats: &HashMap<String, String>) -> Option<(String, bool)> {
    for mime in [
        "text/plain; charset=utf-8",
        "text/plain; charset=us-ascii",
        "text/plain",
    ] {
        if let Some(url) = formats.get(mime) {
            return Some((url.clone(), false));
        }
    }
    if let Some((_, url)) = formats
        .iter()
        .find(|(mime, _)| mime.starts_with("text/plain"))
    {
        return Some((url.clone(), false));
    }
    formats
        .get("text/html")
        .cloned()
        .or_else(|| {
            formats
                .iter()
                .find(|(mime, _)| mime.starts_with("text/html"))
                .map(|(_, url)| url.clone())
        })
        .map(|url| (url, true))
}

fn decode_text(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    decoded.into_owned()
}

fn html_to_plain_text(html: &str) -> String {
    let document = Html::parse_fragment(html);
    let selector = Selector::parse(".mw-parser-output").expect("valid novel selector");
    let raw = document
        .select(&selector)
        .next()
        .map(|element| element.text().collect::<Vec<_>>().join("\n"))
        .unwrap_or_else(|| {
            document
                .root_element()
                .text()
                .collect::<Vec<_>>()
                .join("\n")
        });
    normalize_text(&raw)
}

fn clean_gutenberg_markers(text: &str) -> String {
    let start_re = Regex::new(r"(?im)^\*{3}\s*START OF (?:THE|THIS) PROJECT GUTENBERG EBOOK.*$")
        .expect("valid Gutenberg start regex");
    let end_re = Regex::new(r"(?im)^\*{3}\s*END OF (?:THE|THIS) PROJECT GUTENBERG EBOOK.*$")
        .expect("valid Gutenberg end regex");
    let start = start_re
        .find(text)
        .map(|matched| matched.end())
        .unwrap_or(0);
    let end = end_re
        .find_at(text, start)
        .map(|matched| matched.start())
        .unwrap_or(text.len());
    normalize_text(&text[start..end])
}

fn normalize_text(text: &str) -> String {
    let mut output = String::new();
    let mut previous_blank = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !previous_blank && !output.is_empty() {
                output.push('\n');
            }
            previous_blank = true;
            continue;
        }
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(trimmed);
        previous_blank = false;
    }
    output.trim().to_string()
}

fn looks_like_chapter(book_title: &str, link_title: &str) -> bool {
    let tail = link_title
        .strip_prefix(book_title)
        .unwrap_or(link_title)
        .trim_start_matches(['/', '：', ':', ' ']);
    let chinese = Regex::new(r"^第.{1,16}[回章节卷篇部]").expect("valid Chinese chapter regex");
    let numeric = Regex::new(r"^(?:卷|章|回|篇|部)?\s*[0-9一二三四五六七八九十百千零〇]+")
        .expect("valid numeric chapter regex");
    !tail.is_empty()
        && (chinese.is_match(tail)
            || numeric.is_match(tail)
            || link_title.starts_with(&format!("{book_title}/")))
}

fn chapter_display_title(book_title: &str, chapter_title: &str) -> String {
    chapter_title
        .strip_prefix(book_title)
        .unwrap_or(chapter_title)
        .trim_start_matches(['/', '：', ':', ' '])
        .to_string()
}

fn mediawiki_error(payload: &Value, fallback: &str) -> String {
    payload
        .pointer("/error/info")
        .and_then(Value::as_str)
        .map(|message| format!("维基文库请求失败: {message}"))
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_biquge_search_results() {
        let html = r#"
            <ul class="sort_list">
              <li><span class="s2"><a href="/udtuju/">诡秘之主</a></span><span class="s5"> 爱潜水的乌贼 </span></li>
            </ul>
        "#;
        let books = parse_biquge_search(html);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].id, "udtuju");
        assert_eq!(books[0].title, "诡秘之主");
        assert_eq!(books[0].author.as_deref(), Some("爱潜水的乌贼"));
        assert_eq!(books[0].source, "biquge");
    }

    #[test]
    fn parses_biquge_catalog_pages_and_chapters() {
        let html = r#"
            <select>
              <option value="/udtuju/ml1.html">第1-100章</option>
              <option value="/udtuju/ml2.html">第101-200章</option>
            </select>
            <ul class="chapter-list">
              <li><a href="/udtuju/chapter_a.html">1 第1章 绯红</a></li>
              <li><a href="/udtuju/ml2.html">下一页</a></li>
            </ul>
        "#;
        assert_eq!(
            parse_biquge_catalog_pages("udtuju", html),
            vec![
                (1, "/udtuju/ml1.html".to_string()),
                (2, "/udtuju/ml2.html".to_string())
            ]
        );
        assert_eq!(
            parse_biquge_chapters("udtuju", html),
            vec![("chapter_a.html".to_string(), "1 第1章 绯红".to_string())]
        );
    }

    #[test]
    fn parses_biquge_paginated_chapter_content_without_scripts_or_boilerplate() {
        let html = r#"
            <h1 id="chaptername">1 第1章 绯红（第1页）</h1>
            <div id="txt">
              <script>document.writeln('noise')</script>
              <p>第一段正文。</p>
              <p>一秒记住新域名 www.biquge.com.tw</p>
              <p>第二段正文。</p>
            </div>
            <script>var hhekgsv='/udtuju/chapter_a_1.html';</script>
        "#;
        assert_eq!(
            parse_biquge_chapter_title(html).as_deref(),
            Some("1 第1章 绯红")
        );
        assert_eq!(
            parse_biquge_chapter_content(html),
            "第一段正文。\n\n第二段正文。"
        );
        assert_eq!(
            parse_biquge_next_page(html).as_deref(),
            Some("/udtuju/chapter_a_1.html")
        );
    }

    #[test]
    fn rejects_biquge_path_injection() {
        assert!(validate_biquge_book_id("udtuju").is_ok());
        assert!(validate_biquge_book_id("../admin").is_err());
        assert!(validate_biquge_chapter_id("chapter_a.html").is_ok());
        assert!(validate_biquge_chapter_id("../chapter.html").is_err());
        assert!(validate_biquge_chapter_id("ml2.html").is_err());
    }

    #[tokio::test]
    #[ignore = "requires the public novel source to be online"]
    async fn biquge_live_search_catalog_and_chapter_smoke() {
        let books = search_biquge("诡秘之主")
            .await
            .expect("live Biquge search should succeed");
        let book = books
            .iter()
            .find(|book| book.title == "诡秘之主")
            .or_else(|| books.first())
            .expect("live Biquge search should return at least one result");
        let detail = detail_biquge(&book.id)
            .await
            .expect("live Biquge catalog should load");
        let chapter = detail
            .chapters
            .first()
            .expect("live Biquge catalog should contain chapters");
        let content = read_biquge(&book.id, &chapter.id)
            .await
            .expect("live Biquge chapter should load");
        assert!(content.content.chars().count() > 100);
    }

    #[test]
    fn parses_internet_archive_search_results() {
        let payload = serde_json::json!({
            "response": { "docs": [{
                "identifier": "sample_public_book",
                "title": "Sample Public Book",
                "creator": ["Example Author"],
                "description": "A public text.",
                "language": ["eng"],
                "subject": ["Fiction", "Classics"],
                "licenseurl": "https://creativecommons.org/publicdomain/zero/1.0/"
            }] }
        });
        let books = parse_internet_archive_search(&payload);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].source, "internetarchive");
        assert_eq!(books[0].id, "sample_public_book");
        assert_eq!(books[0].author.as_deref(), Some("Example Author"));
        assert!(books[0].public_domain);
    }

    #[test]
    fn parses_open_library_only_when_a_public_scan_exists() {
        let payload = serde_json::json!({
            "docs": [
                {
                    "key": "/works/OL123W",
                    "title": "Public Scan",
                    "author_name": ["Open Author"],
                    "cover_i": 42,
                    "language": ["eng"],
                    "subject": ["Novel"],
                    "first_publish_year": 1901,
                    "public_scan_b": true,
                    "ia": ["public_scan_01"]
                },
                {
                    "key": "/works/OL124W",
                    "title": "Borrow Only",
                    "public_scan_b": false,
                    "ia": ["borrow_only_01"]
                }
            ]
        });
        let books = parse_open_library_search(&payload);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].source, "openlibrary");
        assert_eq!(books[0].id, "OL123W|public_scan_01");
        assert!(books[0].source_url.ends_with("/works/OL123W"));
    }

    #[test]
    fn parses_standard_ebooks_search_and_download_detail() {
        let search_html = r#"
          <main class="ebooks"><ol><li typeof="schema:Book">
            <a property="schema:url" href="/ebooks/jane-austen/pride-and-prejudice">
              <span property="schema:name">Pride and Prejudice</span>
            </a>
            <a property="schema:author"><span property="schema:name">Jane Austen</span></a>
            <img property="schema:image" src="/images/covers/jane-austen-pride-and-prejudice.svg" />
          </li></ol></main>
        "#;
        let books = parse_standard_ebooks_search(search_html);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].source, "standardebooks");
        assert_eq!(books[0].id, "jane-austen/pride-and-prejudice");

        let detail_html = r#"
          <meta property="schema:description" content="A carefully produced edition." />
          <meta property="og:image" content="https://standardebooks.org/cover.svg" />
          <h1 property="schema:name">Pride and Prejudice</h1>
          <a property="schema:author"><span property="schema:name">Jane Austen</span></a>
          <a href="/subjects/fiction">Fiction</a>
          <a property="schema:contentUrl" href="/ebooks/jane-austen/pride-and-prejudice/downloads/jane-austen_pride-and-prejudice.epub">EPUB</a>
          <a property="schema:contentUrl" href="/ebooks/jane-austen/pride-and-prejudice/downloads/jane-austen_pride-and-prejudice.kepub.epub">Kobo</a>
        "#;
        let detail = parse_standard_ebooks_detail(
            "jane-austen/pride-and-prejudice",
            "https://standardebooks.org/ebooks/jane-austen/pride-and-prejudice",
            detail_html,
        )
        .expect("Standard Ebooks detail should parse");
        assert!(detail.chapters.is_empty());
        assert_eq!(detail.book.download_format.as_deref(), Some("EPUB"));
        assert!(detail
            .book
            .download_url
            .as_deref()
            .is_some_and(|url| url.ends_with("pride-and-prejudice.epub")));
    }

    #[test]
    fn archive_metadata_exposes_safe_text_and_download_files() {
        let payload = serde_json::json!({
            "metadata": {
                "title": "Archive Book",
                "creator": "Archive Author",
                "description": "Description",
                "language": "eng",
                "subject": ["Classics"],
                "licenseurl": "https://creativecommons.org/publicdomain/zero/1.0/"
            },
            "files": [
                { "name": "archive_book.txt", "format": "Text" },
                { "name": "archive_book.epub", "format": "EPUB" },
                { "name": "../unsafe.txt", "format": "Text" }
            ]
        });
        let detail =
            archive_detail_from_metadata("archive_book", &payload, "internetarchive", None, None)
                .expect("archive detail should parse");
        assert_eq!(detail.chapters.len(), 1);
        assert_eq!(detail.chapters[0].id, "archive_book.txt");
        assert_eq!(detail.book.download_format.as_deref(), Some("EPUB"));
        assert!(detail.book.public_domain);
    }

    #[test]
    fn recognizes_wikisource_chapter_links() {
        assert!(looks_like_chapter("紅樓夢", "紅樓夢/第一回"));
        assert!(looks_like_chapter("水滸傳", "第一百回 張三李四"));
        assert!(!looks_like_chapter("紅樓夢", "曹雪芹"));
        assert_eq!(chapter_display_title("紅樓夢", "紅樓夢/第一回"), "第一回");
    }

    #[test]
    fn strips_gutenberg_license_markers() {
        let source = "header\n*** START OF THE PROJECT GUTENBERG EBOOK TEST ***\n\nBody\n\n*** END OF THE PROJECT GUTENBERG EBOOK TEST ***\nfooter";
        assert_eq!(clean_gutenberg_markers(source), "Body");
    }

    #[test]
    fn maps_download_and_readable_formats() {
        let book = GutendexBook {
            id: 11,
            title: "Alice".to_string(),
            authors: vec![GutendexPerson {
                name: "Carroll, Lewis".to_string(),
            }],
            summaries: vec!["Summary".to_string()],
            subjects: vec!["Fantasy".to_string()],
            bookshelves: vec![],
            languages: vec!["en".to_string()],
            copyright: Some(false),
            formats: HashMap::from([
                (
                    "application/epub+zip".to_string(),
                    "https://example.test/book.epub".to_string(),
                ),
                (
                    "text/plain; charset=utf-8".to_string(),
                    "https://example.test/book.txt".to_string(),
                ),
            ]),
        };
        let dto = gutendex_book_to_dto(book.clone());
        assert_eq!(
            dto.download_url.as_deref(),
            Some("https://example.test/book.epub")
        );
        assert_eq!(dto.author.as_deref(), Some("Carroll, Lewis"));
        assert_eq!(
            preferred_gutenberg_text(&book.formats),
            Some(("https://example.test/book.txt".to_string(), false))
        );
    }
}
