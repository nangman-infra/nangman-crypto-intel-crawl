use super::*;

#[test]
fn raw_key_uses_partitioned_prefix() {
    let partition = RawPartition {
        date: "2026-05-07".to_owned(),
        hour: 19,
        source_category: "news".to_owned(),
        source_id: "coindesk_rss".to_owned(),
    };

    assert_eq!(
        raw_object_key(&partition, "intel-crawl-1", 2),
        "raw-intel-events/schema=raw_intel_event_v1/dt=2026-05-07/hour=19/source_category=news/source_id=coindesk_rss/run_id=intel-crawl-1/part-000002.jsonl"
    );
}
