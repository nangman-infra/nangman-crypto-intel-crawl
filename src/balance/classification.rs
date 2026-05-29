use crate::registry::Source;

pub(super) fn is_derivatives_snapshot_source(source: &Source) -> bool {
    source.source_category == "funding"
        && source.fetch_method == "rest_api"
        && !source.is_manual_backfill_source()
}

pub(super) fn is_community_source(source: &Source) -> bool {
    source.source_category == "social"
}
