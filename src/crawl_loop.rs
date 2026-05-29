use crate::balance::SourceBalancePolicy;
use crate::crawl_output::write_storage_outputs;
use chrono::Utc;
use std::error::Error;

mod buffers;
mod context;
mod events;
mod input;
mod source;
mod summary;

pub(crate) use buffers::CrawlBuffers;
use context::CrawlContext;
pub(crate) use input::{CrawlOnceInput, CrawlOutputs};
use source::crawl_source;
pub(crate) use summary::CrawlSummary;
pub(in crate::crawl_loop) use summary::{SourceFailure, SourceOutcome};

pub(crate) async fn crawl_once(input: CrawlOnceInput<'_>) -> Result<CrawlSummary, Box<dyn Error>> {
    let started_at_ms = Utc::now().timestamp_millis();
    let balance_policy = SourceBalancePolicy {
        derivatives_max_events_per_run: input.args.derivatives_max_events_per_run,
        derivatives_max_events_per_source: input.args.derivatives_max_events_per_source,
        community_max_events_per_run: input.args.community_max_events_per_run,
        community_max_events_per_source: input.args.community_max_events_per_source,
    };
    let mut summary = CrawlSummary::new(
        input.sources.len(),
        input.args.dry_run,
        input.publisher.is_enabled(),
    );
    let mut buffers = CrawlBuffers::new(input.outputs.dedup);
    let mut context = CrawlContext::new(
        input.args,
        input.registry,
        input.object_store,
        input.source_states,
        input.matcher,
        input.client,
        balance_policy,
    );

    for source in input.sources {
        crawl_source(&mut context, source, &mut buffers, &mut summary).await?;
    }

    if let Some(storage) = input.storage {
        write_storage_outputs(
            storage,
            input.publisher,
            input.registry,
            buffers,
            started_at_ms,
            &mut summary,
        )
        .await?;
    }

    Ok(summary)
}
