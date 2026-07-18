//! Getchu Galgame metadata source.
//!
//! Getchu exposes public search and product pages rather than a documented API.
//! The pages are EUC-JP encoded and can be slow, so this provider is opt-in.

use ::scraper::{Html, Selector};
use regex::Regex;
use reqwest::header::{ACCEPT, COOKIE, REFERER};

use super::error::ScrapeError;
use super::utils;
use crate::models::{ScrapeDetail, ScrapeResult};

const SEARCH_URL: &str = "https://www.getchu.com/php/search.phtml?search_title={keyword}&list_count=30&sort=title&sort2=down&genre=pc_soft&list_type=list&search=search";
const PRODUCT_URL: &str = "https://www.getchu.com/soft.phtml?id={id}";
const GETCHU_ORIGIN: &str = "https://www.getchu.com";

#[derive(Debug, Clone, Default)]
struct GetchuGameInfo {
    id: String,
    title: String,
    developer: Option<String>,
    description: Option<String>,
    cover: Option<String>,
    release_date: Option<String>,
    tags: Vec<String>,
    screenshots: Vec<String>,
    age_rating: Option<String>,
}

pub async fn search(query: &str) -> Result<Vec<ScrapeResult>, ScrapeError> {
    let query = query.trim();
    if query.is_empty() {
        return Err(ScrapeError::Config("搜索关键词不能为空".into()));
    }

    let keyword = encode_euc_jp_component(query);
    let url = SEARCH_URL.replace("{keyword}", &keyword);
    let html = fetch_getchu_page(&url)
        .await
        .map_err(|error| ScrapeError::Network(format!("Getchu 搜索请求失败: {error}")))?;
    let results = parse_search_page(&html, query);
    if results.is_empty() {
        Err(ScrapeError::NotFound)
    } else {
        Ok(results)
    }
}

pub async fn get_product(product_id: &str) -> Result<ScrapeResult, ScrapeError> {
    let product_id = product_id.trim();
    if product_id.is_empty() || !product_id.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(ScrapeError::Config("Getchu 商品 ID 无效".into()));
    }

    let url = PRODUCT_URL.replace("{id}", product_id);
    let html = fetch_getchu_page(&url)
        .await
        .map_err(|error| ScrapeError::Network(format!("Getchu 详情请求失败: {error}")))?;
    parse_product_page(&html, product_id)
        .ok_or_else(|| ScrapeError::Parse("无法解析 Getchu 商品详情".into()))
}

pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    search(query).await.map_err(|error| error.to_string())
}

