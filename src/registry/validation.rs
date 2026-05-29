mod applies_to;
mod assets;
mod fetch_contract;
mod source_contract;
mod source_state;

use super::SourceRegistry;
use std::collections::HashSet;
use std::error::Error;

impl SourceRegistry {
    pub(super) fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.universe_assets.is_empty() {
            return Err("source registry universe_assets must not be empty".into());
        }
        if self.sources.is_empty() {
            return Err("source registry sources must not be empty".into());
        }
        let universe_assets = assets::validate_universe_assets(&self.universe_assets)?;
        let mut source_ids = HashSet::new();
        for source in &self.sources {
            source_contract::validate_source_id(source, &mut source_ids)?;
            source_contract::validate_source_contract(source, &universe_assets)?;
        }
        Ok(())
    }
}
