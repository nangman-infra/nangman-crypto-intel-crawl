const TRACKING_QUERY_PREFIXES: &[&str] = &["utm_"];
const TRACKING_QUERY_KEYS: &[&str] = &[
    "fbclid", "gclid", "mc_cid", "mc_eid", "igshid", "ref", "ref_src",
];

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
