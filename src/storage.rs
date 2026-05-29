use crate::object_store::ObjectStore;

mod dedup;
mod diagnostic;
mod jsonl;
mod keys;
mod manifest;
mod model;
mod raw;
mod single;

pub(crate) use model::{ManifestInput, StoredRawIntelEvent, UploadedObject};

pub(crate) struct IntelL0Storage {
    object_store: ObjectStore,
    run_id: String,
    chunk_max_records: usize,
}

impl IntelL0Storage {
    pub(crate) fn new(object_store: ObjectStore, run_id: String, chunk_max_records: usize) -> Self {
        Self {
            object_store,
            run_id,
            chunk_max_records: chunk_max_records.max(1),
        }
    }
}
