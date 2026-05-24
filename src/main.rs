mod args;
mod balance;
mod coverage;
mod dedup;
mod event;
mod fetch;
mod health;
mod html;
mod item;
mod normalization;
mod object_store;
mod publisher;
mod registry;
mod replay;
mod rest_api;
mod rss;
mod source_state;
mod storage;
mod symbols;

use args::{Args, DEFAULT_SOURCE_REGISTRY_PATH};
use balance::{
    Admission, SourceBalancePolicy, SourceBalanceRecord, SourceBalanceTracker, SourceRunStats,
};
use chrono::Utc;
use coverage::build_source_coverage_report;
use dedup::{DedupDecision, DedupStore};
use event::{RawIntelEvent, build_raw_intel_event, build_raw_intel_event_created_pointer};
use fetch::SourceFetchResult;
use health::{SourceHealRecord, SourceHealthRecord};
use item::FeedItem;
use object_store::ObjectStore;
use publisher::EventPublisher;
use registry::{Source, SourceRegistry};
use serde::Serialize;
use source_state::{SourceFetchStates, now_ms};
use std::error::Error;
use std::path::PathBuf;
use storage::{IntelL0Storage, ManifestInput, StoredRawIntelEvent, UploadedObject};
use symbols::SymbolMatcher;
use tokio::time::{Duration, sleep};

const SOURCE_FETCH_MAX_ATTEMPTS: usize = 3;
const SOURCE_FETCH_RETRY_BASE_MS: u64 = 750;
const HIGH_CADENCE_INTERVAL_MS: i64 = 15 * 60_000;
const MEDIUM_CADENCE_INTERVAL_MS: i64 = 30 * 60_000;
const LOW_CADENCE_INTERVAL_MS: i64 = 6 * 60 * 60_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let raw_args = std::env::args().collect::<Vec<_>>();
    if healthcheck_requested(&raw_args) {
        run_healthcheck(&raw_args).await?;
        return Ok(());
    }

    let args = match Args::parse(raw_args.into_iter()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let object_store = if args.dry_run {
        None
    } else {
        Some(ObjectStore::connect(args.object_store.clone()).await?)
    };
    let publisher = if args.dry_run {
        EventPublisher::Disabled
    } else {
        EventPublisher::connect(
            args.nats_url.as_deref(),
            &args.nats_subject,
            &args.nats_stream,
        )
        .await?
    };
    if args.replay_pending_outbox {
        let Some(object_store) = object_store.as_ref() else {
            return Err("--replay-pending-outbox requires object store access".into());
        };
        let summary = replay::replay_pending_outbox(object_store, &publisher).await?;
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    let registry = SourceRegistry::load(&args.source_registry).await?;
    if let Some(source_id) = args.source_id.as_deref() {
        registry.require_enabled_source(source_id)?;
    }
    let matcher = SymbolMatcher::new(&registry.universe_assets);
    let mut dedup = if let Some(object_store) = object_store.as_ref() {
        DedupStore::load_from_object_store(object_store, args.dedup_lookback_days).await?
    } else {
        DedupStore::default()
    };

    let client = reqwest::Client::builder()
        .user_agent("NangmanCryptoIntelCrawler/0.2 raw-intel")
        .http1_only()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;
    loop {
        let storage = object_store.as_ref().map(|object_store| {
            IntelL0Storage::new(object_store.clone(), run_id(), args.chunk_max_records)
        });
        let sources = registry.enabled_sources(args.source_id.as_deref());
        let mut source_states = SourceFetchStates::load(object_store.as_ref(), &sources).await?;
        let mut outputs = CrawlOutputs { dedup: &mut dedup };
        let summary = crawl_once(CrawlOnceInput {
            args: &args,
            registry: &registry,
            sources,
            object_store: object_store.as_ref(),
            source_states: &mut source_states,
            matcher: &matcher,
            client: &client,
            outputs: &mut outputs,
            publisher: &publisher,
            storage: storage.as_ref(),
        })
        .await?;
        println!("{}", serde_json::to_string_pretty(&summary)?);
        publisher.flush().await?;
        source_states.persist(object_store.as_ref()).await?;

        let Some(interval_ms) = args.schedule_interval_ms else {
            break;
        };
        sleep(Duration::from_millis(interval_ms)).await;
    }

    Ok(())
}

async fn run_healthcheck(raw_args: &[String]) -> Result<(), Box<dyn Error>> {
    let source_registry = healthcheck_source_registry(raw_args)?;
    SourceRegistry::load(&source_registry).await?;
    Ok(())
}

fn healthcheck_requested(raw_args: &[String]) -> bool {
    raw_args.iter().skip(1).any(|arg| arg == "--healthcheck")
}

fn healthcheck_source_registry(raw_args: &[String]) -> Result<PathBuf, String> {
    let mut source_registry = PathBuf::from(DEFAULT_SOURCE_REGISTRY_PATH);
    let mut index = 1;
    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "--healthcheck" => index += 1,
            "--source-registry" => {
                let Some(value) = raw_args.get(index + 1) else {
                    return Err("--source-registry requires an absolute path".to_owned());
                };
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err("--source-registry requires an absolute path".to_owned());
                }
                source_registry = path;
                index += 2;
            }
            other => return Err(format!("unsupported healthcheck argument: {other}")),
        }
    }
    Ok(source_registry)
}

