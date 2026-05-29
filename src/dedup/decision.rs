#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DedupDecision {
    New,
    ExactDuplicate {
        duplicate_of_event_id: Option<String>,
    },
    ContentDuplicate {
        duplicate_of_event_id: Option<String>,
    },
    NearDuplicate {
        duplicate_of_event_id: Option<String>,
    },
    UpdateOfExisting {
        duplicate_of_event_id: Option<String>,
    },
}

impl DedupDecision {
    pub(crate) fn is_skipped_duplicate(&self) -> bool {
        matches!(
            self,
            Self::ExactDuplicate { .. }
                | Self::ContentDuplicate { .. }
                | Self::NearDuplicate { .. }
        )
    }

    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::ExactDuplicate { .. } => "exact_duplicate",
            Self::ContentDuplicate { .. } => "content_duplicate",
            Self::NearDuplicate { .. } => "near_duplicate",
            Self::UpdateOfExisting { .. } => "update_of_existing",
        }
    }

    pub(crate) fn duplicate_of_event_id(&self) -> Option<String> {
        match self {
            Self::New => None,
            Self::ExactDuplicate {
                duplicate_of_event_id,
            }
            | Self::ContentDuplicate {
                duplicate_of_event_id,
            }
            | Self::NearDuplicate {
                duplicate_of_event_id,
            }
            | Self::UpdateOfExisting {
                duplicate_of_event_id,
            } => duplicate_of_event_id.clone(),
        }
    }
}