async fn fetch_getchu_page(url: &str) -> Result<String, ScrapeError> {
    let response = utils::build_client_ja()?
        .get(url)
        .header(
            ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header(REFERER, "https://www.getchu.com/top.html")
        .header(COOKIE, "getchu_adalt_flag=getchu.com")
        .send()
        .await
        .map_err(|error| ScrapeError::Network(error.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(ScrapeError::Api {
            status: status.as_u16(),
            body: String::new(),
        });
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| ScrapeError::Network(error.to_string()))?;
    Ok(decode_euc_jp(&bytes))
}

fn encode_euc_jp_component(input: &str) -> String {
    let (encoded, _, _) = encoding_rs::EUC_JP.encode(input);
    let mut output = String::with_capacity(encoded.len() * 3);
    for byte in encoded.iter().copied() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(byte as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}

fn decode_euc_jp(bytes: &[u8]) -> String {
    let (decoded, _, had_errors) = encoding_rs::EUC_JP.decode(bytes);
    if had_errors {
        if let Ok(utf8) = std::str::from_utf8(bytes) {
            return utf8.to_string();
        }
    }
    decoded.into_owned()
}

fn parse_search_page(html: &str, query: &str) -> Vec<ScrapeResult> {
    let document = Html::parse_document(html);
    let block_selector = selector("div#detail_block");
    let title_selector = selector("a[href*='soft.phtml?id=']");
    let brand_selector = selector("a[href*='search_brand_id=']");
    let id_re = Regex::new(r"soft\.phtml\?id=(\d+)").expect("valid Getchu id regex");
    let date_re = Regex::new(r"((?:19|20)\d{2})/(\d{2})/(\d{2})").expect("valid Getchu date regex");

    let mut ranked = Vec::new();
    for block in document.select(&block_selector) {
        let Some(anchor) = block.select(&title_selector).find(|anchor| {
            anchor
                .value()
                .attr("href")
                .is_some_and(|href| id_re.is_match(href))
        }) else {
            continue;
        };
        let Some(id) = anchor
            .value()
            .attr("href")
            .and_then(|href| id_re.captures(href))
            .and_then(|captures| captures.get(1))
            .map(|value| value.as_str().to_string())
        else {
            continue;
        };

        let title = element_text(anchor);
        if title.is_empty() {
            continue;
        }
        let developer = block
            .select(&brand_selector)
            .map(element_text)
            .find(|value| !value.is_empty());
        let block_text = block.text().collect::<Vec<_>>().join(" ");
        let release_date = extract_date(&block_text, &date_re);
        let info = GetchuGameInfo {
            id: id.clone(),
            title: title.clone(),
            developer,
            cover: Some(format!(
                "https://www.getchu.com/brandnew/{id}/c{id}package.jpg"
            )),
            release_date,
            ..GetchuGameInfo::default()
        };
        ranked.push((utils::confidence(query, &title), info));
    }

    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    ranked
        .into_iter()
        .take(8)
        .map(|(_, info)| info_to_result(&info))
        .collect()
}

fn parse_product_page(html: &str, product_id: &str) -> Option<ScrapeResult> {
    let document = Html::parse_document(html);
    let title_selector = selector("h2#soft-title");
    let og_title_selector = selector("meta[property='og:title']");
    let og_description_selector = selector("meta[property='og:description']");
    let og_image_selector = selector("meta[property='og:image']");
    let brand_selector = selector("a#brandsite");
    let row_selector = selector("tr");
    let anchor_selector = selector("a");
    let story_selector = selector("h3.tabletitle_1 + div span");
    let date_re = Regex::new(r"((?:19|20)\d{2})/(\d{2})/(\d{2})").ok()?;
    let sample_re = Regex::new(&format!(
        r"/brandnew/{}/c{}sample\d+\.jpg$",
        regex::escape(product_id),
        regex::escape(product_id)
    ))
    .ok()?;

    let title = document
        .select(&title_selector)
        .map(element_text)
        .find(|value| !value.is_empty())
        .or_else(|| {
            document
                .select(&og_title_selector)
                .find_map(|meta| meta.value().attr("content"))
                .map(|value| {
                    value
                        .split(" | ")
                        .next()
                        .unwrap_or(value)
                        .trim()
                        .to_string()
                })
        })?;
    if title.is_empty() || title.starts_with("404 ") {
        return None;
    }

    let developer = document
        .select(&brand_selector)
        .map(element_text)
        .find(|value| !value.is_empty());
    let mut release_date = None;
    let mut tags = Vec::new();
    for row in document.select(&row_selector) {
        let text = row.text().collect::<Vec<_>>().join(" ");
        if release_date.is_none() && text.contains("発売日") {
            release_date = extract_date(&text, &date_re);
        }
        if text.contains("ジャンル") {
            tags.extend(
                row.select(&anchor_selector)
                    .map(element_text)
                    .filter(|value| !value.is_empty() && value.chars().count() <= 30),
            );
        }
    }
    tags.sort();
    tags.dedup();
    tags.truncate(12);

    let meta_description = document
        .select(&og_description_selector)
        .find_map(|meta| meta.value().attr("content"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let story = document
        .select(&story_selector)
        .map(element_text)
        .find(|value| !value.is_empty());
    let description = story
        .or(meta_description)
        .map(|value| utils::truncate(&value, 500));

    let cover = document
        .select(&og_image_selector)
        .find_map(|meta| meta.value().attr("content"))
        .map(absolute_getchu_url)
        .or_else(|| {
            Some(format!(
                "https://www.getchu.com/brandnew/{product_id}/c{product_id}package.jpg"
            ))
        });

    let mut screenshots = document
        .select(&anchor_selector)
        .filter_map(|anchor| anchor.value().attr("href"))
        .filter(|href| sample_re.is_match(href))
        .map(absolute_getchu_url)
        .collect::<Vec<_>>();
    screenshots.sort();
    screenshots.dedup();
    screenshots.truncate(12);

    let page_text = document.root_element().text().collect::<Vec<_>>().join(" ");
    let age_rating = ["全年齢", "18歳以上", "18禁", "15歳以上"]
        .into_iter()
        .find(|rating| page_text.contains(rating))
        .map(str::to_string);

    let info = GetchuGameInfo {
        id: product_id.to_string(),
        title,
        developer,
        description,
        cover,
        release_date,
        tags,
        screenshots,
        age_rating,
    };
    Some(info_to_result(&info))
}

fn info_to_result(info: &GetchuGameInfo) -> ScrapeResult {
    let release_year = info
        .release_date
        .as_deref()
        .and_then(|date| date.get(0..4))
        .and_then(|year| year.parse::<u32>().ok());
    let detail = ScrapeDetail {
        developer: info.developer.clone(),
        genres: info.tags.clone(),
        homepage: Some(PRODUCT_URL.replace("{id}", &info.id)),
        screenshots: info.screenshots.clone(),
        age_rating: info.age_rating.clone(),
        release_date: info.release_date.clone(),
        ..ScrapeDetail::default()
    };

    ScrapeResult {
        title: info.title.clone(),
        description: info.description.clone(),
        cover: info.cover.clone(),
        background: info.screenshots.first().cloned(),
        tags: info.tags.clone(),
        rating: None,
        release_year,
        source: "getchu".to_string(),
        source_id: info.id.clone(),
        detail: Some(detail),
    }
}

fn extract_date(text: &str, date_re: &Regex) -> Option<String> {
    let captures = date_re.captures(text)?;
    Some(format!(
        "{}-{}-{}",
        captures.get(1)?.as_str(),
        captures.get(2)?.as_str(),
        captures.get(3)?.as_str()
    ))
}

fn absolute_getchu_url(value: &str) -> String {
    if value.starts_with("https://") || value.starts_with("http://") {
        value.to_string()
    } else if value.starts_with("//") {
        format!("https:{value}")
    } else if value.starts_with('/') {
        format!("{GETCHU_ORIGIN}{value}")
    } else {
        format!("{GETCHU_ORIGIN}/{value}")
    }
}

fn element_text(element: ::scraper::ElementRef<'_>) -> String {
    utils::clean_html(&element.text().collect::<Vec<_>>().join(" "))
}

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("valid Getchu selector")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_decodes_euc_jp() {
        let encoded = encode_euc_jp_component("サクラ");
        assert!(encoded.starts_with('%'));
        assert!(!encoded.contains("サクラ"));

        let (bytes, _, _) = encoding_rs::EUC_JP.encode("発売日");
        assert_eq!(decode_euc_jp(&bytes), "発売日");
    }

    #[test]
    fn parses_multiple_search_results() {
        let html = r#"
          <div id="detail_block">
            <a href="../soft.phtml?id=681794" class="blueb">CLANNAD メモリアルエディション</a>
            <p><a href="/php/search.phtml?search_brand_id=78">Key</a> 発売日：2010/05/28</p>
          </div>
          <div id="detail_block">
            <a href="../soft.phtml?id=502245" class="blueb">CLANNAD FULL VOICE</a>
          </div>
        "#;
        let results = parse_search_page(html, "CLANNAD");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].source, "getchu");
        assert_eq!(
            results[0].detail.as_ref().unwrap().developer.as_deref(),
            Some("Key")
        );
        assert_eq!(results[0].release_year, Some(2010));
        assert!(results[0].cover.as_deref().unwrap().contains("681794"));
    }

    #[test]
    fn parses_product_metadata_and_samples() {
        let html = r#"
          <html><head>
            <meta property="og:description" content="作品紹介です。">
            <meta property="og:image" content="/brandnew/681794/c681794package.jpg">
          </head><body>
            <h2 id="soft-title">CLANNAD メモリアルエディション</h2>
            <a id="brandsite" href="https://key.example/">Key</a>
            <table>
              <tr><th>発売日</th><td>2010/05/28</td></tr>
              <tr><th>ジャンル</th><td><a>恋愛</a><a>学園</a></td></tr>
            </table>
            <p>全年齢</p>
            <a href="/brandnew/681794/c681794sample1.jpg">sample</a>
          </body></html>
        "#;
        let result = parse_product_page(html, "681794").unwrap();
        assert_eq!(result.title, "CLANNAD メモリアルエディション");
        assert_eq!(result.release_year, Some(2010));
        assert_eq!(result.tags, vec!["学園", "恋愛"]);
        let detail = result.detail.unwrap();
        assert_eq!(detail.age_rating.as_deref(), Some("全年齢"));
        assert_eq!(detail.screenshots.len(), 1);
        assert_eq!(detail.release_date.as_deref(), Some("2010-05-28"));
    }
}
