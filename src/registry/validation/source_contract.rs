use super::super::Source;
use super::applies_to::validate_applies_to_assets;
use super::fetch_contract::validate_fetch_contract;
use super::source_state::validate_source_state;
use std::collections::HashSet;
use std::error::Error;

pub(super) fn validate_source_id<'a>(
    source: &'a Source,
    source_ids: &mut HashSet<&'a str>,
) -> Result<(), Box<dyn Error>> {
    if source.source_id.trim().is_empty() {
        return Err("source source_id must not be empty".into());
    }
    if !source_ids.insert(source.source_id.as_str()) {
        return Err(format!("duplicate source_id {}", source.source_id).into());
    }
    Ok(())
}

pub(super) fn validate_source_contract(
    source: &Source,
    universe_assets: &HashSet<&str>,
) -> Result<(), Box<dyn Error>> {
    validate_source_enums(source)?;
    validate_source_state(source)?;
    validate_fetch_contract(source)?;
    validate_applies_to_assets(source, universe_assets)
}

fn validate_source_enums(source: &Source) -> Result<(), Box<dyn Error>> {
    validate_allowed_value(
        source,
        "source_category",
        &source.source_category,
        &[
            "news",
            "exchange_notice",
            "funding",
            "project_notice",
            "social",
            "governance",
            "developer_activity",
        ],
    )?;
    validate_allowed_value(
        source,
        "trust_tier",
        &source.trust_tier,
        &["T0", "T1", "T2"],
    )?;
    validate_allowed_value(
        source,
        "cadence_tier",
        &source.cadence_tier,
        &["high", "medium", "low"],
    )?;
    validate_allowed_value(
        source,
        "top50_relevance_mode",
        &source.top50_relevance_mode,
        &["symbol_alias_match", "direct_asset"],
    )
}

pub(super) fn validate_allowed_value(
    source: &Source,
    field_name: &str,
    value: &str,
    allowed: &[&str],
) -> Result<(), Box<dyn Error>> {
    if allowed.contains(&value) {
        return Ok(());
    }
    Err(format!(
        "source {} unsupported {} {}",
        source.source_id, field_name, value
    )
    .into())
}
