use super::object_key;
use crate::crawl_loop::CrawlSummary;
use crate::event::{RawIntelEvent, build_raw_intel_event_created_pointer};
use crate::publisher::EventPublisher;
use crate::storage::{IntelL0Storage, StoredRawIntelEvent, UploadedObject};
use chrono::Utc;
use std::error::Error;

pub(super) async fn publish_stored_events(
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

#[derive(Debug, Default)]
pub(super) struct PublishOutcome {
    pub(super) persisted_events: Vec<RawIntelEvent>,
    pub(super) publish_error: Option<String>,
    pub(super) uploaded_objects: Vec<UploadedObject>,
}
