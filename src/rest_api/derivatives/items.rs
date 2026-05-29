use super::*;

pub(in crate::rest_api) fn binance_funding_rate_item(
    record: &BinanceFundingRate,
    url: &str,
) -> FeedItem {
    let body = json!({
        "symbol": record.symbol,
        "funding_rate": record.funding_rate,
        "funding_time_ms": record.funding_time,
        "mark_price": record.mark_price
    })
    .to_string();

    FeedItem {
        id: Some(format!("{}:{}", record.symbol, record.funding_time)),
        title: format!("Binance USD-M funding rate {}", record.symbol),
        body,
        url: url.to_owned(),
        author: Some("Binance Futures".to_owned()),
        published_at: Some(record.funding_time.to_string()),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}

pub(in crate::rest_api) fn binance_funding_rate_history_item(
    record: &BinanceFundingRate,
    url: &str,
    backfill_start_ms: i64,
    backfill_end_ms: i64,
) -> FeedItem {
    let source_time_range_verified =
        record.funding_time >= backfill_start_ms && record.funding_time <= backfill_end_ms;
    let body = json!({
        "symbol": record.symbol,
        "funding_rate": record.funding_rate,
        "funding_time_ms": record.funding_time,
        "mark_price": record.mark_price,
        "historical_source_depth": "range_queryable",
        "backfill_window_start_ms": backfill_start_ms,
        "backfill_window_end_ms": backfill_end_ms,
        "source_time_range_verified": source_time_range_verified
    })
    .to_string();

    FeedItem {
        id: Some(format!("{}:{}", record.symbol, record.funding_time)),
        title: format!("Binance USD-M funding rate history {}", record.symbol),
        body,
        url: url.to_owned(),
        author: Some("Binance Futures".to_owned()),
        published_at: Some(record.funding_time.to_string()),
        historical_source_depth: Some("range_queryable".to_owned()),
        backfill_window_start_ms: Some(backfill_start_ms),
        backfill_window_end_ms: Some(backfill_end_ms),
        source_time_range_verified: Some(source_time_range_verified),
    }
}

pub(super) fn binance_open_interest_item(record: &BinanceOpenInterest, url: &str) -> FeedItem {
    let body = json!({
        "symbol": record.symbol,
        "open_interest": record.open_interest,
        "event_time_ms": record.time
    })
    .to_string();

    FeedItem {
        id: Some(format!("{}:{}", record.symbol, record.time)),
        title: format!("Binance USD-M open interest {}", record.symbol),
        body,
        url: url.to_owned(),
        author: Some("Binance Futures".to_owned()),
        published_at: Some(record.time.to_string()),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}
