use super::{CrawlBuffers, CrawlContext, CrawlSummary};
use crate::balance::SourceRunStats;
use crate::fetch::SourceFetchResult;
use crate::registry::Source;
use crate::source_fetch::{SourceFetchRequest, fetch_source_items};
use crate::source_schedule::should_send_conditional_fetch_headers;
use crate::source_state::now_ms;
use chrono::Utc;
use record::{record_source_failure, record_source_items, record_source_not_modified};
use skip::{should_skip_source_for_backoff, should_skip_source_for_cadence};
use std::error::Error;

mod record;
mod skip;

pub(super) async fn crawl_source(
    context: &mut CrawlContext<'_>,
    source: &Source,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) -> Result<(), Box<dyn Error>> {
    let checked_at_ms = Utc::now().timestamp_millis();
    if should_skip_source_for_backoff(context, source, checked_at_ms, buffers, summary) {
        return Ok(());
    }
    if should_skip_source_for_cadence(context, source, checked_at_ms, buffers, summary) {
        return Ok(());
    }
    let mut source_stats = SourceRunStats::default();
    let result = fetch_source_items(SourceFetchRequest {
        client: context.client,
        registry: context.registry,
        source,
        cache_headers: if should_send_conditional_fetch_headers(context.args) {
            context
                .source_states
                .get(source)
                .map(|state| state.cache_headers())
        } else {
            None
        },
        default_max_items: context.args.max_items_per_source,
        balance_policy: context.balance_policy,
        backfill_start_ms: context.args.backfill_start_ms,
        backfill_end_ms: context.args.backfill_end_ms,
        selection_time_ms: checked_at_ms,
    })
    .await;

    match result {
        Ok(SourceFetchResult::Fetched { items, metadata }) => {
            context
                .source_states
                .get_mut(source)
                .record_success(&metadata, now_ms(), items.len());
            record_source_items(
                context,
                source,
                checked_at_ms,
                items,
                &mut source_stats,
                buffers,
                summary,
            )
            .await?;
        }
        Ok(SourceFetchResult::NotModified { metadata }) => {
            context
                .source_states
                .get_mut(source)
                .record_not_modified(&metadata, now_ms());
            record_source_not_modified(context, source, checked_at_ms, buffers, summary);
        }
        Err(error) => record_source_failure(
            context,
            source,
            checked_at_ms,
            error,
            source_stats,
            buffers,
            summary,
        ),
    }
    Ok(())
}
