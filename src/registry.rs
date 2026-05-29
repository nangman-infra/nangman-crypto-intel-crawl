use serde::Deserialize;
use std::error::Error;
use std::path::Path;

#[cfg(test)]
mod tests;
mod validation;

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
