use super::IntelL0Storage;
use super::keys::manifest_object_key;
use super::model::{IntelL0Manifest, MANIFEST_SCHEMA, ManifestInput, UploadedObject};
use std::error::Error;

impl IntelL0Storage {
    pub(crate) async fn write_manifest(
        &self,
        input: ManifestInput,
    ) -> Result<UploadedObject, Box<dyn Error>> {
        let key = manifest_object_key(input.started_at_ms, &self.run_id);
        let manifest = IntelL0Manifest {
            schema_version: MANIFEST_SCHEMA.to_owned(),
            run_id: self.run_id.clone(),
            status: input.status,
            started_at_ms: input.started_at_ms,
            finished_at_ms: input.finished_at_ms,
            raw_event_count: input.raw_event_count,
            pointer_published_count: input.pointer_published_count,
            pointer_pending_count: input.pointer_pending_count,
            uploaded_object_count: input.uploaded_objects.len(),
            uploaded_objects: input.uploaded_objects,
        };
        let bytes = serde_json::to_vec_pretty(&manifest)?;
        let byte_count = bytes.len();
        self.object_store
            .put_bytes(&key, bytes, "application/json")
            .await?;
        Ok(UploadedObject::new("manifest", key, 1, byte_count))
    }
}
