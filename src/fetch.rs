use crate::item::FeedItem;
use reqwest::header::{ETAG, HeaderMap, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CacheHeaders {
    pub(crate) etag: Option<String>,
    pub(crate) last_modified: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchMetadata {
    pub(crate) http_status: u16,
    pub(crate) etag: Option<String>,
    pub(crate) last_modified: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceFetchResult {
    Fetched {
        items: Vec<FeedItem>,
        metadata: FetchMetadata,
    },
    NotModified {
        metadata: FetchMetadata,
    },
}

pub(crate) fn apply_cache_headers(
    request: reqwest::RequestBuilder,
    cache_headers: Option<&CacheHeaders>,
) -> reqwest::RequestBuilder {
    let Some(cache_headers) = cache_headers else {
        return request;
    };
    let request = if let Some(etag) = cache_headers.etag.as_deref() {
        request.header(IF_NONE_MATCH, etag)
    } else {
        request
    };
    if let Some(last_modified) = cache_headers.last_modified.as_deref() {
        request.header(IF_MODIFIED_SINCE, last_modified)
    } else {
        request
    }
}

pub(crate) fn metadata_from_headers(
    status: reqwest::StatusCode,
    headers: &HeaderMap,
) -> FetchMetadata {
    FetchMetadata {
        http_status: status.as_u16(),
        etag: header_to_string(headers, ETAG),
        last_modified: header_to_string(headers, LAST_MODIFIED),
    }
}

fn header_to_string(headers: &HeaderMap, name: reqwest::header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn captures_cache_metadata() {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"abc\""));
        headers.insert(
            LAST_MODIFIED,
            HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );

        let metadata = metadata_from_headers(reqwest::StatusCode::OK, &headers);

        assert_eq!(metadata.http_status, 200);
        assert_eq!(metadata.etag.as_deref(), Some("\"abc\""));
        assert_eq!(
            metadata.last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
    }
}
