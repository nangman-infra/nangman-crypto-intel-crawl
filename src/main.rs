mod args;
mod balance;
mod coverage;
mod crawl_loop;
mod crawl_output;
mod dedup;
mod event;
mod fetch;
mod health;
mod healthcheck;
mod html;
mod item;
mod normalization;
mod object_store;
mod publisher;
mod registry;
mod replay;
mod rest_api;
mod rss;
mod source_fetch;
mod source_schedule;
mod source_state;
mod storage;
mod symbols;

use args::Args;
use crawl_loop::{CrawlOnceInput, CrawlOutputs, crawl_once};
use crawl_output::run_id;
use dedup::DedupStore;
use healthcheck::{healthcheck_requested, run_healthcheck};
use object_store::ObjectStore;
use publisher::EventPublisher;
use registry::SourceRegistry;
use source_state::SourceFetchStates;
use std::error::Error;
use storage::IntelL0Storage;
use symbols::SymbolMatcher;
use tokio::time::{Duration, sleep};

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
