mod args;
mod balance;
mod coverage;
mod dedup;
mod event;
mod health;
mod html;
mod item;
mod object_store;
mod publisher;
mod registry;
mod rest_api;
mod rss;
mod storage;
mod symbols;

use args::Args;
use balance::{
    Admission, SourceBalancePolicy, SourceBalanceRecord, SourceBalanceTracker, SourceRunStats,
};
use chrono::Utc;
use coverage::build_source_coverage_report;
use dedup::DedupStore;
use event::{RawIntelEvent, build_raw_intel_event, build_raw_intel_event_created_pointer};
use health::{SourceHealRecord, SourceHealthRecord};
use item::FeedItem;
use object_store::ObjectStore;
use publisher::EventPublisher;
use registry::{Source, SourceRegistry};
use serde::Serialize;
use std::error::Error;
use storage::{IntelL0Storage, ManifestInput, StoredRawIntelEvent, UploadedObject};
use symbols::SymbolMatcher;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = match Args::parse(std::env::args()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let registry = SourceRegistry::load(&args.source_registry).await?;
    if let Some(source_id) = args.source_id.as_deref() {
        registry.require_enabled_source(source_id)?;
    }
    let matcher = SymbolMatcher::new(&registry.universe_assets);
    let object_store = if args.dry_run {
        None
    } else {
        Some(ObjectStore::connect(args.object_store.clone()).await?)
    };
    let mut dedup = if let Some(object_store) = object_store.as_ref() {
        DedupStore::load_from_object_store(object_store, args.dedup_lookback_days).await?
    } else {
        DedupStore::default()
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
        let mut outputs = CrawlOutputs { dedup: &mut dedup };
        let summary = crawl_once(
            &args,
            &registry,
            &matcher,
            &client,
            &mut outputs,
            &publisher,
            storage.as_ref(),
        )
        .await?;
        println!("{}", serde_json::to_string_pretty(&summary)?);
        publisher.flush().await?;

        let Some(interval_ms) = args.schedule_interval_ms else {
            break;
        };
        sleep(Duration::from_millis(interval_ms)).await;
    }

    Ok(())
}

async fn crawl_once(
    args: &Args,
    registry: &SourceRegistry,
    matcher: &SymbolMatcher,
    client: &reqwest::Client,
    outputs: &mut CrawlOutputs<'_>,
    publisher: &EventPublisher,
    storage: Option<&IntelL0Storage>,
) -> Result<CrawlSummary, Box<dyn Error>> {
    let started_at_ms = Utc::now().timestamp_millis();
    let balance_policy = SourceBalancePolicy {
        derivatives_max_events_per_run: args.derivatives_max_events_per_run,
        derivatives_max_events_per_source: args.derivatives_max_events_per_source,
        community_max_events_per_run: args.community_max_events_per_run,
        community_max_events_per_source: args.community_max_events_per_source,
    };
    let sources = registry.enabled_sources(args.source_id.as_deref());
    let mut summary = CrawlSummary::new(sources.len(), args.dry_run, publisher.is_enabled());
    let mut buffers = CrawlBuffers::new(outputs.dedup);
    let context = CrawlContext {
        args,
        registry,
        matcher,
        client,
        balance_policy,
    };

    for source in sources {
        crawl_source(&context, source, &mut buffers, &mut summary).await?;
    }

    if let Some(storage) = storage {
        write_storage_outputs(
            storage,
            publisher,
            registry,
            buffers,
            started_at_ms,
            &mut summary,
        )
        .await?;
    }

    Ok(summary)
}

async fn crawl_source(
    context: &CrawlContext<'_>,
    source: &Source,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) -> Result<(), Box<dyn Error>> {
    let checked_at_ms = Utc::now().timestamp_millis();
    let mut source_stats = SourceRunStats::default();
    let result = fetch_source_items(
        context.client,
        context.registry,
        source,
        context.args.max_items_per_source,
        context.balance_policy,
        context.args.backfill_start_ms,
        context.args.backfill_end_ms,
    )
    .await;

    match result {
        Ok(items) => record_source_items(
            context,
            source,
            checked_at_ms,
            items,
            &mut source_stats,
            buffers,
            summary,
        ),
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

fn record_source_items(
    context: &CrawlContext<'_>,
    source: &Source,
    checked_at_ms: i64,
    items: Vec<FeedItem>,
    source_stats: &mut SourceRunStats,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) {
    let fetched_at_ms = Utc::now().timestamp_millis();
    let source_items_seen = items.len();
    source_stats.items_seen = source_items_seen;
    summary.sources_ok += 1;
    summary.items_seen += source_items_seen;
    let source_outcome = append_source_events(
        context,
        source,
        items,
        fetched_at_ms,
        source_stats,
        buffers,
        summary,
    );
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
}

fn append_source_events(
    context: &CrawlContext<'_>,
    source: &Source,
    items: Vec<FeedItem>,
    fetched_at_ms: i64,
    source_stats: &mut SourceRunStats,
    buffers: &mut CrawlBuffers<'_>,
    summary: &mut CrawlSummary,
) -> SourceOutcome {
    let mut outcome = SourceOutcome::default();
    for item in items {
        let matched_assets = context
            .matcher
            .match_item(&item.title, &item.body, &item.url);
        let event = build_raw_intel_event(source, &item, &matched_assets, fetched_at_ms);
        if is_duplicate_event(context.args, buffers, event.dedup_key()) {
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
        buffers.raw_events.push(event);
    }
    outcome
}

fn is_duplicate_event(args: &Args, buffers: &mut CrawlBuffers<'_>, dedup_key: &str) -> bool {
    !args.dry_run && !buffers.dedup.is_new(dedup_key)
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
    context: &CrawlContext<'_>,
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
    if let Some(object) = storage
        .write_dedup_index(&persisted_events, observed_at_ms)
        .await?
    {
        uploaded_objects.push(object);
    }
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
        return Err(format!("NATS publish failed after RustFS upload: {error}").into());
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

struct CrawlContext<'a> {
    args: &'a Args,
    registry: &'a SourceRegistry,
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

async fn fetch_source_items(
    client: &reqwest::Client,
    registry: &SourceRegistry,
    source: &Source,
    default_max_items: usize,
    balance_policy: SourceBalancePolicy,
    backfill_start_ms: Option<i64>,
    backfill_end_ms: Option<i64>,
) -> Result<Vec<FeedItem>, Box<dyn Error>> {
    let item_limit =
        balance_policy.effective_item_limit(source, source.item_limit(default_max_items));
    match source.fetch_method.as_str() {
        "rss" => rss::fetch_feed_items(client, source, item_limit).await,
        "rest_api" => {
            rest_api::fetch_feed_items(
                client,
                source,
                &registry.universe_assets,
                item_limit,
                backfill_start_ms,
                backfill_end_ms,
            )
            .await
        }
        "html_crawl" => html::fetch_feed_items(client, source, item_limit).await,
        other => Err(format!("{} unsupported fetch_method {other}", source.source_id).into()),
    }
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
