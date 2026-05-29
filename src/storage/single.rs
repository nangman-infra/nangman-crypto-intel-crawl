use super::IntelL0Storage;
use super::jsonl::build_jsonl_chunk;
use super::model::UploadedObject;
use serde::Serialize;
use std::error::Error;

impl IntelL0Storage {
    pub(super) async fn write_single_jsonl_object<T: Serialize>(
        &self,
        object_family: &str,
        key: &str,
        records: &[T],
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        if records.is_empty() {
            return Ok(None);
        }
        let (bytes, _) = build_jsonl_chunk(records)?;
        let byte_count = bytes.len();
        self.object_store
            .put_bytes(key, bytes, "application/x-ndjson")
            .await?;
        Ok(Some(UploadedObject::new(
            object_family,
            key,
            records.len(),
            byte_count,
        )))
    }
}
