use crate::fetch::{CacheHeaders, SourceFetchResult, apply_cache_headers, metadata_from_headers};
use crate::item::FeedItem;
use crate::registry::Source;
use std::error::Error;

const MIN_ANCHOR_TITLE_CHARS: usize = 8;
const MIN_CONTEXT_BODY_CHARS: usize = 40;
const MAX_CONTEXT_SCAN_BYTES: usize = 2_000;
const MAX_CONTEXT_BODY_CHARS: usize = 1_200;
const PAGE_SUMMARY_ID_SUFFIX: &str = "#page-summary";

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

        if should_skip_anchor(&title, &href) {
            continue;
        }
        let url = absolutize_url(base_url, &href);
        let context_body = extract_context_body(body, anchor_start, cursor, &title);
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

fn extract_page_summary_item(source: &Source, body: &str) -> Option<FeedItem> {
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
        .find(|candidate| candidate.chars().count() >= MIN_CONTEXT_BODY_CHARS)
}

fn extract_meta_content(body: &str, selector_attr: &str, selector_value: &str) -> Option<String> {
    let mut cursor = 0;
    while let Some(offset) = body[cursor..].find("<meta") {
        let tag_start = cursor + offset;
        let Some(tag_end_offset) = body[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_offset;
        let tag = &body[tag_start..=tag_end];
        let selector_matches = attr_value(tag, selector_attr)
            .is_some_and(|value| value.eq_ignore_ascii_case(selector_value));
        if selector_matches && let Some(content) = attr_value(tag, "content") {
            let cleaned = clean_html_text(&content);
            if !cleaned.trim().is_empty() {
                return Some(cleaned);
            }
        }
        cursor = tag_end + 1;
    }
    None
}

fn extract_tag_text(body: &str, tag_name: &str) -> Option<String> {
    let tag_html = extract_tag_html(body, tag_name)?;
    let cleaned = clean_html_text(tag_html);
    (!cleaned.trim().is_empty()).then_some(cleaned)
}

fn extract_tag_html<'a>(body: &'a str, tag_name: &str) -> Option<&'a str> {
    let open = format!("<{tag_name}");
    let close = format!("</{tag_name}>");
    let start = body.find(&open)?;
    let open_end = body[start..].find('>').map(|offset| start + offset + 1)?;
    let close_start = body[open_end..]
        .find(&close)
        .map(|offset| open_end + offset)?;
    Some(&body[open_end..close_start])
}

fn extract_href(open_tag: &str) -> Option<String> {
    let value = attr_value(open_tag, "href")?;
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("tel:")
    {
        return None;
    }
    Some(trimmed.to_owned())
}

