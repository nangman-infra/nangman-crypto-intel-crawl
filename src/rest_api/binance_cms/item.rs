use super::types::{BinanceCmsArticle, BinanceCmsCatalog};
use crate::item::FeedItem;
use serde_json::json;

pub(in crate::rest_api) fn binance_cms_article_item(
    article: &BinanceCmsArticle,
    body: String,
) -> FeedItem {
    let url = format!(
        "https://www.binance.com/en/support/announcement/detail/{}",
        article.code
    );

    FeedItem {
        id: Some(article.code.clone()),
        title: article.title.clone(),
        body,
        url,
        author: Some("Binance".to_owned()),
        published_at: Some(article.release_date.to_string()),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}

pub(in crate::rest_api) fn binance_cms_article_metadata_body(
    source_id: &str,
    catalog: &BinanceCmsCatalog,
    article: &BinanceCmsArticle,
) -> String {
    json!({
        "catalog_id": catalog.catalog_id,
        "catalog_name": catalog.catalog_name,
        "article_id": article.id,
        "article_code": article.code,
        "source_id": source_id
    })
    .to_string()
}
