use super::UniverseAsset;

pub(in crate::rest_api) const LIVE_DERIVATIVES_SELECTION_ROTATION_MS: i64 = 30 * 60_000;

pub(in crate::rest_api) fn prioritized_derivatives_assets(
    assets: &[UniverseAsset],
) -> Vec<&UniverseAsset> {
    assets
        .iter()
        .filter(|asset| asset.rss_seed_status.as_deref() == Some("asset_specific_verified"))
        .chain(
            assets.iter().filter(|asset| {
                asset.rss_seed_status.as_deref() != Some("asset_specific_verified")
            }),
        )
        .collect()
}

pub(in crate::rest_api) fn prioritized_live_derivatives_assets<'a>(
    assets: &'a [UniverseAsset],
    source_id: &str,
    selection_time_ms: i64,
) -> Vec<&'a UniverseAsset> {
    prioritized_live_derivatives_assets_for_seed(
        assets,
        live_derivatives_selection_seed(source_id, selection_time_ms),
    )
}

pub(in crate::rest_api) fn prioritized_live_derivatives_assets_for_seed(
    assets: &[UniverseAsset],
    selection_seed: usize,
) -> Vec<&UniverseAsset> {
    let verified = rotated_assets(
        assets
            .iter()
            .filter(|asset| asset.rss_seed_status.as_deref() == Some("asset_specific_verified"))
            .collect::<Vec<_>>(),
        selection_seed,
    );
    let global_only = rotated_assets(
        assets
            .iter()
            .filter(|asset| asset.rss_seed_status.as_deref() != Some("asset_specific_verified"))
            .collect::<Vec<_>>(),
        selection_seed,
    );
    interleave_assets(verified, global_only)
}

pub(super) fn rotated_assets(
    mut assets: Vec<&UniverseAsset>,
    selection_seed: usize,
) -> Vec<&UniverseAsset> {
    if !assets.is_empty() {
        let offset = selection_seed % assets.len();
        assets.rotate_left(offset);
    }
    assets
}

pub(super) fn interleave_assets<'a>(
    primary: Vec<&'a UniverseAsset>,
    secondary: Vec<&'a UniverseAsset>,
) -> Vec<&'a UniverseAsset> {
    let mut ranked = Vec::with_capacity(primary.len() + secondary.len());
    let max_len = primary.len().max(secondary.len());
    for index in 0..max_len {
        if let Some(asset) = primary.get(index) {
            ranked.push(*asset);
        }
        if let Some(asset) = secondary.get(index) {
            ranked.push(*asset);
        }
    }
    ranked
}

pub(super) fn live_derivatives_selection_seed(source_id: &str, selection_time_ms: i64) -> usize {
    let time_slot = selection_time_ms.max(0) / LIVE_DERIVATIVES_SELECTION_ROTATION_MS;
    stable_source_offset(source_id).wrapping_add(time_slot as usize)
}

pub(super) fn stable_source_offset(source_id: &str) -> usize {
    source_id.bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as usize)
    })
}
