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
    let mut raw_events = Vec::new();
    let mut health_records = Vec::new();
    let mut heal_records = Vec::new();
    let mut balance_records = Vec::new();
    let mut balance_tracker = SourceBalanceTracker::default();

    for source in sources {
        let checked_at_ms = Utc::now().timestamp_millis();
        let mut source_stats = SourceRunStats::default();
        match fetch_source_items(
            client,
            registry,
            source,
            args.max_items_per_source,
            balance_policy,
            args.backfill_start_ms,
            args.backfill_end_ms,
        )
        .await
        {
            Ok(items) => {
                let fetched_at_ms = Utc::now().timestamp_millis();
                let source_items_seen = items.len();
                source_stats.items_seen = source_items_seen;
                summary.sources_ok += 1;
                summary.items_seen += source_items_seen;
                let mut source_events_written = 0;
                let mut source_duplicates_skipped = 0;
                for item in items {
                    let matched_assets = matcher.match_item(&item.title, &item.body, &item.url);
                    let event =
                        build_raw_intel_event(source, &item, &matched_assets, fetched_at_ms);
                    if !args.dry_run && !outputs.dedup.is_new(event.dedup_key()) {
                        summary.events_skipped_duplicate += 1;
                        source_duplicates_skipped += 1;
                        source_stats.duplicates_skipped += 1;
                        continue;
                    }
                    source_stats.candidates_after_dedup += 1;
                    match balance_tracker.admit(source, balance_policy) {
                        Admission::Admit => {}
                        Admission::Suppress { reason } => {
                            summary.events_suppressed_by_balance += 1;
                            source_stats.record_suppression(reason);
                            continue;
                        }
                    }
                    source_events_written += usize::from(!args.dry_run);
                    source_stats.events_emitted += 1;
                    raw_events.push(event);
                }
                let health = SourceHealthRecord::ok(
                    source,
                    checked_at_ms,
                    Utc::now().timestamp_millis(),
                    source_items_seen,
                    source_events_written,
                    source_duplicates_skipped,
                );
                health_records.push(health);
                balance_records.push(SourceBalanceRecord::new(
                    source,
                    source_stats,
                    balance_policy,
                    Utc::now().timestamp_millis(),
                ));
                summary.source_health_written += usize::from(!args.dry_run);
            }
            Err(error) => {
                let error = error.to_string();
                let observed_at_ms = Utc::now().timestamp_millis();
                summary.sources_failed += 1;
                summary.failures.push(SourceFailure {
                    source_id: source.source_id.clone(),
                    error: error.clone(),
                });
                let health = SourceHealthRecord::failed(
                    source,
                    checked_at_ms,
                    observed_at_ms,
                    error.clone(),
                );
                health_records.push(health);
                balance_records.push(SourceBalanceRecord::new(
                    source,
                    source_stats,
                    balance_policy,
                    observed_at_ms,
                ));
                summary.source_health_written += usize::from(!args.dry_run);
                let heal = SourceHealRecord::retry_after_failure(
                    source,
                    observed_at_ms,
                    &error,
                    args.schedule_interval_ms,
                );
                heal_records.push(heal);
                summary.source_heal_written += usize::from(!args.dry_run);
            }
        }
    }

    if let Some(storage) = storage {
        let mut uploaded_objects = Vec::new();
        let (stored_events, raw_uploaded) = storage.write_raw_events(&raw_events).await?;
        summary.events_written = stored_events.len();
        uploaded_objects.extend(raw_uploaded);

        let observed_at_ms = Utc::now().timestamp_millis();
        if let Some(object) = storage
            .write_source_health(&health_records, observed_at_ms)
            .await?
        {
            uploaded_objects.push(object);
        }
        if let Some(object) = storage
            .write_source_heal(&heal_records, observed_at_ms)
            .await?
        {
            uploaded_objects.push(object);
        }
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
            .write_source_balance(&balance_records, observed_at_ms)
            .await?
        {
            summary.source_balance_written += 1;
            summary.source_balance_key = Some(object_key(&object));
            uploaded_objects.push(object);
        }
        let PublishOutcome {
            dedup_events,
            publish_error,
            uploaded_objects: publish_uploaded_objects,
        } = publish_stored_events(storage, publisher, &stored_events, &mut summary).await?;
        uploaded_objects.extend(publish_uploaded_objects);
        if let Some(object) = storage
            .write_dedup_index(&dedup_events, observed_at_ms)
            .await?
        {
            uploaded_objects.push(object);
        }
        let object = storage
            .write_manifest(ManifestInput {
                status: if publish_error.is_none() {
                    "success"
                } else {
                    "publish_failed"
                }
                .to_owned(),
                started_at_ms,
                finished_at_ms: Utc::now().timestamp_millis(),
                uploaded_objects,
                raw_event_count: stored_events.len(),
                pointer_published_count: summary.events_published,
                pointer_pending_count: summary.pointer_publish_pending,
            })
            .await?;
        summary.manifest_written = 1;
        summary.manifest_key = Some(object_key(&object));
        if let Some(error) = publish_error {
            return Err(format!("NATS publish failed after RustFS upload: {error}").into());
        }
    }

    Ok(summary)
}

struct CrawlOutputs<'a> {
    dedup: &'a mut DedupStore,
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
    let mut dedup_events = Vec::new();
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
                dedup_events.push(stored.event.clone());
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
        dedup_events,
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
    dedup_events: Vec<RawIntelEvent>,
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