fn attr_value(tag: &str, attr_name: &str) -> Option<String> {
    let attr_pos = tag.find(&format!("{attr_name}="))?;
    let after = &tag[attr_pos + attr_name.len() + 1..];
    let mut chars = after.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    Some(chars.take_while(|ch| *ch != quote).collect::<String>())
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

fn should_skip_anchor(title: &str, href: &str) -> bool {
    title.chars().count() < MIN_ANCHOR_TITLE_CHARS
        || is_navigation_title(title)
        || is_static_asset_link(href)
}

fn is_navigation_title(title: &str) -> bool {
    let normalized = title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase();
    matches!(
        normalized.as_str(),
        "ABOUT"
            | "ABOUT US"
            | "APP"
            | "BLOG"
            | "CAREERS"
            | "CONTACT"
            | "CONTACT US"
            | "COOKIE POLICY"
            | "DISCORD"
            | "DOCS"
            | "DOCUMENTATION"
            | "GITHUB"
            | "GET STARTED"
            | "HOME"
            | "LAUNCH APP"
            | "LEARN MORE"
            | "LOGIN"
            | "MEDIUM"
            | "MENU"
            | "NEWS"
            | "PRIVACY"
            | "PRIVACY POLICY"
            | "READ MORE"
            | "SIGN IN"
            | "TELEGRAM"
            | "TERMS"
            | "TERMS OF SERVICE"
            | "TWITTER"
            | "X"
    )
}

fn is_static_asset_link(href: &str) -> bool {
    let lower = href.split('?').next().unwrap_or(href).to_ascii_lowercase();
    matches!(
        lower.rsplit('.').next(),
        Some("avif" | "css" | "gif" | "ico" | "jpeg" | "jpg" | "js" | "png" | "svg" | "webp")
    )
}

fn extract_context_body(
    document: &str,
    anchor_start: usize,
    anchor_end: usize,
    title: &str,
) -> String {
    let start = find_context_start(document, anchor_start);
    let end = find_context_end(document, anchor_end);
    if start >= end || end > document.len() {
        return String::new();
    }
    let context = clean_html_text(&document[start..end]);
    context_body_from_clean_text(&context, title)
}

fn find_context_start(document: &str, anchor_start: usize) -> usize {
    let lookback_start = floor_char_boundary(
        document,
        anchor_start.saturating_sub(MAX_CONTEXT_SCAN_BYTES),
    );
    let lookback = &document[lookback_start..anchor_start];
    ["<article", "<li", "<section", "<div", "<p"]
        .iter()
        .filter_map(|tag| lookback.rfind(tag).map(|offset| lookback_start + offset))
        .max()
        .unwrap_or(lookback_start)
}

fn find_context_end(document: &str, anchor_end: usize) -> usize {
    let bounded_end = ceil_char_boundary(
        document,
        anchor_end
            .saturating_add(MAX_CONTEXT_SCAN_BYTES)
            .min(document.len()),
    );
    let lookahead = &document[anchor_end..bounded_end];
    ["</article>", "</li>", "</section>", "</div>", "</p>"]
        .iter()
        .filter_map(|tag| {
            lookahead
                .find(tag)
                .map(|offset| anchor_end + offset + tag.len())
        })
        .min()
        .unwrap_or(bounded_end)
}

fn floor_char_boundary(value: &str, mut index: usize) -> usize {
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(value: &str, mut index: usize) -> usize {
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn context_body_from_clean_text(context: &str, title: &str) -> String {
    let trimmed_context = context.trim();
    if trimmed_context.is_empty() {
        return String::new();
    }
    let trimmed_title = title.trim();
    let without_title = trimmed_context
        .strip_prefix(trimmed_title)
        .unwrap_or(trimmed_context)
        .trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '-' | '|' | ':' | '·'));
    let candidate = if without_title.chars().count() >= MIN_CONTEXT_BODY_CHARS {
        without_title
    } else if trimmed_context != trimmed_title
        && trimmed_context.chars().count() >= MIN_CONTEXT_BODY_CHARS
    {
        trimmed_context
    } else {
        ""
    };
    truncate_clean_text(candidate, MAX_CONTEXT_BODY_CHARS)
}

fn truncate_clean_text(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn clean_visible_text(value: &str) -> String {
    clean_html_text(&strip_ignored_html_blocks(value))
}

fn strip_ignored_html_blocks(value: &str) -> String {
    let mut output = value.to_owned();
    for tag in ["script", "style", "svg", "noscript"] {
        output = strip_html_block(&output, tag);
    }
    output
}

fn strip_html_block(value: &str, tag_name: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    let open_pattern = format!("<{tag_name}");
    let close_pattern = format!("</{tag_name}>");
    while let Some(start_offset) = value[cursor..].find(&open_pattern) {
        let start = cursor + start_offset;
        output.push_str(&value[cursor..start]);
        let Some(close_offset) = value[start..].find(&close_pattern) else {
            cursor = value.len();
            break;
        };
        cursor = start + close_offset + close_pattern.len();
    }
    output.push_str(&value[cursor..]);
    output
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
        .replace("&rsquo;", "'")
        .replace("&ldquo;", "\"")
        .replace("&rdquo;", "\"")
        .replace("&#x27;", "'")
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
        assert_eq!(items[0].body, "");
    }

    #[test]
    fn captures_article_card_context_body() {
        let body = r#"
          <html><body>
            <article>
              <a href="/blog/protocol-upgrade">Protocol upgrade approved</a>
              <p>Validators approved a network upgrade with a new execution schedule and migration notes for operators.</p>
            </article>
          </body></html>
        "#;

        let items = extract_anchor_items("https://example.org/blog", body, 5);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Protocol upgrade approved");
        assert_eq!(items[0].url, "https://example.org/blog/protocol-upgrade");
        assert_eq!(
            items[0].body,
            "Validators approved a network upgrade with a new execution schedule and migration notes for operators."
        );
    }

    #[test]
    fn prepends_direct_asset_page_summary_from_metadata() {
        let source = source("project_usd1_world_liberty_attestation_html", &["USD1"]);
        let body = r#"
          <html>
            <head>
              <title>World Liberty Financial - USD1 Attestation Reports</title>
              <meta name="description" content="View monthly USD1 reserve attestation reports for full transparency into the reserves backing USD1."/>
            </head>
            <body>
              <a href="/">Logo iconWorld Liberty Financial Logo</a>
            </body>
          </html>
        "#;

        let mut items = extract_anchor_items(&source.source_url, body, 5);
        if let Some(page_summary) = extract_page_summary_item(&source, body) {
            items.insert(0, page_summary);
        }

        assert_eq!(
            items[0].id.as_deref(),
            Some("https://example.org#page-summary")
        );
        assert_eq!(
            items[0].title,
            "World Liberty Financial - USD1 Attestation Reports"
        );
        assert!(items[0].body.contains("reserve attestation reports"));
    }

    #[test]
    fn direct_asset_page_summary_uses_visible_ssr_text() {
        let source = source("project_pepe_official_html", &["PEPE"]);
        let body = r#"
          <html>
            <head><title>PEPE</title></head>
            <body>
              <script>{"noisy":"TOKENOMICS"}</script>
              <main>
                <h1>$pepe</h1>
                <p>The most memeable memecoin in existence.</p>
                <p>Launched stealth with no presale, zero taxes, LP burnt and contract renounced, $PEPE is a coin for the people.</p>
              </main>
            </body>
          </html>
        "#;

        let page_summary = extract_page_summary_item(&source, body).unwrap();

        assert_eq!(page_summary.title, "PEPE");
        assert!(page_summary.body.contains("zero taxes"));
        assert!(!page_summary.body.contains("noisy"));
    }

    #[test]
    fn skips_page_summary_for_broad_html_sources() {
        let source = source("news_html", &[]);
        let body = r#"<html><head><title>News</title></head><body><main>Long broad market text that is not tied to a direct asset source.</main></body></html>"#;

        assert!(extract_page_summary_item(&source, body).is_none());
    }

    #[test]
    fn skips_navigation_and_static_asset_links() {
        let body = r#"
          <html><body>
            <a href="/blog">Blog</a>
            <a href="/assets/logo.svg">Download logo</a>
            <a href="/updates/token-launch">Token launch details</a>
          </body></html>
        "#;

        let items = extract_anchor_items("https://example.org", body, 5);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Token launch details");
    }

    fn source(source_id: &str, assets: &[&str]) -> Source {
        Source {
            source_id: source_id.to_owned(),
            source_category: "project_notice".to_owned(),
            source_name: source_id.to_owned(),
            source_url: "https://example.org".to_owned(),
            fetch_method: "html_crawl".to_owned(),
            adapter: None,
            max_items_per_run: None,
            trust_tier: "T1".to_owned(),
            cadence_tier: "low".to_owned(),
            language_hint: "en".to_owned(),
            enabled: true,
            source_state: None,
            activation_blocker: None,
            top50_relevance_mode: "direct_asset".to_owned(),
            applies_to_assets: crate::registry::AppliesToAssets::List(
                assets.iter().map(|asset| (*asset).to_owned()).collect(),
            ),
        }
    }
}
