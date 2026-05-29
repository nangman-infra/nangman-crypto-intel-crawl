mod decision_flow;
mod load;

use std::collections::{HashMap, HashSet};

pub(super) const NEAR_DUPLICATE_HAMMING_THRESHOLD: u32 = 4;

#[derive(Debug, Default)]
pub(crate) struct DedupStore {
    legacy_keys: HashSet<String>,
    exact_source_keys: HashMap<String, KnownEvent>,
    canonical_url_hashes: HashMap<String, String>,
    normalized_content_hashes: HashMap<String, String>,
    simhashes: Vec<KnownSimhash>,
    loaded_v2_prefixes: HashSet<String>,
}

#[derive(Debug, Clone)]
struct KnownEvent {
    event_id: String,
    normalized_content_hash: String,
}

#[derive(Debug, Clone)]
struct KnownSimhash {
    event_id: String,
    simhash64: u64,
}
