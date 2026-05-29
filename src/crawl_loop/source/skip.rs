use super::super::{CrawlBuffers, CrawlContext, CrawlSummary};
use crate::balance::{SourceBalanceRecord, SourceRunStats};
use crate::health::SourceHealthRecord;
use crate::registry::Source;
use crate::source_schedule::cadence_interval_ms;
use chrono::Utc;

pub(super) fn should_skip_source_for_backoff(
    context: &CrawlContext<'_>,
    source: &Source,
    checked_at_ms: i64,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) -> bool {
    let Some(state) = context.source_states.get(source) else {
        return false;
    };
    if !state.is_backing_off(checked_at_ms) {
        return false;
    }
    let observed_at_ms = Utc::now().timestamp_millis();
    summary.sources_ok += 1;
    buffers
        .health_records
        .push(SourceHealthRecord::skipped_backoff(
            source,
            checked_at_ms,
            observed_at_ms,
            state.backoff_until_ms(),
        ));
    buffers.balance_records.push(SourceBalanceRecord::new(
        source,
        SourceRunStats::default(),
        context.balance_policy,
        observed_at_ms,
    ));
    summary.source_health_written += usize::from(!context.args.dry_run);
    true
}

pub(super) fn should_skip_source_for_cadence(
    context: &CrawlContext<'_>,
    source: &Source,
    checked_at_ms: i64,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) -> bool {
    if context.args.source_id.is_some() || context.args.backfill_start_ms.is_some() {
        return false;
    }
    if context.args.backfill_end_ms.is_some() {
        return false;
    }
    let Some(state) = context.source_states.get(source) else {
        return false;
    };
    let Some(last_success_at_ms) = state.last_success_at_ms() else {
        return false;
    };
    let next_due_at_ms = last_success_at_ms.saturating_add(cadence_interval_ms(source));
    if checked_at_ms >= next_due_at_ms {
        return false;
    }

    let observed_at_ms = Utc::now().timestamp_millis();
    summary.sources_ok += 1;
    buffers
        .health_records
        .push(SourceHealthRecord::skipped_cadence(
            source,
            checked_at_ms,
            observed_at_ms,
            next_due_at_ms,
        ));
    buffers.balance_records.push(SourceBalanceRecord::new(
        source,
        SourceRunStats::default(),
        context.balance_policy,
        observed_at_ms,
    ));
    summary.source_health_written += usize::from(!context.args.dry_run);
    true
}
