use super::types::{BinanceCmsArticle, BinanceCmsDetailResponse};

pub(super) async fn fetch_binance_cms_article_body(
    client: &reqwest::Client,
    article: &BinanceCmsArticle,
) -> Option<String> {
    let url = format!(
        "https://www.binance.com/bapi/composite/v1/public/cms/article/detail/query?articleCode={}",
        article.code
    );
    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload = response.json::<BinanceCmsDetailResponse>().await.ok()?;
    if payload.code != "000000" {
        return None;
    }
    let detail = payload.data?;
    let body_json = serde_json::from_str::<serde_json::Value>(&detail.body).ok()?;
    let mut parts = Vec::new();
    collect_binance_text(&body_json, &mut parts);
    let body = parts.join(" ");
    if body.trim().is_empty() {
        None
    } else {
        Some(body)
    }
}

pub(in crate::rest_api) fn collect_binance_text(
    value: &serde_json::Value,
    parts: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("node").and_then(serde_json::Value::as_str) == Some("text")
                && let Some(text) = map.get("text").and_then(serde_json::Value::as_str)
            {
                parts.push(text.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            for child in map.values() {
                collect_binance_text(child, parts);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                collect_binance_text(child, parts);
            }
        }
        _ => {}
    }
}
