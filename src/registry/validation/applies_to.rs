use super::super::{AppliesToAssets, Source};
use std::collections::HashSet;
use std::error::Error;

pub(super) fn validate_applies_to_assets(
    source: &Source,
    universe_assets: &HashSet<&str>,
) -> Result<(), Box<dyn Error>> {
    match &source.applies_to_assets {
        AppliesToAssets::All(value) => validate_all_assets_source(source, value),
        AppliesToAssets::List(values) => {
            validate_asset_list_source(source, values, universe_assets)
        }
    }
}

fn validate_all_assets_source(source: &Source, value: &str) -> Result<(), Box<dyn Error>> {
    if value != "all_major_50" {
        return Err(format!(
            "source {} unsupported applies_to_assets value {}",
            source.source_id, value
        )
        .into());
    }
    if source.top50_relevance_mode != "symbol_alias_match" {
        return Err(format!(
            "source {} all_major_50 requires symbol_alias_match",
            source.source_id
        )
        .into());
    }
    Ok(())
}

fn validate_asset_list_source(
    source: &Source,
    values: &[String],
    universe_assets: &HashSet<&str>,
) -> Result<(), Box<dyn Error>> {
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
    validate_enabled_asset_membership(source, values, universe_assets)
}

fn validate_enabled_asset_membership(
    source: &Source,
    values: &[String],
    universe_assets: &HashSet<&str>,
) -> Result<(), Box<dyn Error>> {
    if !source.enabled {
        return Ok(());
    }
    for asset in values {
        if !universe_assets.contains(asset.as_str()) {
            return Err(format!(
                "enabled source {} applies to non-universe asset {}",
                source.source_id, asset
            )
            .into());
        }
    }
    Ok(())
}
