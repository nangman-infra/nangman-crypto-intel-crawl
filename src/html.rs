use crate::fetch::{CacheHeaders, SourceFetchResult, apply_cache_headers, metadata_from_headers};
use crate::registry::Source;
use std::error::Error;

mod anchors;
mod page_summary;
#[cfg(test)]
mod tests;
mod text;

use anchors::extract_anchor_items;
use page_summary::extract_page_summary_item;

pub(crate) async fn fetch_feed_items(
    client: &reqwest::Client,
    source: &Source,
    cache_headers: Option<&CacheHeaders>,
    max_items: usize,
) -> Result<SourceFetchResult, Box<dyn Error>> {
    let request = client
        .get(&source.source_url)
        .header("Accept", "text/html,application/xhtml+xml");
    let response = apply_cache_headers(request, cache_headers).send().await?;
    let status = response.status();
    let metadata = metadata_from_headers(status, response.headers());
    if status == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(SourceFetchResult::NotModified { metadata });
    }
    if !status.is_success() {
        return Err(format!("{} returned HTTP {}", source.source_id, status.as_u16()).into());
    }
    let body = response.text().await?;
    if looks_blocked(&body) {
        return Err(format!("{} returned a bot challenge page", source.source_id).into());
    }
    let mut items = extract_anchor_items(&source.source_url, &body, max_items);
    if let Some(page_summary) = extract_page_summary_item(source, &body) {
        items.insert(0, page_summary);
        items.truncate(max_items);
    }
    Ok(SourceFetchResult::Fetched { items, metadata })
}

fn looks_blocked(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("just a moment") && lower.contains("cloudflare")
}
