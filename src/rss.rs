use crate::fetch::{CacheHeaders, SourceFetchResult, apply_cache_headers, metadata_from_headers};
use crate::item::FeedItem;
use crate::registry::Source;
use roxmltree::{Document, Node};
use std::error::Error;

pub(crate) async fn fetch_feed_items(
    client: &reqwest::Client,
    source: &Source,
    cache_headers: Option<&CacheHeaders>,
    max_items: usize,
) -> Result<SourceFetchResult, Box<dyn Error>> {
    let request = client.get(&source.source_url).header(
        "Accept",
        "application/rss+xml, application/atom+xml, application/xml, text/xml",
    );
    let response = apply_cache_headers(request, cache_headers).send().await?;
    let status = response.status();
    let metadata = metadata_from_headers(status, response.headers());
    if status == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(SourceFetchResult::NotModified { metadata });
    }
    if !status.is_success() {
        return Err(format!("{} returned HTTP {}", source.source_id, status.as_u16()).into());
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body = response.text().await?;
    if !looks_like_xml_feed(&body, &content_type) {
        return Err(format!("{} did not return an XML feed", source.source_id).into());
    }
    Ok(SourceFetchResult::Fetched {
        items: parse_feed_items(&body, max_items)?,
        metadata,
    })
}

fn looks_like_xml_feed(body: &str, content_type: &str) -> bool {
    let trimmed = body.trim_start();
    trimmed.starts_with("<?xml")
        || trimmed.starts_with("<rss")
        || trimmed.starts_with("<feed")
        || content_type.contains("xml")
        || content_type.contains("rss")
        || content_type.contains("atom")
}

fn parse_feed_items(body: &str, max_items: usize) -> Result<Vec<FeedItem>, Box<dyn Error>> {
    let doc = Document::parse(body)?;
    let rss_items = doc
        .descendants()
        .filter(|node| tag_name(*node) == "item")
        .take(max_items)
        .map(rss_item)
        .collect::<Vec<_>>();
    if !rss_items.is_empty() {
        return Ok(rss_items);
    }

    Ok(doc
        .descendants()
        .filter(|node| tag_name(*node) == "entry")
        .take(max_items)
        .map(atom_entry)
        .collect())
}

fn rss_item(node: Node<'_, '_>) -> FeedItem {
    FeedItem {
        id: first_child_text(node, &["guid"]).filter(|value| !value.trim().is_empty()),
        title: first_child_text(node, &["title"]).unwrap_or_default(),
        body: first_child_text(node, &["description", "encoded", "content"]).unwrap_or_default(),
        url: first_child_text(node, &["link"]).unwrap_or_default(),
        author: first_child_text(node, &["creator", "author"]),
        published_at: first_child_text(node, &["pubDate", "published", "updated"]),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}

fn atom_entry(node: Node<'_, '_>) -> FeedItem {
    FeedItem {
        id: first_child_text(node, &["id"]).filter(|value| !value.trim().is_empty()),
        title: first_child_text(node, &["title"]).unwrap_or_default(),
        body: first_child_text(node, &["summary", "content"]).unwrap_or_default(),
        url: atom_link(node).unwrap_or_default(),
        author: atom_author(node),
        published_at: first_child_text(node, &["published", "updated"]),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}

fn atom_link(node: Node<'_, '_>) -> Option<String> {
    node.children()
        .filter(|child| child.is_element() && tag_name(*child) == "link")
        .find_map(|child| {
            child
                .attribute("href")
                .or_else(|| child.text())
                .map(normalize_text)
        })
}

fn atom_author(node: Node<'_, '_>) -> Option<String> {
    node.children()
        .filter(|child| child.is_element() && tag_name(*child) == "author")
        .find_map(|author| {
            first_child_text(author, &["name"]).or_else(|| author.text().map(normalize_text))
        })
}

fn first_child_text(node: Node<'_, '_>, names: &[&str]) -> Option<String> {
    node.children()
        .filter(|child| child.is_element())
        .find(|child| names.iter().any(|name| tag_name(*child) == *name))
        .and_then(|child| child.text())
        .map(normalize_text)
}

fn tag_name<'a, 'input>(node: Node<'a, 'input>) -> &'input str {
    node.tag_name().name()
}

fn normalize_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rss_items() {
        let feed = r#"<?xml version="1.0"?><rss><channel><item><title>BTC update</title><link>https://example.com/a</link><description>Body</description><guid>g1</guid><pubDate>Thu, 07 May 2026 03:13:11 +0000</pubDate></item></channel></rss>"#;

        let items = parse_feed_items(feed, 10).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "BTC update");
        assert_eq!(items[0].url, "https://example.com/a");
    }

    #[test]
    fn parses_atom_entries() {
        let feed = r#"<?xml version="1.0"?><feed><entry><title>ETH update</title><link href="https://example.com/e"/><summary>Body</summary><id>id1</id><updated>2026-05-07T03:13:11Z</updated></entry></feed>"#;

        let items = parse_feed_items(feed, 10).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "ETH update");
        assert_eq!(items[0].url, "https://example.com/e");
    }
}
