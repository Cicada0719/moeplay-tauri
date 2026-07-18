use ::scraper::{Html, Selector};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const GUTENDEX_API: &str = "https://gutendex.com";
const WIKISOURCE_API: &str = "https://zh.wikisource.org/w/api.php";
const MAX_CHAPTERS: usize = 300;
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
        "gutenberg" => search_gutenberg(query).await,
        "wikisource" => search_wikisource(query).await,
        "all" | "" => {
            let (gutenberg, wikisource) =
                tokio::join!(search_gutenberg(query), search_wikisource(query));
            let mut books = Vec::new();
            let mut errors = Vec::new();
            match gutenberg {
                Ok(mut items) => books.append(&mut items),
                Err(error) => errors.push(error),
            }
            match wikisource {
                Ok(mut items) => books.append(&mut items),
                Err(error) => errors.push(error),
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
        "gutenberg" => read_gutenberg(&book_id, &chapter_id).await,
        "wikisource" => read_wikisource(&book_id, &chapter_id).await,
        _ => Err("不支持的小说源".to_string()),
    }
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
    crate::http_client::build_reqwest_client(
        timeout_secs,
        crate::http_client::app_user_agent_with_context("novel-reader"),
    )
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
