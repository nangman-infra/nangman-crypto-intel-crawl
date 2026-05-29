use super::super::Source;
use std::error::Error;

pub(super) fn validate_fetch_contract(source: &Source) -> Result<(), Box<dyn Error>> {
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
        return Err(format!("rest_api source {} must define adapter", source.source_id).into());
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
        return Err(format!("source {} source_url must use https", source.source_id).into());
    }
    Ok(())
}