async fn crawl_once(input: CrawlOnceInput<'_>) -> Result<CrawlSummary, Box<dyn Error>> {
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
    let mut context = CrawlContext {
        args: input.args,
        registry: input.registry,
        object_store: input.object_store,
        source_states: input.source_states,
        matcher: input.matcher,
        client: input.client,
        balance_policy,
    };

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

async fn crawl_source(
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
        cache_headers: context
            .source_states
            .get(source)
            .map(|state| state.cache_headers()),
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

async fn record_source_items(
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

fn record_source_not_modified(
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

fn build_candidate_events(
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

fn append_source_events(
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

fn record_source_failure(
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

fn should_skip_source_for_backoff(
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

fn should_skip_source_for_cadence(
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

fn cadence_interval_ms(source: &Source) -> i64 {
    match source.cadence_tier.as_str() {
        "high" => HIGH_CADENCE_INTERVAL_MS,
        "medium" => MEDIUM_CADENCE_INTERVAL_MS,
        "low" => LOW_CADENCE_INTERVAL_MS,
        _ => HIGH_CADENCE_INTERVAL_MS,
    }
}

async fn write_storage_outputs(
    storage: &IntelL0Storage,
    publisher: &EventPublisher,
    registry: &SourceRegistry,
    buffers: CrawlBuffers<'_>,
    started_at_ms: i64,
    summary: &mut CrawlSummary,
) -> Result<(), Box<dyn Error>> {
    let mut uploaded_objects = Vec::new();
    let (stored_events, raw_uploaded) = storage.write_raw_events(&buffers.raw_events).await?;
    summary.events_written = stored_events.len();
    uploaded_objects.extend(raw_uploaded);

    let observed_at_ms = Utc::now().timestamp_millis();
    write_diagnostic_objects(
        storage,
        registry,
        &buffers,
        observed_at_ms,
        summary,
        &mut uploaded_objects,
    )
    .await?;
    let PublishOutcome {
        persisted_events,
        publish_error,
        uploaded_objects: publish_uploaded_objects,
    } = publish_stored_events(storage, publisher, &stored_events, summary).await?;
    uploaded_objects.extend(publish_uploaded_objects);
    uploaded_objects.extend(
        storage
            .write_dedup_index(&persisted_events, observed_at_ms)
            .await?,
    );
    write_manifest(
        storage,
        started_at_ms,
        stored_events.len(),
        publish_error,
        uploaded_objects,
        summary,
    )
    .await
}

async fn write_diagnostic_objects(
    storage: &IntelL0Storage,
    registry: &SourceRegistry,
    buffers: &CrawlBuffers<'_>,
    observed_at_ms: i64,
    summary: &mut CrawlSummary,
    uploaded_objects: &mut Vec<UploadedObject>,
) -> Result<(), Box<dyn Error>> {
    push_optional_object(
        uploaded_objects,
        storage
            .write_source_health(&buffers.health_records, observed_at_ms)
            .await?,
    );
    push_optional_object(
        uploaded_objects,
        storage
            .write_source_heal(&buffers.heal_records, observed_at_ms)
            .await?,
    );
    let coverage_records = build_source_coverage_report(registry, observed_at_ms);
    if let Some(object) = storage
        .write_source_coverage(&coverage_records, observed_at_ms)
        .await?
    {
        summary.source_coverage_written += 1;
        summary.source_coverage_key = Some(object_key(&object));
        uploaded_objects.push(object);
    }
    if let Some(object) = storage
        .write_source_balance(&buffers.balance_records, observed_at_ms)
        .await?
    {
        summary.source_balance_written += 1;
        summary.source_balance_key = Some(object_key(&object));
        uploaded_objects.push(object);
    }
    Ok(())
}

fn push_optional_object(
    uploaded_objects: &mut Vec<UploadedObject>,
    object: Option<UploadedObject>,
) {
    if let Some(object) = object {
        uploaded_objects.push(object);
    }
}

async fn write_manifest(
    storage: &IntelL0Storage,
    started_at_ms: i64,
    raw_event_count: usize,
    publish_error: Option<String>,
    uploaded_objects: Vec<UploadedObject>,
    summary: &mut CrawlSummary,
) -> Result<(), Box<dyn Error>> {
    let object = storage
        .write_manifest(ManifestInput {
            status: manifest_status(&publish_error).to_owned(),
            started_at_ms,
            finished_at_ms: Utc::now().timestamp_millis(),
            uploaded_objects,
            raw_event_count,
            pointer_published_count: summary.events_published,
            pointer_pending_count: summary.pointer_publish_pending,
        })
        .await?;
    summary.manifest_written = 1;
    summary.manifest_key = Some(object_key(&object));
    if let Some(error) = publish_error {
        return Err(format!("NATS publish failed after S3 upload: {error}").into());
    }
    Ok(())
}

fn manifest_status(publish_error: &Option<String>) -> &'static str {
    if publish_error.is_none() {
        "success"
    } else {
        "publish_failed"
    }
}

struct CrawlOutputs<'a> {
    dedup: &'a mut DedupStore,
}

struct CrawlOnceInput<'a> {
    args: &'a Args,
    registry: &'a SourceRegistry,
    sources: Vec<&'a Source>,
    object_store: Option<&'a ObjectStore>,
    source_states: &'a mut SourceFetchStates,
    matcher: &'a SymbolMatcher,
    client: &'a reqwest::Client,
    outputs: &'a mut CrawlOutputs<'a>,
    publisher: &'a EventPublisher,
    storage: Option<&'a IntelL0Storage>,
}

struct CrawlContext<'a> {
    args: &'a Args,
    registry: &'a SourceRegistry,
    object_store: Option<&'a ObjectStore>,
    source_states: &'a mut SourceFetchStates,
    matcher: &'a SymbolMatcher,
    client: &'a reqwest::Client,
    balance_policy: SourceBalancePolicy,
}

struct CrawlBuffers<'a> {
    dedup: &'a mut DedupStore,
    raw_events: Vec<RawIntelEvent>,
    health_records: Vec<SourceHealthRecord>,
    heal_records: Vec<SourceHealRecord>,
    balance_records: Vec<SourceBalanceRecord>,
    balance_tracker: SourceBalanceTracker,
}

impl<'a> CrawlBuffers<'a> {
    fn new(dedup: &'a mut DedupStore) -> Self {
        Self {
            dedup,
            raw_events: Vec::new(),
            health_records: Vec::new(),
            heal_records: Vec::new(),
            balance_records: Vec::new(),
            balance_tracker: SourceBalanceTracker::default(),
        }
    }
}

#[derive(Debug, Default)]
struct SourceOutcome {
    events_written: usize,
    duplicates_skipped: usize,
}

struct SourceFetchRequest<'a> {
    client: &'a reqwest::Client,
    registry: &'a SourceRegistry,
    source: &'a Source,
    cache_headers: Option<fetch::CacheHeaders>,
    default_max_items: usize,
    balance_policy: SourceBalancePolicy,
    backfill_start_ms: Option<i64>,
    backfill_end_ms: Option<i64>,
    selection_time_ms: i64,
}

async fn fetch_source_items(
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

async fn publish_stored_events(
    storage: &IntelL0Storage,
    publisher: &EventPublisher,
    stored_events: &[StoredRawIntelEvent],
    summary: &mut CrawlSummary,
) -> Result<PublishOutcome, Box<dyn Error>> {
    if stored_events.is_empty() {
        return Ok(PublishOutcome::default());
    }

    let mut published = Vec::new();
    let mut pending = Vec::new();
    let mut uploaded_objects = Vec::new();
    let mut first_error: Option<String> = None;
    for stored in stored_events {
        let pointer = build_raw_intel_event_created_pointer(
            &stored.event,
            stored.storage_ref.clone(),
            Utc::now().timestamp_millis(),
        );
        match publisher.publish(pointer.event_id(), &pointer).await {
            Ok(()) => {
                if publisher.is_enabled() {
                    summary.events_published += 1;
                    published.push(pointer);
                }
            }
            Err(error) => {
                if publisher.is_enabled() {
                    summary.pointer_publish_pending += 1;
                    pending.push(pointer);
                }
                if first_error.is_none() {
                    first_error = Some(error.to_string());
                }
            }
        }
    }

    let persisted_events = stored_events
        .iter()
        .map(|stored| stored.event.clone())
        .collect();
    let observed_at_ms = Utc::now().timestamp_millis();
    if let Some(object) = storage
        .write_publish_outbox("published", &published, observed_at_ms)
        .await?
    {
        summary.outbox_published_written += 1;
        summary.outbox_published_key = Some(object_key(&object));
        uploaded_objects.push(object);
    }
    if let Some(object) = storage
        .write_publish_outbox("pending", &pending, observed_at_ms)
        .await?
    {
        summary.outbox_pending_written += 1;
        summary.outbox_pending_key = Some(object_key(&object));
        uploaded_objects.push(object);
    }

    Ok(PublishOutcome {
        persisted_events,
        publish_error: first_error,
        uploaded_objects,
    })
}

fn object_key(object: &UploadedObject) -> String {
    object.key().to_owned()
}

fn run_id() -> String {
    format!("intel-crawl-{}", Utc::now().format("%Y%m%dT%H%M%S%fZ"))
}

#[derive(Debug, Default)]
struct PublishOutcome {
    persisted_events: Vec<RawIntelEvent>,
    publish_error: Option<String>,
    uploaded_objects: Vec<UploadedObject>,
}

#[derive(Debug, Serialize)]
struct CrawlSummary {
    dry_run: bool,
    nats_publish_enabled: bool,
    sources_selected: usize,
    sources_ok: usize,
    sources_failed: usize,
    items_seen: usize,
    events_written: usize,
    events_published: usize,
    pointer_publish_pending: usize,
    events_skipped_duplicate: usize,
    events_suppressed_by_balance: usize,
    source_health_written: usize,
    source_heal_written: usize,
    source_coverage_written: usize,
    source_balance_written: usize,
    outbox_published_written: usize,
    outbox_pending_written: usize,
    manifest_written: usize,
    source_coverage_key: Option<String>,
    source_balance_key: Option<String>,
    outbox_published_key: Option<String>,
    outbox_pending_key: Option<String>,
    manifest_key: Option<String>,
    failures: Vec<SourceFailure>,
}

impl CrawlSummary {
    fn new(sources_selected: usize, dry_run: bool, nats_publish_enabled: bool) -> Self {
        Self {
            dry_run,
            nats_publish_enabled,
            sources_selected,
            sources_ok: 0,
            sources_failed: 0,
            items_seen: 0,
            events_written: 0,
            events_published: 0,
            pointer_publish_pending: 0,
            events_skipped_duplicate: 0,
            events_suppressed_by_balance: 0,
            source_health_written: 0,
            source_heal_written: 0,
            source_coverage_written: 0,
            source_balance_written: 0,
            outbox_published_written: 0,
            outbox_pending_written: 0,
            manifest_written: 0,
            source_coverage_key: None,
            source_balance_key: None,
            outbox_published_key: None,
            outbox_pending_key: None,
            manifest_key: None,
            failures: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct SourceFailure {
    source_id: String,
    error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::AppliesToAssets;

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

    #[test]
    fn detects_healthcheck_mode() {
        assert!(healthcheck_requested(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned()
        ]));
        assert!(!healthcheck_requested(&["intel-crawl-app".to_owned()]));
    }

    #[test]
    fn healthcheck_uses_default_registry_path() {
        let source_registry = healthcheck_source_registry(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned(),
        ])
        .unwrap();

        assert_eq!(source_registry, PathBuf::from(DEFAULT_SOURCE_REGISTRY_PATH));
    }

    #[test]
    fn healthcheck_accepts_explicit_absolute_registry_path() {
        let source_registry = healthcheck_source_registry(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned(),
            "--source-registry".to_owned(),
            "/tmp/source-registry.json".to_owned(),
        ])
        .unwrap();

        assert_eq!(source_registry, PathBuf::from("/tmp/source-registry.json"));
    }

    #[test]
    fn healthcheck_rejects_relative_registry_path() {
        let error = healthcheck_source_registry(&[
            "intel-crawl-app".to_owned(),
            "--healthcheck".to_owned(),
            "--source-registry".to_owned(),
            "source-registry.json".to_owned(),
        ])
        .unwrap_err();

        assert!(error.contains("--source-registry requires an absolute path"));
    }

    #[test]
    fn cadence_intervals_match_source_tiers() {
        assert_eq!(cadence_interval_ms(&source("high")), 15 * 60_000);
        assert_eq!(cadence_interval_ms(&source("medium")), 30 * 60_000);
        assert_eq!(cadence_interval_ms(&source("low")), 6 * 60 * 60_000);
    }

    fn source(cadence_tier: &str) -> Source {
        Source {
            source_id: "source".to_owned(),
            source_category: "news".to_owned(),
            source_name: "Source".to_owned(),
            source_url: "https://example.com/feed.xml".to_owned(),
            fetch_method: "rss".to_owned(),
            adapter: None,
            max_items_per_run: None,
            trust_tier: "T1".to_owned(),
            cadence_tier: cadence_tier.to_owned(),
            language_hint: "en".to_owned(),
            enabled: true,
            source_state: None,
            activation_blocker: None,
            top50_relevance_mode: "symbol_alias_match".to_owned(),
            applies_to_assets: AppliesToAssets::All("all_major_50".to_owned()),
        }
    }
}
