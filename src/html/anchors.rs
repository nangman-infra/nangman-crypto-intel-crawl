use super::text::clean_html_text;
use crate::item::FeedItem;

mod context;
mod link;
mod scan;

use context::extract_context_body;
use link::{absolutize_url, extract_href, should_skip_anchor};
use scan::next_anchor;

pub(super) fn extract_anchor_items(base_url: &str, body: &str, max_items: usize) -> Vec<FeedItem> {
    let mut items = Vec::new();
    let mut cursor = 0;
    while let Some(anchor) = next_anchor(body, cursor) {
        cursor = anchor.end;
        let Some(href) = extract_href(anchor.open_tag) else {
            continue;
        };
        let title = clean_html_text(anchor.title_html);

        if should_skip_anchor(&title, &href) {
            continue;
        }
        let url = absolutize_url(base_url, &href);
        let context_body = extract_context_body(body, anchor.start, anchor.end, &title);
        items.push(FeedItem {
            id: Some(url.clone()),
            title,
            body: context_body,
            url,
            author: None,
            published_at: None,
            historical_source_depth: None,
            backfill_window_start_ms: None,
            backfill_window_end_ms: None,
            source_time_range_verified: None,
        });
        if items.len() >= max_items {
            break;
        }
    }
    items
}
