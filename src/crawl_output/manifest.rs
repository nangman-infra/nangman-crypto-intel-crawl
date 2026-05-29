use super::object_key;
use crate::crawl_loop::CrawlSummary;
use crate::storage::{IntelL0Storage, ManifestInput, UploadedObject};
use chrono::Utc;
use std::error::Error;

pub(super) async fn write_manifest(
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
