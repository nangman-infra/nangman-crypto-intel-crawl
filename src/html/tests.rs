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
fn direct_asset_page_summary_trims_purchase_sections() {
    let source = source("project_pepe_official_html", &["PEPE"]);
    let body = r#"
          <html>
            <head><title>PEPE</title></head>
            <body>
              <main>
                <h1>$pepe</h1>
                <p>The most memeable memecoin in existence.</p>
                <p>Launched stealth with no presale, zero taxes, LP burnt and contract renounced, $PEPE is a coin for the people.</p>
                <h2>How to buy</h2>
                <p>Create a wallet and buy from an exchange.</p>
              </main>
            </body>
          </html>
        "#;

    let page_summary = extract_page_summary_item(&source, body).unwrap();

    assert!(page_summary.body.contains("zero taxes"));
    assert!(
        !page_summary
            .body
            .to_ascii_lowercase()
            .contains("how to buy")
    );
    assert!(!page_summary.body.to_ascii_lowercase().contains("buy from"));
}

#[test]
fn direct_asset_page_summary_keeps_body_when_action_marker_is_navigation() {
    let source = source("project_pepe_official_html", &["PEPE"]);
    let body = r#"
          <html>
            <head><title>PEPE</title></head>
            <body>
              <main>
                <nav><a href="/how-to-buy">How to buy</a><a href="/buy">Buy now</a></nav>
                <h1>$pepe</h1>
                <p>The most memeable memecoin in existence.</p>
                <p>Launched stealth with no presale, zero taxes, LP burnt and contract renounced, $PEPE is a coin for the people.</p>
              </main>
            </body>
          </html>
        "#;

    let page_summary = extract_page_summary_item(&source, body).unwrap();

    assert!(page_summary.body.contains("zero taxes"));
    assert!(page_summary.body.contains("$pepe"));
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

#[test]
fn skips_unsafe_href_schemes_case_insensitively() {
    let body = r#"
          <html><body>
            <a href="JavaScript:alert(1)">Protocol upgrade approved</a>
            <a href="DATA:text/html;base64,deadbeef">Token launch details</a>
            <a href="/updates/token-launch">Token launch details</a>
          </body></html>
        "#;

    let items = extract_anchor_items("https://example.org", body, 5);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].url, "https://example.org/updates/token-launch");
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
