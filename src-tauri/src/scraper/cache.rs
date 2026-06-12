// 萌游 MoeGame · 刮削搜索缓存（M6 性能优化）
//
// 内存 HashMap 缓存：key = "query|source", value = (results, inserted_at)
// TTL 1 小时。再次搜索同 query 时先返回缓存，后台异步刷新。

use crate::models::ScrapeResult;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// 缓存条目
struct CacheEntry {
    results: Vec<ScrapeResult>,
    inserted_at: Instant,
}

/// 搜索缓存（线程安全）
pub struct ScrapeCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
    ttl_secs: u64,
}

impl ScrapeCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl_secs,
        }
    }

    fn cache_key(query: &str, source: &str) -> String {
        format!("{}|{}", query.to_lowercase().trim(), source)
    }

    /// 尝试获取缓存。返回 Some 如果命中且未过期。
    pub fn get(&self, query: &str, source: &str) -> Option<Vec<ScrapeResult>> {
        let map = self.entries.lock().unwrap();
        let key = Self::cache_key(query, source);
        if let Some(entry) = map.get(&key) {
            if entry.inserted_at.elapsed().as_secs() < self.ttl_secs {
                return Some(entry.results.clone());
            }
        }
        None
    }

    /// 写入缓存。
    pub fn set(&self, query: &str, source: &str, results: Vec<ScrapeResult>) {
        let mut map = self.entries.lock().unwrap();
        let key = Self::cache_key(query, source);
        map.insert(
            key,
            CacheEntry {
                results,
                inserted_at: Instant::now(),
            },
        );
    }

    /// 清理过期条目。
    pub fn prune(&self) -> usize {
        let mut map = self.entries.lock().unwrap();
        let before = map.len();
        map.retain(|_, v| v.inserted_at.elapsed().as_secs() < self.ttl_secs);
        before - map.len()
    }

    /// 缓存条目数。
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// 缓存是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 清空缓存。
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

impl Default for ScrapeCache {
    fn default() -> Self {
        Self::new(3600) // 1 hour
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(title: &str, source: &str) -> ScrapeResult {
        ScrapeResult {
            title: title.to_string(),
            description: None,
            cover: None,
            background: None,
            tags: vec![],
            rating: None,
            release_year: None,
            source: source.to_string(),
            source_id: "1".to_string(),
            detail: None,
        }
    }

    #[test]
    fn test_cache_hit_and_expire() {
        let cache = ScrapeCache::new(1); // 1 second TTL
        assert!(cache.get("clannad", "vndb").is_none());

        cache.set("clannad", "vndb", vec![make_result("CLANNAD", "vndb")]);
        assert_eq!(cache.get("clannad", "vndb").unwrap().len(), 1);

        std::thread::sleep(std::time::Duration::from_secs(2));
        assert!(cache.get("clannad", "vndb").is_none());
    }

    #[test]
    fn test_cache_query_normalization() {
        let cache = ScrapeCache::new(60);
        cache.set("CLANNAD", "vndb", vec![make_result("CLANNAD", "vndb")]);
        assert!(cache.get("ClannAd", "vndb").is_some()); // case-insensitive
        assert!(cache.get("clannad ", "vndb").is_some()); // trimmed
    }

    #[test]
    fn test_cache_different_sources_isolated() {
        let cache = ScrapeCache::new(60);
        cache.set("test", "vndb", vec![make_result("V", "vndb")]);
        cache.set("test", "bangumi", vec![make_result("B", "bangumi")]);
        assert_eq!(cache.get("test", "vndb").unwrap()[0].source, "vndb");
        assert_eq!(cache.get("test", "bangumi").unwrap()[0].source, "bangumi");
    }
}
