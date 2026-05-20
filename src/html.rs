use crate::fetch::{CacheHeaders, SourceFetchResult, apply_cache_headers, metadata_from_headers};
use crate::item::FeedItem;
use crate::registry::Source;
use std::error::Error;

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
    Ok(SourceFetchResult::Fetched {
        items: extract_anchor_items(&source.source_url, &body, max_items),
        metadata,
    })
}

fn looks_blocked(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("just a moment") && lower.contains("cloudflare")
}

fn extract_anchor_items(base_url: &str, body: &str, max_items: usize) -> Vec<FeedItem> {
    let mut items = Vec::new();
    let mut cursor = 0;
    while let Some(anchor_offset) = body[cursor..].find("<a") {
        let anchor_start = cursor + anchor_offset;
        let Some(anchor_open_end_offset) = body[anchor_start..].find('>') else {
            break;
        };
        let anchor_open_end = anchor_start + anchor_open_end_offset;
        let open_tag = &body[anchor_start..=anchor_open_end];
        let Some(href) = extract_href(open_tag) else {
            cursor = anchor_open_end + 1;
            continue;
        };
        let Some(anchor_close_offset) = body[anchor_open_end + 1..].find("</a>") else {
            break;
        };
        let anchor_close = anchor_open_end + 1 + anchor_close_offset;
        let title = clean_html_text(&body[anchor_open_end + 1..anchor_close]);
        cursor = anchor_close + "</a>".len();

        if title.chars().count() < 8 {
            continue;
        }
        let url = absolutize_url(base_url, &href);
        items.push(FeedItem {
            id: Some(url.clone()),
            title,
            body: String::new(),
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

fn extract_href(open_tag: &str) -> Option<String> {
    let href_pos = open_tag.find("href=")?;
    let after = &open_tag[href_pos + "href=".len()..];
    let mut chars = after.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value = chars.take_while(|ch| *ch != quote).collect::<String>();
    if value.trim().is_empty() || value.starts_with('#') || value.starts_with("javascript:") {
        return None;
    }
    Some(value)
}

fn absolutize_url(base_url: &str, href: &str) -> String {
    if href.starts_with("https://") || href.starts_with("http://") {
        return href.to_owned();
    }
    if href.starts_with('/')
        && let Some((scheme, rest)) = base_url.split_once("://")
        && let Some(host) = rest.split('/').next()
    {
        return format!("{scheme}://{host}{href}");
    }
    let base = base_url.trim_end_matches('/');
    format!("{base}/{href}")
}

fn clean_html_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_static_anchor_items() {
        let body = r#"<html><body><a href="/service_center/notice?id=1"><span>거래 지원 종료 안내</span></a></body></html>"#;

        let items = extract_anchor_items("https://upbit.com/service_center/notice", body, 5);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "거래 지원 종료 안내");
        assert_eq!(items[0].url, "https://upbit.com/service_center/notice?id=1");
    }
}
