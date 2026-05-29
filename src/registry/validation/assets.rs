use super::super::UniverseAsset;
use std::collections::HashSet;
use std::error::Error;

pub(super) fn validate_universe_assets(
    assets: &[UniverseAsset],
) -> Result<HashSet<&str>, Box<dyn Error>> {
    let mut universe_assets = HashSet::new();
    let mut reference_symbols = HashSet::new();
    for asset in assets {
        validate_universe_asset(asset, &mut universe_assets, &mut reference_symbols)?;
    }
    Ok(universe_assets)
}

fn validate_universe_asset<'a>(
    asset: &'a UniverseAsset,
    universe_assets: &mut HashSet<&'a str>,
    reference_symbols: &mut HashSet<&'a str>,
) -> Result<(), Box<dyn Error>> {
    if asset.asset.trim().is_empty() {
        return Err("source registry asset must not be empty".into());
    }
    if asset.reference_symbol_native.trim().is_empty() {
        return Err(format!(
            "source registry asset {} reference_symbol_native must not be empty",
            asset.asset
        )
        .into());
    }
    if !universe_assets.insert(asset.asset.as_str()) {
        return Err(format!("duplicate universe asset {}", asset.asset).into());
    }
    if !reference_symbols.insert(asset.reference_symbol_native.as_str()) {
        return Err(format!(
            "duplicate reference symbol {}",
            asset.reference_symbol_native
        )
        .into());
    }
    Ok(())
}
