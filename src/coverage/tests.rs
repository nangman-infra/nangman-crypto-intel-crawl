use super::*;
use crate::coverage::report::AvailableDisabledSourceDetail;
use crate::registry::{AppliesToAssets, Source, SourceRegistry, UniverseAsset};

#[test]
fn reports_asset_specific_and_global_coverage() {
    let registry = SourceRegistry {
        universe_assets: vec![
            UniverseAsset {
                asset: "BTC".to_owned(),
                reference_symbol_native: "BTCUSDT".to_owned(),
                rss_seed_status: Some("asset_specific_verified".to_owned()),
            },
            UniverseAsset {
                asset: "DOGE".to_owned(),
                reference_symbol_native: "DOGEUSDT".to_owned(),
                rss_seed_status: Some("global_news_only".to_owned()),
            },
        ],
        sources: vec![
            global_news(),
            direct_project("BTC", true),
            direct_project("DOGE", false),
        ],
    };

    let report = build_source_coverage_report(&registry, 10);

    assert_eq!(report[0].coverage_status, "asset_specific_enabled");
    assert_eq!(
        report[1].coverage_status,
        "asset_specific_available_disabled"
    );
    assert!(
        report[1]
            .quality_gaps
            .contains(&"missing_enabled_asset_specific_source".to_owned())
    );
    assert_eq!(
        report[1].available_disabled_direct_sources,
        vec![AvailableDisabledSourceDetail {
            source_id: "project_DOGE".to_owned(),
            source_name: "project_DOGE".to_owned(),
            source_url: "https://example.com/feed.xml".to_owned(),
            source_category: "project_notice".to_owned(),
            fetch_method: "rss".to_owned(),
            trust_tier: "T1".to_owned(),
            activation_blocker: Some("not_active".to_owned()),
        }]
    );
}

fn global_news() -> Source {
    source(
        "news",
        "news",
        true,
        AppliesToAssets::All("all_major_50".to_owned()),
    )
}

fn direct_project(asset: &str, enabled: bool) -> Source {
    source(
        &format!("project_{asset}"),
        "project_notice",
        enabled,
        AppliesToAssets::List(vec![asset.to_owned()]),
    )
}

fn source(
    source_id: &str,
    source_category: &str,
    enabled: bool,
    applies_to_assets: AppliesToAssets,
) -> Source {
    Source {
        source_id: source_id.to_owned(),
        source_category: source_category.to_owned(),
        source_name: source_id.to_owned(),
        source_url: "https://example.com/feed.xml".to_owned(),
        fetch_method: "rss".to_owned(),
        adapter: None,
        max_items_per_run: None,
        trust_tier: "T1".to_owned(),
        cadence_tier: "medium".to_owned(),
        language_hint: "en".to_owned(),
        enabled,
        source_state: (!enabled).then(|| "available_disabled".to_owned()),
        activation_blocker: (!enabled).then(|| "not_active".to_owned()),
        top50_relevance_mode: "symbol_alias_match".to_owned(),
        applies_to_assets,
    }
}
