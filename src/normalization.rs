mod fingerprint;
mod hashing;
mod text;
mod url;

pub(crate) use fingerprint::content_fingerprint;
pub(crate) use hashing::{hamming_distance, hash_hex, simhash64};
pub(crate) use text::normalize_text_for_dedup;
pub(crate) use url::canonicalize_url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContentFingerprint {
    pub(crate) canonical_url: String,
    pub(crate) canonical_url_hash: String,
    pub(crate) normalized_text_hash: String,
    pub(crate) simhash64: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_url_removes_tracking_and_fragment() {
        assert_eq!(
            canonicalize_url("HTTPS://Example.COM/a/?utm_source=x&b=2&a=1#section"),
            "https://example.com/a?a=1&b=2"
        );
    }

    #[test]
    fn normalized_text_ignores_case_and_punctuation() {
        assert_eq!(
            normalize_text_for_dedup("Binance lists TEST-token!"),
            "binance lists test token"
        );
    }

    #[test]
    fn simhash_is_stable_for_small_text_changes() {
        let left = simhash64(&normalize_text_for_dedup("binance lists test token today"));
        let right = simhash64(&normalize_text_for_dedup("binance lists test token today."));

        assert_eq!(hamming_distance(left, right), 0);
    }
}
