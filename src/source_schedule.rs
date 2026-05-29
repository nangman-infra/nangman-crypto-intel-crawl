use crate::args::Args;
use crate::registry::Source;

const HIGH_CADENCE_INTERVAL_MS: i64 = 15 * 60_000;
const MEDIUM_CADENCE_INTERVAL_MS: i64 = 30 * 60_000;
const LOW_CADENCE_INTERVAL_MS: i64 = 6 * 60 * 60_000;

pub(crate) fn should_send_conditional_fetch_headers(args: &Args) -> bool {
    args.source_id.is_none() && args.backfill_start_ms.is_none() && args.backfill_end_ms.is_none()
}

pub(crate) fn cadence_interval_ms(source: &Source) -> i64 {
    match source.cadence_tier.as_str() {
        "high" => HIGH_CADENCE_INTERVAL_MS,
        "medium" => MEDIUM_CADENCE_INTERVAL_MS,
        "low" => LOW_CADENCE_INTERVAL_MS,
        _ => HIGH_CADENCE_INTERVAL_MS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::AppliesToAssets;

    #[test]
    fn cadence_intervals_match_source_tiers() {
        assert_eq!(cadence_interval_ms(&source("high")), 15 * 60_000);
        assert_eq!(cadence_interval_ms(&source("medium")), 30 * 60_000);
        assert_eq!(cadence_interval_ms(&source("low")), 6 * 60 * 60_000);
    }

    #[test]
    fn scheduled_runs_send_conditional_fetch_headers() {
        let args = Args::parse(["intel-crawl-app"].into_iter().map(str::to_owned)).unwrap();

        assert!(should_send_conditional_fetch_headers(&args));
    }

    #[test]
    fn manual_source_runs_bypass_conditional_fetch_headers() {
        let args = Args::parse(
            [
                "intel-crawl-app",
                "--source-id",
                "project_pepe_official_html",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert!(!should_send_conditional_fetch_headers(&args));
    }

    #[test]
    fn backfill_runs_bypass_conditional_fetch_headers() {
        let args = Args::parse(
            [
                "intel-crawl-app",
                "--backfill-start-ms",
                "1000",
                "--backfill-end-ms",
                "2000",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert!(!should_send_conditional_fetch_headers(&args));
    }

    fn source(cadence_tier: &str) -> Source {
        Source {
            source_id: "source".to_owned(),
            source_category: "news".to_owned(),
            source_name: "Source".to_owned(),
            source_url: "https://example.com/feed.xml".to_owned(),
            fetch_method: "rss".to_owned(),
            adapter: None,
            max_items_per_run: None,
            trust_tier: "T1".to_owned(),
            cadence_tier: cadence_tier.to_owned(),
            language_hint: "en".to_owned(),
            enabled: true,
            source_state: None,
            activation_blocker: None,
            top50_relevance_mode: "symbol_alias_match".to_owned(),
            applies_to_assets: AppliesToAssets::All("all_major_50".to_owned()),
        }
    }
}
