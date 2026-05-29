use super::super::events::{append_source_events, build_candidate_events};
use super::super::{CrawlBuffers, CrawlContext, CrawlSummary, SourceFailure};
use crate::balance::{SourceBalanceRecord, SourceRunStats};
use crate::health::{SourceHealRecord, SourceHealthRecord};
use crate::item::FeedItem;
use crate::registry::Source;
use chrono::Utc;
use std::error::Error;

pub(super) async fn record_source_items(
    context: &mut CrawlContext<'_>,
    source: &Source,
    checked_at_ms: i64,
    items: Vec<FeedItem>,
    source_stats: &mut SourceRunStats,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) -> Result<(), Box<dyn Error>> {
    let fetched_at_ms = Utc::now().timestamp_millis();
    let source_items_seen = items.len();
    source_stats.items_seen = source_items_seen;
    summary.sources_ok += 1;
    summary.items_seen += source_items_seen;
    let mut events = build_candidate_events(context, source, items, fetched_at_ms);
    if !context.args.dry_run
        && let Some(object_store) = context.object_store
    {
        buffers
            .dedup
            .load_candidate_shards(object_store, &events, context.args.dedup_lookback_days)
            .await?;
    }
    let source_outcome =
        append_source_events(context, source, &mut events, source_stats, buffers, summary);
    buffers.health_records.push(SourceHealthRecord::ok(
        source,
        checked_at_ms,
        Utc::now().timestamp_millis(),
        source_items_seen,
        source_outcome.events_written,
        source_outcome.duplicates_skipped,
    ));
    buffers.balance_records.push(SourceBalanceRecord::new(
        source,
        source_stats.clone(),
        context.balance_policy,
        Utc::now().timestamp_millis(),
    ));
    summary.source_health_written += usize::from(!context.args.dry_run);
    Ok(())
}

pub(super) fn record_source_not_modified(
    context: &CrawlContext<'_>,
    source: &Source,
    checked_at_ms: i64,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) {
    let observed_at_ms = Utc::now().timestamp_millis();
    summary.sources_ok += 1;
    buffers
        .health_records
        .push(SourceHealthRecord::not_modified(
            source,
            checked_at_ms,
            observed_at_ms,
        ));
    buffers.balance_records.push(SourceBalanceRecord::new(
        source,
        SourceRunStats::default(),
        context.balance_policy,
        observed_at_ms,
    ));
    summary.source_health_written += usize::from(!context.args.dry_run);
}

pub(super) fn record_source_failure(
    context: &mut CrawlContext<'_>,
    source: &Source,
    checked_at_ms: i64,
    error: Box<dyn Error>,
    source_stats: SourceRunStats,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) {
    let error = error.to_string();
    let observed_at_ms = Utc::now().timestamp_millis();
    summary.sources_failed += 1;
    summary.failures.push(SourceFailure {
        source_id: source.source_id.clone(),
        error: error.clone(),
    });
    buffers.health_records.push(SourceHealthRecord::failed(
        source,
        checked_at_ms,
        observed_at_ms,
        error.clone(),
    ));
    buffers.balance_records.push(SourceBalanceRecord::new(
        source,
        source_stats,
        context.balance_policy,
        observed_at_ms,
    ));
    summary.source_health_written += usize::from(!context.args.dry_run);
    buffers
        .heal_records
        .push(SourceHealRecord::retry_after_failure(
            source,
            observed_at_ms,
            &error,
            context.args.schedule_interval_ms,
        ));
    summary.source_heal_written += usize::from(!context.args.dry_run);
    context
        .source_states
        .get_mut(source)
        .record_failure(&error, observed_at_ms);
}
