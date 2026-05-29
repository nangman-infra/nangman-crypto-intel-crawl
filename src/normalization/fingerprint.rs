use super::{ContentFingerprint, canonicalize_url, hash_hex, normalize_text_for_dedup, simhash64};

pub(crate) fn content_fingerprint(title: &str, body: &str, url: &str) -> ContentFingerprint {
    let canonical_url = canonicalize_url(url);
    let normalized_text = normalized_title_body(title, body);
    ContentFingerprint {
        canonical_url_hash: hash_hex(&canonical_url),
        normalized_text_hash: hash_hex(&normalized_text),
        simhash64: simhash64(&normalized_text),
        canonical_url,
    }
}

fn normalized_title_body(title: &str, body: &str) -> String {
    let mut text = String::with_capacity(title.len() + body.len() + 1);
    text.push_str(title);
    text.push('\n');
    text.push_str(body);
    normalize_text_for_dedup(&text)
}
