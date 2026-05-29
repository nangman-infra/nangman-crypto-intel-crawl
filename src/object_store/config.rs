use std::error::Error;

use super::validation::{validate_bucket_name, validate_region};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObjectStoreConfig {
    pub(crate) bucket: String,
    pub(crate) region: String,
}

impl ObjectStoreConfig {
    pub(super) fn validate(&self) -> Result<(), Box<dyn Error>> {
        validate_bucket_name(&self.bucket)?;
        validate_region(&self.region)?;
        Ok(())
    }
}
