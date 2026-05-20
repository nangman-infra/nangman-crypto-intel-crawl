use serde::Deserialize;
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SourceRegistry {
    pub(crate) universe_assets: Vec<UniverseAsset>,
    pub(crate) sources: Vec<Source>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UniverseAsset {
    pub(crate) asset: String,
    pub(crate) reference_symbol_native: String,
    #[serde(default)]
    pub(crate) rss_seed_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Source {
    pub(crate) source_id: String,
    pub(crate) source_category: String,
    pub(crate) source_name: String,
    pub(crate) source_url: String,
    pub(crate) fetch_method: String,
    pub(crate) adapter: Option<String>,
    pub(crate) max_items_per_run: Option<usize>,
    pub(crate) trust_tier: String,
    pub(crate) cadence_tier: String,
    pub(crate) language_hint: String,
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) source_state: Option<String>,
    #[serde(default)]
    pub(crate) activation_blocker: Option<String>,
    pub(crate) top50_relevance_mode: String,
    pub(crate) applies_to_assets: AppliesToAssets,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum AppliesToAssets {
    All(String),
    List(Vec<String>),
}

impl SourceRegistry {
    pub(crate) async fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let raw = tokio::fs::read_to_string(path).await?;
        let registry = serde_json::from_str::<Self>(&raw)?;
        registry.validate()?;
        Ok(registry)
    }

    pub(crate) fn enabled_sources(&self, source_id: Option<&str>) -> Vec<&Source> {
        self.sources
            .iter()
            .filter(|source| {
                source.enabled
                    || source_id.is_some_and(|id| {
                        source.source_id == id && source.is_manual_backfill_source()
                    })
            })
            .filter(|source| source_id.is_none_or(|id| source.source_id == id))
            .collect()
    }

    pub(crate) fn require_enabled_source(&self, source_id: &str) -> Result<(), Box<dyn Error>> {
        if self
            .sources
            .iter()
            .any(|source| source.enabled && source.source_id == source_id)
        {
            return Ok(());
        }
        if self
            .sources
            .iter()
            .any(|source| source.source_id == source_id && source.is_manual_backfill_source())
        {
            return Ok(());
        }
        Err(format!("enabled source_id not found: {source_id}").into())
    }

    fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.universe_assets.is_empty() {
            return Err("source registry universe_assets must not be empty".into());
        }
        if self.sources.is_empty() {
            return Err("source registry sources must not be empty".into());
        }
        let mut universe_assets = HashSet::new();
        let mut reference_symbols = HashSet::new();
        for asset in &self.universe_assets {
            if asset.asset.trim().is_empty() {
                return Err("source registry asset must not be empty".into());
            }
            if asset.reference_symbol_native.trim().is_empty() {
                return Err(format!(
                    "source registry asset {} reference_symbol_native must not be empty",
                    asset.asset
                )
                .into());
            }
            if !universe_assets.insert(asset.asset.as_str()) {
                return Err(format!("duplicate universe asset {}", asset.asset).into());
            }
            if !reference_symbols.insert(asset.reference_symbol_native.as_str()) {
                return Err(format!(
                    "duplicate reference symbol {}",
                    asset.reference_symbol_native
                )
                .into());
            }
        }

        let mut source_ids = HashSet::new();
        for source in &self.sources {
            if source.source_id.trim().is_empty() {
                return Err("source source_id must not be empty".into());
            }
            if !source_ids.insert(source.source_id.as_str()) {
                return Err(format!("duplicate source_id {}", source.source_id).into());
            }
            if !matches!(
                source.source_category.as_str(),
                "news"
                    | "exchange_notice"
                    | "funding"
                    | "project_notice"
                    | "social"
                    | "governance"
                    | "developer_activity"
            ) {
                return Err(format!(
                    "source {} unsupported source_category {}",
                    source.source_id, source.source_category
                )
                .into());
            }
            if !matches!(source.trust_tier.as_str(), "T0" | "T1" | "T2") {
                return Err(format!(
                    "source {} unsupported trust_tier {}",
                    source.source_id, source.trust_tier
                )
                .into());
            }
            if !matches!(source.cadence_tier.as_str(), "high" | "medium" | "low") {
                return Err(format!(
                    "source {} unsupported cadence_tier {}",
                    source.source_id, source.cadence_tier
                )
                .into());
            }
            if !matches!(
                source.top50_relevance_mode.as_str(),
                "symbol_alias_match" | "direct_asset"
            ) {
                return Err(format!(
                    "source {} unsupported top50_relevance_mode {}",
                    source.source_id, source.top50_relevance_mode
                )
                .into());
            }
            if let Some(source_state) = &source.source_state {
                if !matches!(
                    source_state.as_str(),
                    "enabled" | "available_disabled" | "blocked" | "unsupported"
                ) {
                    return Err(format!(
                        "source {} unsupported source_state {}",
                        source.source_id, source_state
                    )
                    .into());
                }
                if source.enabled && source_state != "enabled" {
                    return Err(format!(
                        "source {} enabled=true requires source_state enabled",
                        source.source_id
                    )
                    .into());
                }
                if !source.enabled && source_state == "enabled" {
                    return Err(format!(
                        "source {} source_state enabled conflicts with enabled=false",
                        source.source_id
                    )
                    .into());
                }
                if !source.enabled
                    && source_state != "enabled"
                    && source
                        .activation_blocker
                        .as_deref()
                        .unwrap_or("")
                        .trim()
                        .is_empty()
                {
                    return Err(format!(
                        "source {} disabled inventory source requires activation_blocker",
                        source.source_id
                    )
                    .into());
                }
            }
            if source.enabled
                && !matches!(
                    source.fetch_method.as_str(),
                    "rss" | "rest_api" | "html_crawl"
                )
            {
                return Err(format!(
                    "enabled source {} uses unsupported fetch_method {}",
                    source.source_id, source.fetch_method
                )
                .into());
            }
            if source.fetch_method == "rest_api"
                && source.adapter.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(
                    format!("rest_api source {} must define adapter", source.source_id).into(),
                );
            }
            if let Some(max_items_per_run) = source.max_items_per_run
                && max_items_per_run == 0
            {
                return Err(format!(
                    "source {} max_items_per_run must be greater than zero",
                    source.source_id
                )
                .into());
            }
            if !source.source_url.starts_with("https://") {
                return Err(
                    format!("source {} source_url must use https", source.source_id).into(),
                );
            }
            if let AppliesToAssets::All(value) = &source.applies_to_assets
                && value != "all_major_50"
            {
                return Err(format!(
                    "source {} unsupported applies_to_assets value {}",
                    source.source_id, value
                )
                .into());
            }
            match &source.applies_to_assets {
                AppliesToAssets::All(_) => {
                    if source.top50_relevance_mode != "symbol_alias_match" {
                        return Err(format!(
                            "source {} all_major_50 requires symbol_alias_match",
                            source.source_id
                        )
                        .into());
                    }
                }
                AppliesToAssets::List(values) => {
                    if values.is_empty() {
                        return Err(format!(
                            "source {} applies_to_assets list must not be empty",
                            source.source_id
                        )
                        .into());
                    }
                    if source.top50_relevance_mode != "direct_asset" {
                        return Err(format!(
                            "source {} asset list requires direct_asset relevance mode",
                            source.source_id
                        )
                        .into());
                    }
                    if source.enabled {
                        for asset in values {
                            if !universe_assets.contains(asset.as_str()) {
                                return Err(format!(
                                    "enabled source {} applies to non-universe asset {}",
                                    source.source_id, asset
                                )
                                .into());
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Source {
    pub(crate) fn is_manual_backfill_source(&self) -> bool {
        !self.enabled
            && self.source_state.as_deref() == Some("available_disabled")
            && self.adapter.as_deref() == Some("binance_usdm_funding_rate_history")
    }
}

impl Source {
    pub(crate) fn direct_assets(&self) -> &[String] {
        match &self.applies_to_assets {
            AppliesToAssets::All(_) => &[],
            AppliesToAssets::List(values) => values,
        }
    }

    pub(crate) fn item_limit(&self, default_limit: usize) -> usize {
        self.max_items_per_run
            .map_or(default_limit, |source_limit| {
                source_limit.min(default_limit)
            })
    }
}

#[cfg(test)]
mod tests {
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
}
