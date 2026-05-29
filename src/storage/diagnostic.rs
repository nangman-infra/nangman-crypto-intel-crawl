use super::IntelL0Storage;
use super::keys::{
    publish_outbox_object_key, source_balance_object_key, source_coverage_object_key,
    source_heal_object_key, source_health_object_key,
};
use super::model::UploadedObject;
use serde::Serialize;
use std::error::Error;

impl IntelL0Storage {
    pub(crate) async fn write_source_health<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_health",
            &source_health_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_source_heal<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_heal",
            &source_heal_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_source_coverage<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_coverage",
            &source_coverage_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_source_balance<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_balance",
            &source_balance_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_publish_outbox<T: Serialize>(
        &self,
        status: &str,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "publish_outbox",
            &publish_outbox_object_key(status, observed_at_ms, &self.run_id),
            records,
        )
        .await
    }
}
