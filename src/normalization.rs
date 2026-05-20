use sha2::{Digest, Sha256};

const TRACKING_QUERY_PREFIXES: &[&str] = &["utm_"];
const TRACKING_QUERY_KEYS: &[&str] = &[
    "fbclid", "gclid", "mc_cid", "mc_eid", "igshid", "ref", "ref_src",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContentFingerprint {
    pub(crate) canonical_url: String,
    pub(crate) canonical_url_hash: String,
    pub(crate) normalized_text_hash: String,
    pub(crate) simhash64: u64,
}

pub(crate) fn content_fingerprint(title: &str, body: &str, url: &str) -> ContentFingerprint {
    let canonical_url = canonicalize_url(url);
    let normalized_text = normalize_text_for_dedup(&format!("{title}\n{body}"));
    ContentFingerprint {
        canonical_url_hash: hash_hex(&canonical_url),
        normalized_text_hash: hash_hex(&normalized_text),
        simhash64: simhash64(&normalized_text),
        canonical_url,
    }
}

pub(crate) fn canonicalize_url(url: &str) -> String {
    let without_fragment = url.split('#').next().unwrap_or(url).trim();
    let (base, query) = without_fragment
        .split_once('?')
        .map_or((without_fragment, ""), |(base, query)| (base, query));
    let normalized_base = normalize_base_url(base);
    let normalized_query = normalize_query(query);
    if normalized_query.is_empty() {
        normalized_base
    } else {
        format!("{normalized_base}?{normalized_query}")
    }
}

pub(crate) fn normalize_text_for_dedup(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch.to_lowercase().collect::<String>()
            } else {
                " ".to_owned()
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn hash_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn simhash64(normalized_text: &str) -> u64 {
    let tokens = normalized_text.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return 0;
    }
    let features = shingles(&tokens);
    let mut weights = [0i32; 64];
    for feature in features {
        let hash = first_u64_hash(&feature);
        for (bit, weight) in weights.iter_mut().enumerate() {
            if hash & (1u64 << bit) == 0 {
                *weight -= 1;
            } else {
                *weight += 1;
            }
        }
    }
    weights.iter().enumerate().fold(0u64, |acc, (bit, weight)| {
        if *weight > 0 {
            acc | (1u64 << bit)
        } else {
            acc
        }
    })
}

pub(crate) fn hamming_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

fn normalize_base_url(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return trimmed.to_owned();
    };
    let Some((host, path)) = rest.split_once('/') else {
        return format!(
            "{}://{}",
            scheme.to_ascii_lowercase(),
            rest.to_ascii_lowercase()
        );
    };
    format!(
        "{}://{}/{}",
        scheme.to_ascii_lowercase(),
        host.to_ascii_lowercase(),
        path
    )
}

fn normalize_query(query: &str) -> String {
    let mut pairs = query
        .split('&')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            let key = trimmed.split('=').next().unwrap_or("").to_ascii_lowercase();
            if is_tracking_query_key(&key) {
                return None;
            }
            Some(trimmed.to_owned())
        })
        .collect::<Vec<_>>();
    pairs.sort();
    pairs.join("&")
}

fn is_tracking_query_key(key: &str) -> bool {
    TRACKING_QUERY_KEYS.contains(&key)
        || TRACKING_QUERY_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix))
}

fn shingles(tokens: &[&str]) -> Vec<String> {
    if tokens.len() < 4 {
        return tokens.iter().map(|token| (*token).to_owned()).collect();
    }
    tokens
        .windows(4)
        .map(|window| window.join(" "))
        .collect::<Vec<_>>()
}

fn first_u64_hash(value: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
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
