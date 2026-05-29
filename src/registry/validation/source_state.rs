use super::super::Source;
use super::source_contract::validate_allowed_value;
use std::error::Error;

pub(super) fn validate_source_state(source: &Source) -> Result<(), Box<dyn Error>> {
    let Some(source_state) = source.source_state.as_deref() else {
        return Ok(());
    };
    validate_allowed_value(
        source,
        "source_state",
        source_state,
        &["enabled", "available_disabled", "blocked", "unsupported"],
    )?;
    validate_enabled_state_alignment(source, source_state)?;
    validate_activation_blocker(source, source_state)
}

fn validate_enabled_state_alignment(
    source: &Source,
    source_state: &str,
) -> Result<(), Box<dyn Error>> {
    if source.enabled && source_state != "enabled" {
        return Err(format!(
            "source {} enabled=true requires source_state enabled",
            source.source_id
        )
        .into());
    }
    if !source.enabled && source_state == "enabled" {
        return Err(format!(
            "source {} source_state enabled conflicts with enabled=false",
            source.source_id
        )
        .into());
    }
    Ok(())
}

fn validate_activation_blocker(source: &Source, source_state: &str) -> Result<(), Box<dyn Error>> {
    let has_blocker = !source
        .activation_blocker
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty();
    if !source.enabled && source_state != "enabled" && !has_blocker {
        return Err(format!(
            "source {} disabled inventory source requires activation_blocker",
            source.source_id
        )
        .into());
    }
    Ok(())
}
