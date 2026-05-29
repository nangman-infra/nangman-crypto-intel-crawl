use super::text::{
    clean_visible_text, context_body_from_clean_text, extract_meta_content, extract_tag_html,
    extract_tag_text,
};
use crate::item::FeedItem;
use crate::registry::Source;

const PAGE_SUMMARY_ID_SUFFIX: &str = "#page-summary";

pub(super) fn extract_page_summary_item(source: &Source, body: &str) -> Option<FeedItem> {
    if source.direct_assets().is_empty() {
        return None;
    }
    let title = extract_page_title(body).unwrap_or_else(|| source.source_name.clone());
    let summary_body = extract_page_summary_body(&title, body)?;
    Some(FeedItem {
        id: Some(format!("{}{}", source.source_url, PAGE_SUMMARY_ID_SUFFIX)),
        title,
        body: summary_body,
        url: source.source_url.clone(),
        author: None,
        published_at: None,
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    })
}

fn extract_page_title(body: &str) -> Option<String> {
    for selector in [
        ("property", "og:title"),
        ("name", "twitter:title"),
        ("name", "title"),
    ] {
        if let Some(value) = extract_meta_content(body, selector.0, selector.1) {
            return Some(value);
        }
    }
    extract_tag_text(body, "title")
}

fn extract_page_summary_body(title: &str, body: &str) -> Option<String> {
    let mut candidates = Vec::new();
    for selector in [
        ("name", "description"),
        ("property", "og:description"),
        ("name", "twitter:description"),
    ] {
        if let Some(value) = extract_meta_content(body, selector.0, selector.1) {
            candidates.push(value);
        }
    }
    if let Some(main) = extract_tag_html(body, "main") {
        candidates.push(clean_visible_text(main));
    }
    if let Some(body_html) = extract_tag_html(body, "body") {
        candidates.push(clean_visible_text(body_html));
    }
    candidates
        .into_iter()
        .map(|candidate| context_body_from_clean_text(&candidate, title))
        .find(|candidate| !candidate.is_empty())
}
