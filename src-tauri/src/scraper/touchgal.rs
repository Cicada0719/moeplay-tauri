//! TouchGAL 源 —— 已与 Kungal 合并为同一站点。
//!
//! 背景：touchgal.io / touchgal.ink 与 kungal.com 是同一项目（KUN Galgame Patch）的镜像。
//! 实测 touchgal.ink 被 Cloudflare 5 秒盾拦截（服务端 curl/reqwest 拿到的是 "Just a moment..."
//! 挑战页，不可用）；而 kungal.com 的 JSON API 服务端可直连。因此 search_all 里的 "touchgal"
//! 源统一委托给 kungal 现行 API，避免维护两份会随站点改版而失效的解析逻辑。
//!
//! 现行 kungal API（已验证）：
//!   - 搜索: GET /api/search?keywords=&type=galgame&page=1&limit=N  → {code,data:{items:[...]}}
//!   - 详情: GET /api/galgame/{id}                                  → {code,data:{...富字段...}}
//!   - 下载: GET /api/galgame/{id}/resource                          → 需登录（401 用户登录失效）

use crate::models::ScrapeResult;

/// 简易搜索接口（search_all 使用）。委托给 kungal 现行实现。
pub async fn search_simple(query: &str) -> Result<Vec<ScrapeResult>, String> {
    crate::scraper::kungal::search_simple(query).await
}
