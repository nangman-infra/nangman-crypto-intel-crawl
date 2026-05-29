use super::object_key;
use crate::coverage::build_source_coverage_report;
use crate::crawl_loop::{CrawlBuffers, CrawlSummary};
use crate::registry::SourceRegistry;
use crate::storage::{IntelL0Storage, UploadedObject};
use std::error::Error;

pub(super) async fn write_diagnostic_objects(
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
