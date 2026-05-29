use super::{CrawlBuffers, CrawlContext, CrawlSummary, SourceOutcome};
use crate::args::Args;
use crate::balance::{Admission, SourceBalancePolicy, SourceRunStats};
use crate::dedup::DedupDecision;
use crate::event::{RawIntelEvent, build_raw_intel_event};
use crate::item::FeedItem;
use crate::registry::Source;

pub(super) fn build_candidate_events(
    context: &CrawlContext<'_>,
    source: &Source,
    items: Vec<FeedItem>,
    fetched_at_ms: i64,
) -> Vec<RawIntelEvent> {
    items
        .into_iter()
        .map(|item| {
            let matched_assets = context
                .matcher
                .match_item(&item.title, &item.body, &item.url);
            build_raw_intel_event(source, &item, &matched_assets, fetched_at_ms)
        })
        .collect()
}

pub(super) fn append_source_events(
    context: &CrawlContext<'_>,
    source: &Source,
    events: &mut [RawIntelEvent],
    source_stats: &mut SourceRunStats,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) -> SourceOutcome {
    let mut outcome = SourceOutcome::default();
    for event in events {
        let dedup_decision = dedup_decision(context.args, buffers, event);
        event.set_dedup_outcome(
            dedup_decision.label(),
            dedup_decision.duplicate_of_event_id(),
        );
        if dedup_decision.is_skipped_duplicate() {
            summary.events_skipped_duplicate += 1;
            outcome.duplicates_skipped += 1;
            source_stats.duplicates_skipped += 1;
            continue;
        }
        source_stats.candidates_after_dedup += 1;
        if suppress_by_balance(
            source,
            context.balance_policy,
            buffers,
            source_stats,
            summary,
        ) {
            continue;
        }
        outcome.events_written += usize::from(!context.args.dry_run);
        source_stats.events_emitted += 1;
        buffers.raw_events.push(event.clone());
    }
    outcome
}

fn dedup_decision(
    args: &Args,
    buffers: &mut CrawlBuffers<'_>,
    event: &RawIntelEvent,
) -> DedupDecision {
    if args.dry_run {
        DedupDecision::New
    } else {
        buffers.dedup.decide_and_insert(event)
    }
}

fn suppress_by_balance(
    source: &Source,
    balance_policy: SourceBalancePolicy,
    buffers: &mut CrawlBuffers<'_>,
    source_stats: &mut SourceRunStats,
    summary: &mut CrawlSummary,
) -> bool {
    match buffers.balance_tracker.admit(source, balance_policy) {
        Admission::Admit => false,
        Admission::Suppress { reason } => {
            summary.events_suppressed_by_balance += 1;
            source_stats.record_suppression(reason);
            true
        }
    }
}
