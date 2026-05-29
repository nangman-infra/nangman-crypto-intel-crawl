use sha2::{Digest, Sha256};

use super::SOURCE_FETCH_STATE_SCHEMA;

pub(super) fn state_object_key(source_id: &str) -> String {
    format!(
        "source-fetch-state/schema={SOURCE_FETCH_STATE_SCHEMA}/source_id={}/state.json",
        path_segment(source_id)
    )
}

pub(super) fn state_id(source_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_id.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .chars()
        .take(24)
        .collect::<String>();
    format!("source_fetch_state_{suffix}")
}

fn path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
