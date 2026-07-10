use moeplay_lib::anime::{fetch_roads, fetch_rule_by_name, search_anime};

#[tokio::test]
#[ignore = "requires live GitHub and anime source access"]
async fn live_anime_sources_can_search_and_load_episodes() {
    let candidates = ["TvTFun", "aafun", "gugu3", "MXdm", "AGE"];
    let mut diagnostics = Vec::new();
    for name in candidates {
        let rule = match tokio::time::timeout(
            std::time::Duration::from_secs(20),
            fetch_rule_by_name(name),
        )
        .await
        {
            Ok(Ok(rule)) => rule,
            Ok(Err(error)) => {
                diagnostics.push(format!("{name}: rule {error}"));
                continue;
            }
            Err(_) => {
                diagnostics.push(format!("{name}: rule timeout"));
                continue;
            }
        };
        let items = match tokio::time::timeout(
            std::time::Duration::from_secs(20),
            search_anime(&rule, "葬送的芙莉莲"),
        )
        .await
        {
            Ok(Ok(items)) if !items.is_empty() => items,
            Ok(Ok(_)) => {
                diagnostics.push(format!("{name}: no search result"));
                continue;
            }
            Ok(Err(error)) => {
                diagnostics.push(format!("{name}: search {error}"));
                continue;
            }
            Err(_) => {
                diagnostics.push(format!("{name}: search timeout"));
                continue;
            }
        };
        for item in items.into_iter().take(3) {
            match tokio::time::timeout(
                std::time::Duration::from_secs(25),
                fetch_roads(&rule, &item.url),
            )
            .await
            {
                Ok(Ok(roads)) if roads.iter().any(|road| !road.episodes.is_empty()) => return,
                Ok(Ok(_)) => diagnostics.push(format!("{name}: empty roads for {}", item.name)),
                Ok(Err(error)) => diagnostics.push(format!("{name}: roads {error}")),
                Err(_) => diagnostics.push(format!("{name}: roads timeout")),
            }
        }
    }
    panic!(
        "no live anime source completed search -> roads: {}",
        diagnostics.join(" | ")
    );
}
