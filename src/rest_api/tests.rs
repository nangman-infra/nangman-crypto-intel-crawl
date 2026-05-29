use super::binance_cms::{
    BinanceCmsArticle, BinanceCmsCatalog, binance_cms_article_item,
    binance_cms_article_metadata_body, collect_binance_text,
};
use super::derivatives::{
    BinanceFundingRate, LIVE_DERIVATIVES_SELECTION_ROTATION_MS, binance_funding_rate_history_item,
    binance_funding_rate_history_url, prioritized_derivatives_assets,
    prioritized_live_derivatives_assets, prioritized_live_derivatives_assets_for_seed, with_query,
};
use crate::registry::UniverseAsset;
use serde_json::json;

#[test]
fn builds_binance_cms_article_item() {
    let catalog = BinanceCmsCatalog {
        catalog_id: 48,
        catalog_name: "New Cryptocurrency Listing".to_owned(),
        articles: Vec::new(),
    };
    let article = BinanceCmsArticle {
        id: 1,
        code: "abc".to_owned(),
        title: "Binance lists TEST".to_owned(),
        release_date: 1778137169304,
    };

    let body =
        binance_cms_article_metadata_body("exchange_binance_listing_rest", &catalog, &article);
    let item = binance_cms_article_item(&article, body);

    assert_eq!(item.id.as_deref(), Some("abc"));
    assert_eq!(item.published_at.as_deref(), Some("1778137169304"));
    assert!(item.url.ends_with("/abc"));
    assert!(item.body.contains("exchange_binance_listing_rest"));
}

#[test]
fn extracts_binance_rich_text_body() {
    let raw = json!({
        "node": "root",
        "child": [
            {"node": "element", "child": [{"node": "text", "text": "Fellow Binancians,"}]},
            {"node": "text", "text": "Trading starts soon."}
        ]
    });
    let mut parts = Vec::new();

    collect_binance_text(&raw, &mut parts);

    assert_eq!(parts.join(" "), "Fellow Binancians, Trading starts soon.");
}

#[test]
fn appends_query_with_existing_params() {
    assert_eq!(
        with_query("https://example.com/path?a=1", &[("symbol", "BTCUSDT")]),
        "https://example.com/path?a=1&symbol=BTCUSDT"
    );
}

#[test]
fn builds_funding_rate_history_url() {
    assert_eq!(
        binance_funding_rate_history_url(
            "https://fapi.binance.com/fapi/v1/fundingRate",
            "BTCUSDT",
            1764892800000,
            1764979200000,
            "1000",
        ),
        "https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&startTime=1764892800000&endTime=1764979200000&limit=1000"
    );
}

#[test]
fn funding_history_item_carries_backfill_metadata() {
    let record = BinanceFundingRate {
        symbol: "BTCUSDT".to_owned(),
        funding_rate: "0.00010000".to_owned(),
        funding_time: 1764892800000,
        mark_price: "90000.0".to_owned(),
    };

    let item = binance_funding_rate_history_item(
        &record,
        "https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT",
        1764892800000,
        1764979200000,
    );

    assert_eq!(
        item.historical_source_depth.as_deref(),
        Some("range_queryable")
    );
    assert_eq!(item.backfill_window_start_ms, Some(1764892800000));
    assert_eq!(item.backfill_window_end_ms, Some(1764979200000));
    assert_eq!(item.source_time_range_verified, Some(true));
    assert!(item.body.contains("\"source_time_range_verified\":true"));
}

#[test]
fn derivatives_assets_prioritize_verified_asset_specific_symbols() {
    let assets = vec![
        asset("USDC", "global_news_only"),
        asset("BTC", "asset_specific_verified"),
        asset("ETH", "asset_specific_verified"),
        asset("TST", "global_news_only"),
    ];

    let ranked = prioritized_derivatives_assets(&assets)
        .into_iter()
        .map(|asset| asset.asset.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ranked, vec!["BTC", "ETH", "USDC", "TST"]);
}

#[test]
fn live_derivatives_assets_interleave_verified_and_global_only_symbols() {
    let assets = vec![
        asset("USDC", "asset_specific_verified"),
        asset("BTC", "asset_specific_verified"),
        asset("ETH", "asset_specific_verified"),
        asset("SOL", "asset_specific_verified"),
        asset("TST", "global_news_only"),
        asset("DOGS", "global_news_only"),
        asset("CHIP", "global_news_only"),
    ];

    let ranked = prioritized_live_derivatives_assets_for_seed(&assets, 0)
        .into_iter()
        .map(|asset| asset.asset.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        ranked,
        vec!["USDC", "TST", "BTC", "DOGS", "ETH", "CHIP", "SOL"]
    );
}

#[test]
fn live_derivatives_assets_rotate_between_selection_windows() {
    let assets = vec![
        asset("USDC", "asset_specific_verified"),
        asset("BTC", "asset_specific_verified"),
        asset("ETH", "asset_specific_verified"),
        asset("SOL", "asset_specific_verified"),
        asset("TON", "asset_specific_verified"),
        asset("ZEC", "asset_specific_verified"),
        asset("TST", "global_news_only"),
        asset("DOGS", "global_news_only"),
        asset("CHIP", "global_news_only"),
        asset("PEPE", "global_news_only"),
    ];

    let first_window = prioritized_live_derivatives_assets(&assets, "funding", 0)
        .into_iter()
        .take(6)
        .map(|asset| asset.asset.as_str())
        .collect::<Vec<_>>();
    let second_window = prioritized_live_derivatives_assets(
        &assets,
        "funding",
        LIVE_DERIVATIVES_SELECTION_ROTATION_MS,
    )
    .into_iter()
    .take(6)
    .map(|asset| asset.asset.as_str())
    .collect::<Vec<_>>();

    assert_ne!(first_window, second_window);
    assert_eq!(first_window.len(), 6);
    assert_eq!(second_window.len(), 6);
}

#[test]
fn live_derivatives_sources_use_different_asset_offsets() {
    let assets = vec![
        asset("USDC", "asset_specific_verified"),
        asset("BTC", "asset_specific_verified"),
        asset("ETH", "asset_specific_verified"),
        asset("SOL", "asset_specific_verified"),
        asset("TON", "asset_specific_verified"),
        asset("ZEC", "asset_specific_verified"),
        asset("TST", "global_news_only"),
        asset("DOGS", "global_news_only"),
        asset("CHIP", "global_news_only"),
        asset("PEPE", "global_news_only"),
    ];

    let funding_window = prioritized_live_derivatives_assets(
        &assets,
        "derivatives_binance_usdm_funding_rate_rest",
        0,
    )
    .into_iter()
    .take(6)
    .map(|asset| asset.asset.as_str())
    .collect::<Vec<_>>();
    let open_interest_window = prioritized_live_derivatives_assets(
        &assets,
        "derivatives_binance_usdm_open_interest_rest",
        0,
    )
    .into_iter()
    .take(6)
    .map(|asset| asset.asset.as_str())
    .collect::<Vec<_>>();

    assert_ne!(funding_window, open_interest_window);
}

fn asset(asset: &str, rss_seed_status: &str) -> UniverseAsset {
    UniverseAsset {
        asset: asset.to_owned(),
        reference_symbol_native: format!("{asset}USDT"),
        rss_seed_status: Some(rss_seed_status.to_owned()),
    }
}
