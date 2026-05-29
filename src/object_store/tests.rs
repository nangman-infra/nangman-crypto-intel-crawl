use super::config::ObjectStoreConfig;
use super::validation::{validate_object_key, validate_object_prefix};

#[test]
fn rejects_empty_config_values() {
    let error = ObjectStoreConfig {
        bucket: "".to_owned(),
        region: "us-east-1".to_owned(),
    }
    .validate()
    .unwrap_err()
    .to_string();

    assert!(error.contains("bucket"));
}

#[test]
fn rejects_bucket_placeholder() {
    let error = ObjectStoreConfig {
        bucket: "<bucket-name>".to_owned(),
        region: "ap-northeast-2".to_owned(),
    }
    .validate()
    .unwrap_err()
    .to_string();

    assert!(error.contains("placeholder"));
}

#[test]
fn accepts_valid_bucket_config() {
    ObjectStoreConfig {
        bucket: "nangman-intel-crawl-l0-dev".to_owned(),
        region: "ap-northeast-2".to_owned(),
    }
    .validate()
    .unwrap();
}

#[test]
fn rejects_invalid_bucket_names() {
    for bucket in [
        "ab",
        "NangmanBucket",
        "nangman_bucket",
        "nangman..bucket",
        "nangman-.bucket",
        "192.168.5.4",
        "xn--nangman-bucket",
        "nangman-bucket--x-s3",
    ] {
        let error = ObjectStoreConfig {
            bucket: bucket.to_owned(),
            region: "us-east-1".to_owned(),
        }
        .validate()
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("bucket"),
            "expected bucket validation error for {bucket}, got {error}"
        );
    }
}

#[test]
fn rejects_invalid_region_values() {
    let error = ObjectStoreConfig {
        bucket: "nangman-intel-crawl-l0-dev".to_owned(),
        region: " ap-northeast-2".to_owned(),
    }
    .validate()
    .unwrap_err()
    .to_string();

    assert!(error.contains("region"));
}

#[test]
fn validates_object_keys_before_requests() {
    validate_object_key("raw-intel-event/schema=v1/dt=2026-05-29/part-000001.jsonl").unwrap();

    for key in [
        "",
        "./state.json",
        "state/../next.json",
        "state/\n/next.json",
    ] {
        let error = validate_object_key(key).unwrap_err().to_string();
        assert!(
            error.contains("key"),
            "expected key validation error for {key:?}, got {error}"
        );
    }
}

#[test]
fn validates_list_prefixes_before_requests() {
    validate_object_prefix("").unwrap();
    validate_object_prefix("dedup-index/schema=dedup_index_v1/dt=2026-05-29/").unwrap();

    let error = validate_object_prefix("dedup-index/../raw/")
        .unwrap_err()
        .to_string();
    assert!(error.contains("prefix"));
}
