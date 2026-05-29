mod detail;
mod item;
mod request;
mod types;

use crate::fetch::{CacheHeaders, SourceFetchResult, metadata_from_headers};
use crate::registry::Source;
use std::error::Error;
use tokio::time::{Duration, sleep};

#[cfg(test)]
pub(super) use self::detail::collect_binance_text;
use self::detail::fetch_binance_cms_article_body;
pub(super) use self::item::{binance_cms_article_item, binance_cms_article_metadata_body};
use self::request::get_json_response_with_retry;
use self::types::BinanceCmsResponse;
#[cfg(test)]
pub(super) use self::types::{BinanceCmsArticle, BinanceCmsCatalog};

pub(super) async fn fetch_binance_cms_announcements(
    client: &reqwest::Client,
    source: &Source,
    cache_headers: Option<&CacheHeaders>,
    max_items: usize,
) -> Result<SourceFetchResult, Box<dyn Error>> {
    let response = get_json_response_with_retry(client, &source.source_url, cache_headers).await?;
    let status = response.status();
    let metadata = metadata_from_headers(status, response.headers());
    if status == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(SourceFetchResult::NotModified { metadata });
    }
    if !status.is_success() {
        return Err(format!("{} returned HTTP {}", source.source_id, status.as_u16()).into());
    }
    let payload = response.json::<BinanceCmsResponse>().await?;
    if payload.code != "000000" {
        return Err(format!(
            "{} returned Binance code {}",
            source.source_id, payload.code
        )
        .into());
    }

    let mut items = Vec::new();
    if let Some(data) = payload.data {
        for catalog in data.catalogs {
            for article in &catalog.articles {
                sleep(Duration::from_millis(100)).await;
                let body = fetch_binance_cms_article_body(client, article)
                    .await
                    .unwrap_or_else(|| {
                        binance_cms_article_metadata_body(&source.source_id, &catalog, article)
                    });
                items.push(binance_cms_article_item(article, body));
                if items.len() >= max_items {
                    return Ok(SourceFetchResult::Fetched { items, metadata });
                }
            }
        }
    }
    Ok(SourceFetchResult::Fetched { items, metadata })
}
