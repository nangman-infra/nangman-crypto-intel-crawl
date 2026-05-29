use super::{DedupStore, KnownEvent, KnownSimhash, NEAR_DUPLICATE_HAMMING_THRESHOLD};
use crate::dedup::decision::DedupDecision;
use crate::event::RawIntelEvent;
use crate::normalization::hamming_distance;

impl DedupStore {
    pub(crate) fn decide_and_insert(&mut self, event: &RawIntelEvent) -> DedupDecision {
        if self.legacy_keys.contains(event.dedup_key()) {
            return DedupDecision::ExactDuplicate {
                duplicate_of_event_id: None,
            };
        }

        if let Some(known) = self.exact_source_keys.get(event.exact_source_key()) {
            if known.normalized_content_hash == event.normalized_content_hash() {
                return DedupDecision::ExactDuplicate {
                    duplicate_of_event_id: Some(known.event_id.clone()),
                };
            }
            let duplicate_of_event_id = Some(known.event_id.clone());
            self.insert_event(event);
            return DedupDecision::UpdateOfExisting {
                duplicate_of_event_id,
            };
        }

        if let Some(event_id) = self
            .normalized_content_hashes
            .get(event.normalized_content_hash())
        {
            return DedupDecision::ContentDuplicate {
                duplicate_of_event_id: Some(event_id.clone()),
            };
        }

        if let Some(known) = self.near_duplicate(event.simhash64_value()) {
            return DedupDecision::NearDuplicate {
                duplicate_of_event_id: Some(known.event_id.clone()),
            };
        }

        self.insert_event(event);
        DedupDecision::New
    }

    fn near_duplicate(&self, simhash64: u64) -> Option<&KnownSimhash> {
        if simhash64 == 0 {
            return None;
        }
        self.simhashes.iter().find(|known| {
            known.simhash64 != 0
                && hamming_distance(known.simhash64, simhash64) <= NEAR_DUPLICATE_HAMMING_THRESHOLD
        })
    }

    fn insert_event(&mut self, event: &RawIntelEvent) {
        self.legacy_keys.insert(event.dedup_key().to_owned());
        self.exact_source_keys.insert(
            event.exact_source_key().to_owned(),
            KnownEvent {
                event_id: event.event_id().to_owned(),
                normalized_content_hash: event.normalized_content_hash().to_owned(),
            },
        );
        self.canonical_url_hashes.insert(
            event.canonical_url_hash().to_owned(),
            event.event_id().to_owned(),
        );
        self.normalized_content_hashes.insert(
            event.normalized_content_hash().to_owned(),
            event.event_id().to_owned(),
        );
        self.simhashes.push(KnownSimhash {
            event_id: event.event_id().to_owned(),
            simhash64: event.simhash64_value(),
        });
    }
}
