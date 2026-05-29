use crate::fetch::{CacheHeaders, apply_cache_headers};
use tokio::time::{Duration, sleep};

pub(super) async fn get_json_response_with_retry(
    client: &reqwest::Client,
    url: &str,
    cache_headers: Option<&CacheHeaders>,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut last_response = None;
    for attempt in 0..=1 {
        let request = client.get(url).header("Accept", "application/json");
        let response = apply_cache_headers(request, cache_headers).send().await?;
        if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Ok(response);
        }
        last_response = Some(response);
        if attempt == 0 {
            sleep(Duration::from_secs(2)).await;
        }
    }
    if let Some(response) = last_response {
        Ok(response)
    } else {
        let request = client.get(url).header("Accept", "application/json");
        apply_cache_headers(request, cache_headers).send().await
    }
}
