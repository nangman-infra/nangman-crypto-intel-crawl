use crate::registry::Source;

pub(super) fn content_quality(source: &Source, body: &str) -> &'static str {
    if source.source_category == "funding" {
        return "numeric_observation";
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        "title_only"
    } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
        "metadata_fallback"
    } else if trimmed.chars().count() < 120 {
        "short_text"
    } else {
        "full_text"
    }
}

pub(super) fn content_quality_score(
    source: &Source,
    content_quality: &str,
    direct_asset_count: usize,
    matched_asset_count: usize,
) -> u8 {
    let mut score = match content_quality {
        "full_text" => 70,
        "short_text" => 55,
        "numeric_observation" => 50,
        "metadata_fallback" => 40,
        "title_only" => 25,
        _ => 35,
    };
    if source.trust_tier == "T0" {
        score += 15;
    } else if source.trust_tier == "T1" {
        score += 8;
    }
    if direct_asset_count > 0 {
        score += 10;
    } else if matched_asset_count > 0 {
        score += 5;
    }
    score.min(100)
}
