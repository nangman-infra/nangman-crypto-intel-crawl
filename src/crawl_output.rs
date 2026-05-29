mod diagnostics;
mod manifest;
mod publish;

use self::diagnostics::write_diagnostic_objects;
use self::manifest::write_manifest;
use self::publish::{PublishOutcome, publish_stored_events};
use crate::crawl_loop::{CrawlBuffers, CrawlSummary};
use crate::publisher::EventPublisher;
use crate::registry::SourceRegistry;
use crate::storage::{IntelL0Storage, UploadedObject};
use chrono::Utc;
use std::error::Error;

pub(crate) async fn write_storage_outputs(
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

pub(super) fn run_id() -> String {
    format!("intel-crawl-{}", Utc::now().format("%Y%m%dT%H%M%S%fZ"))
}

pub(super) fn object_key(object: &UploadedObject) -> String {
    object.key().to_owned()
}
