use crate::balance::SourceBalancePolicy;
use crate::fetch::{self, SourceFetchResult};
use crate::registry::{Source, SourceRegistry};
use crate::{html, rest_api, rss};
use std::error::Error;
use tokio::time::{Duration, sleep};

const SOURCE_FETCH_MAX_ATTEMPTS: usize = 3;
const SOURCE_FETCH_RETRY_BASE_MS: u64 = 750;

pub(crate) struct SourceFetchRequest<'a> {
    pub(crate) client: &'a reqwest::Client,
    pub(crate) registry: &'a SourceRegistry,
    pub(crate) source: &'a Source,
    pub(crate) cache_headers: Option<fetch::CacheHeaders>,
    pub(crate) default_max_items: usize,
    pub(crate) balance_policy: SourceBalancePolicy,
    pub(crate) backfill_start_ms: Option<i64>,
    pub(crate) backfill_end_ms: Option<i64>,
    pub(crate) selection_time_ms: i64,
}

pub(crate) async fn fetch_source_items(
    request: SourceFetchRequest<'_>,
) -> Result<SourceFetchResult, Box<dyn Error>> {
    let mut last_retryable_error: Option<String> = None;
    for attempt in 1..=SOURCE_FETCH_MAX_ATTEMPTS {
        match fetch_source_items_once(&request).await {
            Ok(result) => return Ok(result),
            Err(error) => {
                let error_message = error.to_string();
                if !is_retryable_fetch_error(&error_message) {
                    return Err(error);
                }
                if attempt == SOURCE_FETCH_MAX_ATTEMPTS {
                    return Err(format!(
                        "{error_message} after {SOURCE_FETCH_MAX_ATTEMPTS} attempts"
                    )
                    .into());
                }
                last_retryable_error = Some(error_message);
                sleep(fetch_retry_delay(attempt)).await;
            }
        }
    }
    Err(last_retryable_error
        .unwrap_or_else(|| "source fetch failed".to_owned())
        .into())
}

async fn fetch_source_items_once(
    request: &SourceFetchRequest<'_>,
) -> Result<SourceFetchResult, Box<dyn Error>> {
    let item_limit = request.balance_policy.effective_item_limit(
        request.source,
        request.source.item_limit(request.default_max_items),
    );
    match request.source.fetch_method.as_str() {
        "rss" => {
            rss::fetch_feed_items(
                request.client,
                request.source,
                request.cache_headers.as_ref(),
                item_limit,
            )
            .await
        }
        "rest_api" => {
            rest_api::fetch_feed_items(
                request.client,
                request.source,
                rest_api::RestFetchOptions {
                    assets: &request.registry.universe_assets,
                    cache_headers: request.cache_headers.as_ref(),
                    max_items: item_limit,
                    backfill_start_ms: request.backfill_start_ms,
                    backfill_end_ms: request.backfill_end_ms,
                    selection_time_ms: request.selection_time_ms,
                },
            )
            .await
        }
        "html_crawl" => {
            html::fetch_feed_items(
                request.client,
                request.source,
                request.cache_headers.as_ref(),
                item_limit,
            )
            .await
        }
        other => Err(format!(
            "{} unsupported fetch_method {other}",
            request.source.source_id
        )
        .into()),
    }
}

fn fetch_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(SOURCE_FETCH_RETRY_BASE_MS.saturating_mul(attempt as u64))
}

fn is_retryable_fetch_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("timed out")
        || lower.contains("connection")
        || lower.contains("operation timed out")
        || lower.contains("request timeout")
        || [408, 425, 429, 500, 502, 503, 504]
            .iter()
            .any(|status| lower.contains(&format!("http {status}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_fetch_errors_cover_transient_source_failures() {
        assert!(is_retryable_fetch_error(
            "social_hackernews_bitcoin_rss returned HTTP 502"
        ));
        assert!(is_retryable_fetch_error("news feed returned HTTP 503"));
        assert!(is_retryable_fetch_error("request timed out"));
        assert!(is_retryable_fetch_error(
            "connection closed before message completed"
        ));
    }

    #[test]
    fn retryable_fetch_errors_exclude_structural_failures() {
        assert!(!is_retryable_fetch_error("news returned HTTP 404"));
        assert!(!is_retryable_fetch_error(
            "source returned a bot challenge page"
        ));
        assert!(!is_retryable_fetch_error(
            "unsupported fetch_method websocket"
        ));
    }

    #[test]
    fn fetch_retry_delay_uses_short_linear_backoff() {
        assert_eq!(fetch_retry_delay(1), Duration::from_millis(750));
        assert_eq!(fetch_retry_delay(2), Duration::from_millis(1_500));
    }
}
