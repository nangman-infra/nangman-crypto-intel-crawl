use super::*;

#[test]
fn parses_registry_shape() {
    let raw = r#"{
      "universe_assets": [{"asset":"BTC","reference_symbol_native":"BTCUSDT","rss_seed_status":"asset_specific_verified"}],
      "sources": [{
        "source_id":"news",
        "source_category":"news",
        "source_name":"News",
        "source_url":"https://example.com/rss.xml",
        "fetch_method":"rss",
        "adapter": null,
        "trust_tier":"T1",
        "cadence_tier":"medium",
        "language_hint":"en",
        "enabled":true,
        "top50_relevance_mode":"symbol_alias_match",
        "applies_to_assets":"all_major_50"
      }]
    }"#;

    let registry = serde_json::from_str::<SourceRegistry>(raw).unwrap();

    assert_eq!(registry.universe_assets[0].asset, "BTC");
    assert!(matches!(
        registry.sources[0].applies_to_assets,
        AppliesToAssets::All(_)
    ));
}

#[test]
fn rejects_enabled_source_for_non_universe_asset() {
    let raw = r#"{
      "universe_assets": [{"asset":"BTC","reference_symbol_native":"BTCUSDT"}],
      "sources": [{
        "source_id":"project_glm",
        "source_category":"project_notice",
        "source_name":"Golem",
        "source_url":"https://example.com/rss.xml",
        "fetch_method":"rss",
        "adapter": null,
        "trust_tier":"T0",
        "cadence_tier":"low",
        "language_hint":"en",
        "enabled":true,
        "top50_relevance_mode":"direct_asset",
        "applies_to_assets":["GLM"]
      }]
    }"#;

    let registry = serde_json::from_str::<SourceRegistry>(raw).unwrap();
    let error = registry.validate().unwrap_err().to_string();

    assert!(error.contains("non-universe asset GLM"));
}

#[test]
fn rejects_unknown_enabled_source_id() {
    let raw = r#"{
      "universe_assets": [{"asset":"BTC","reference_symbol_native":"BTCUSDT"}],
      "sources": [{
        "source_id":"news",
        "source_category":"news",
        "source_name":"News",
        "source_url":"https://example.com/rss.xml",
        "fetch_method":"rss",
        "adapter": null,
        "trust_tier":"T1",
        "cadence_tier":"medium",
        "language_hint":"en",
        "enabled":true,
        "top50_relevance_mode":"symbol_alias_match",
        "applies_to_assets":"all_major_50"
      }]
    }"#;

    let registry = serde_json::from_str::<SourceRegistry>(raw).unwrap();

    assert!(registry.require_enabled_source("missing").is_err());
}

#[test]
fn accepts_available_disabled_social_source_inventory() {
    let raw = r#"{
      "universe_assets": [{"asset":"BTC","reference_symbol_native":"BTCUSDT"}],
      "sources": [{
        "source_id":"community_reddit_bitcoin",
        "source_category":"social",
        "source_name":"Reddit Bitcoin RSS",
        "source_url":"https://www.reddit.com/r/Bitcoin/new/.rss",
        "fetch_method":"rss",
        "adapter": null,
        "trust_tier":"T2",
        "cadence_tier":"high",
        "language_hint":"en",
        "enabled":false,
        "source_state":"available_disabled",
        "activation_blocker":"community_noise_budget_required",
        "top50_relevance_mode":"symbol_alias_match",
        "applies_to_assets":"all_major_50"
      }]
    }"#;

    let registry = serde_json::from_str::<SourceRegistry>(raw).unwrap();

    assert!(registry.validate().is_ok());
}

#[test]
fn allows_explicit_manual_funding_history_backfill_source() {
    let raw = r#"{
      "universe_assets": [{"asset":"BTC","reference_symbol_native":"BTCUSDT"}],
      "sources": [{
        "source_id":"derivatives_binance_usdm_funding_rate_history_rest",
        "source_category":"funding",
        "source_name":"Binance USD-M Futures Funding Rate History",
        "source_url":"https://fapi.binance.com/fapi/v1/fundingRate",
        "fetch_method":"rest_api",
        "adapter":"binance_usdm_funding_rate_history",
        "trust_tier":"T1",
        "cadence_tier":"medium",
        "language_hint":"en",
        "enabled":false,
        "source_state":"available_disabled",
        "activation_blocker":"manual_backfill_requires_start_end_ms",
        "top50_relevance_mode":"symbol_alias_match",
        "applies_to_assets":"all_major_50"
      }]
    }"#;

    let registry = serde_json::from_str::<SourceRegistry>(raw).unwrap();

    assert!(registry.validate().is_ok());
    assert!(
        registry
            .require_enabled_source("derivatives_binance_usdm_funding_rate_history_rest")
            .is_ok()
    );
    assert_eq!(
        registry
            .enabled_sources(Some("derivatives_binance_usdm_funding_rate_history_rest"))
            .len(),
        1
    );
    assert!(registry.enabled_sources(None).is_empty());
}

#[test]
fn runtime_item_limit_caps_source_limit() {
    let mut source = Source {
        source_id: "derivatives_binance_usdm_funding_rate_history_rest".to_owned(),
        source_category: "funding".to_owned(),
        source_name: "Binance USD-M Futures Funding Rate History".to_owned(),
        source_url: "https://fapi.binance.com/fapi/v1/fundingRate".to_owned(),
        fetch_method: "rest_api".to_owned(),
        adapter: Some("binance_usdm_funding_rate_history".to_owned()),
        max_items_per_run: Some(1000),
        trust_tier: "T1".to_owned(),
        cadence_tier: "medium".to_owned(),
        language_hint: "en".to_owned(),
        enabled: false,
        source_state: Some("available_disabled".to_owned()),
        activation_blocker: Some("manual_backfill_requires_start_end_ms".to_owned()),
        top50_relevance_mode: "symbol_alias_match".to_owned(),
        applies_to_assets: AppliesToAssets::All("all_major_50".to_owned()),
    };

    assert_eq!(source.item_limit(3), 3);
    assert_eq!(source.item_limit(1500), 1000);

    source.max_items_per_run = None;
    assert_eq!(source.item_limit(3), 3);
}

#[test]
fn rejects_disabled_inventory_without_activation_blocker() {
    let raw = r#"{
      "universe_assets": [{"asset":"BTC","reference_symbol_native":"BTCUSDT"}],
      "sources": [{
        "source_id":"community_reddit_bitcoin",
        "source_category":"social",
        "source_name":"Reddit Bitcoin RSS",
        "source_url":"https://www.reddit.com/r/Bitcoin/new/.rss",
        "fetch_method":"rss",
        "adapter": null,
        "trust_tier":"T2",
        "cadence_tier":"high",
        "language_hint":"en",
        "enabled":false,
        "source_state":"available_disabled",
        "top50_relevance_mode":"symbol_alias_match",
        "applies_to_assets":"all_major_50"
      }]
    }"#;

    let registry = serde_json::from_str::<SourceRegistry>(raw).unwrap();
    let error = registry.validate().unwrap_err().to_string();

    assert!(error.contains("requires activation_blocker"));
}
