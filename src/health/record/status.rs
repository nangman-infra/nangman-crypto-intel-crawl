#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HealthStatus {
    Ok,
    NoItems,
    Failed,
    NotModified,
    SkippedBackoff,
    SkippedCadence,
}

impl HealthStatus {
    pub(super) fn from_items_seen(items_seen: usize) -> Self {
        if items_seen == 0 {
            Self::NoItems
        } else {
            Self::Ok
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NoItems => "no_items",
            Self::Failed => "failed",
            Self::NotModified => "not_modified",
            Self::SkippedBackoff => "skipped_backoff",
            Self::SkippedCadence => "skipped_cadence",
        }
    }

    pub(super) fn health_level(self) -> &'static str {
        match self {
            Self::Ok | Self::NotModified | Self::SkippedCadence => "healthy",
            Self::NoItems | Self::SkippedBackoff => "degraded",
            Self::Failed => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BackoffState {
    None,
    Scheduled,
    Active,
}

impl BackoffState {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Scheduled => "scheduled",
            Self::Active => "active",
        }
    }
}
