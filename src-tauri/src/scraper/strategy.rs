// 萌游 MoeGame · 刮削策略与路由系统（M3）
//
// 刮削策略：
//   - Full:        全量搜索所有源 + AI 增强 + 合并
//   - Incremental: 只取已有结果中缺失的字段（patch missing）
//   - PatchMissing: 检测当前游戏缺失字段，仅刮削补充
//   - RetryFailed: 重新刮削之前失败的游戏
//
// 刮削路由：
//   - source="steam"  → 优先 Steam 元数据
//   - source="local"  → VNDB → Bangumi → DLsite 链式回退
//   - source="dlsite" → DLsite + VNDB 互补
//   - 无 source       → 全局搜索

use serde::{Deserialize, Serialize};

/// 刮削策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScrapeStrategy {
    /// 全量：所有源 + AI + 合并
    Full,
    /// 增量：只跑上次之后新增/变动的
    Incremental,
    /// 补缺：检测缺失字段，仅刮这些
    PatchMissing,
    /// 重试失败：重刮之前 failed 的游戏
    RetryFailed,
}

impl std::fmt::Display for ScrapeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Incremental => write!(f, "incremental"),
            Self::PatchMissing => write!(f, "patch_missing"),
            Self::RetryFailed => write!(f, "retry_failed"),
        }
    }
}

/// 刮削路由：根据游戏来源选择最优搜索路径。
#[derive(Debug, Clone)]
pub struct ScrapeRouter;

/// 路由结果：按优先级排列的源列表。
#[derive(Debug, Clone)]
pub struct RoutePlan {
    /// 优先搜索的源（按顺序）
    pub primary: Vec<String>,
    /// 备选搜索的源
    pub fallback: Vec<String>,
    /// 是否启用 AI 增强
    pub with_ai: bool,
    /// 是否下载截图
    pub fetch_screenshots: bool,
}

impl ScrapeRouter {
    /// 根据游戏来源和已有元数据制定路由计划。
    pub fn plan(source: Option<&str>, has_cover: bool, has_desc: bool) -> RoutePlan {
        match source {
            Some("steam") => RoutePlan {
                primary: vec!["steam".into()],
                fallback: if has_desc && has_cover {
                    vec![]
                } else {
                    vec!["vndb".into(), "bangumi".into()]
                },
                with_ai: !has_desc,
                fetch_screenshots: !has_cover,
            },
            Some("dlsite") => RoutePlan {
                primary: vec!["dlsite".into()],
                fallback: vec!["vndb".into(), "bangumi".into()],
                with_ai: !has_desc,
                fetch_screenshots: !has_cover,
            },
            Some("local") | Some("emulator") | None => RoutePlan {
                primary: vec!["vndb".into(), "bangumi".into()],
                fallback: vec![
                    "dlsite".into(),
                    "ymgal".into(),
                    "kungal".into(),
                    "steam".into(),
                    "erogamescape".into(),
                ],
                with_ai: true,
                fetch_screenshots: true,
            },
            Some(other) => {
                // 尝试将该源作为 primary
                RoutePlan {
                    primary: vec![other.to_string()],
                    fallback: vec!["vndb".into(), "bangumi".into(), "dlsite".into()],
                    with_ai: true,
                    fetch_screenshots: true,
                }
            }
        }
    }

    /// 制定"补缺"路由：根据缺失字段推荐源。
    pub fn plan_patch(
        missing_cover: bool,
        missing_desc: bool,
        missing_tags: bool,
        missing_rating: bool,
    ) -> RoutePlan {
        let mut primary = vec![];
        if missing_cover {
            primary.push("steam".to_string());
            primary.push("vndb".to_string());
        }
        if missing_desc {
            primary.push("vndb".to_string());
            primary.push("bangumi".to_string());
        }
        if missing_tags {
            primary.push("dlsite".to_string());
            primary.push("vndb".to_string());
        }
        if missing_rating {
            primary.push("vndb".to_string());
            primary.push("erogamescape".to_string());
        }
        // 去重
        let mut seen = std::collections::HashSet::new();
        primary.retain(|s| seen.insert(s.clone()));

        RoutePlan {
            primary,
            fallback: vec![],
            with_ai: missing_desc,
            fetch_screenshots: missing_cover,
        }
    }
}

/// 刮削请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeRequest {
    pub query: String,
    pub strategy: ScrapeStrategy,
    pub source_hint: Option<String>,
    pub vndb_enabled: bool,
    pub bangumi_enabled: bool,
    pub dlsite_enabled: bool,
    pub getchu_enabled: bool,
    pub touchgal_enabled: bool,
    pub erogamescape_enabled: bool,
    pub ymgal_enabled: bool,
    pub kungal_enabled: bool,
    pub steam_enabled: bool,
    pub pcgw_enabled: bool,
    pub with_ai: bool,
    pub with_merge: bool,
    pub fetch_screenshots: bool,
    pub max_results: usize,
}

impl Default for ScrapeRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            strategy: ScrapeStrategy::Full,
            source_hint: None,
            vndb_enabled: true,
            bangumi_enabled: true,
            dlsite_enabled: true,
            getchu_enabled: false,
            touchgal_enabled: true,
            erogamescape_enabled: true,
            ymgal_enabled: true,
            kungal_enabled: true,
            steam_enabled: true,
            pcgw_enabled: true,
            with_ai: true,
            with_merge: true,
            fetch_screenshots: true,
            max_results: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_steam_source() {
        let plan = ScrapeRouter::plan(Some("steam"), false, false);
        assert_eq!(plan.primary, vec!["steam"]);
        assert!(!plan.fallback.is_empty()); // 无封面无描述→fallback
        assert!(plan.with_ai);
    }

    #[test]
    fn test_route_local_source() {
        let plan = ScrapeRouter::plan(Some("local"), false, false);
        assert_eq!(plan.primary, vec!["vndb", "bangumi"]);
        assert!(plan.fallback.len() >= 3);
        assert!(plan.with_ai);
    }

    #[test]
    fn test_route_steam_with_full_data() {
        let plan = ScrapeRouter::plan(Some("steam"), true, true);
        assert_eq!(plan.primary, vec!["steam"]);
        assert!(plan.fallback.is_empty()); // 数据齐全→不fallback
        assert!(!plan.with_ai);
    }

    #[test]
    fn test_patch_plan_all_missing() {
        let plan = ScrapeRouter::plan_patch(true, true, true, true);
        assert!(plan.primary.contains(&"steam".to_string()));
        assert!(plan.primary.contains(&"vndb".to_string()));
        assert!(plan.primary.contains(&"dlsite".to_string()));
    }

    #[test]
    fn test_strategy_display() {
        assert_eq!(ScrapeStrategy::Full.to_string(), "full");
        assert_eq!(ScrapeStrategy::Incremental.to_string(), "incremental");
        assert_eq!(ScrapeStrategy::PatchMissing.to_string(), "patch_missing");
    }
}
