use super::{Error, Source};

pub(in crate::rest_api) fn with_query(base_url: &str, params: &[(&str, &str)]) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base_url}{separator}{query}")
}

pub(in crate::rest_api) fn binance_funding_rate_history_url(
    base_url: &str,
    symbol: &str,
    start_time_ms: i64,
    end_time_ms: i64,
    limit: &str,
) -> String {
    with_query(
        base_url,
        &[
            ("symbol", symbol),
            ("startTime", &start_time_ms.to_string()),
            ("endTime", &end_time_ms.to_string()),
            ("limit", limit),
        ],
    )
}

pub(in crate::rest_api) fn required_backfill_window(
    source: &Source,
    backfill_start_ms: Option<i64>,
    backfill_end_ms: Option<i64>,
) -> Result<(i64, i64), Box<dyn Error>> {
    let Some(start_ms) = backfill_start_ms else {
        return Err(format!("{} requires --backfill-start-ms", source.source_id).into());
    };
    let Some(end_ms) = backfill_end_ms else {
        return Err(format!("{} requires --backfill-end-ms", source.source_id).into());
    };
    if start_ms >= end_ms {
        return Err(format!(
            "{} requires backfill_start_ms < backfill_end_ms",
            source.source_id
        )
        .into());
    }
    Ok((start_ms, end_ms))
}
