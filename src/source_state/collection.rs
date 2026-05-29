use super::SOURCE_FETCH_STATE_SCHEMA;
use super::keys::state_object_key;
use super::state::SourceFetchState;
use crate::object_store::ObjectStore;
use crate::registry::Source;
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Default)]
pub(crate) struct SourceFetchStates {
    states: BTreeMap<String, SourceFetchState>,
}

impl SourceFetchStates {
    pub(crate) async fn load(
        object_store: Option<&ObjectStore>,
        sources: &[&Source],
    ) -> Result<Self, Box<dyn Error>> {
        let mut states = BTreeMap::new();
        for source in sources {
            let state = if let Some(object_store) = object_store {
                load_state(object_store, source).await?
            } else {
                SourceFetchState::new(source)
            };
            states.insert(source.source_id.clone(), state);
        }
        Ok(Self { states })
    }

    pub(crate) fn get(&self, source: &Source) -> Option<&SourceFetchState> {
        self.states.get(&source.source_id)
    }

    pub(crate) fn get_mut(&mut self, source: &Source) -> &mut SourceFetchState {
        self.states
            .entry(source.source_id.clone())
            .or_insert_with(|| SourceFetchState::new(source))
    }

    pub(crate) async fn persist(
        &self,
        object_store: Option<&ObjectStore>,
    ) -> Result<(), Box<dyn Error>> {
        let Some(object_store) = object_store else {
            return Ok(());
        };
        for state in self.states.values() {
            let bytes = serde_json::to_vec_pretty(state)?;
            object_store
                .put_bytes(
                    &state_object_key(&state.source_id),
                    bytes,
                    "application/json",
                )
                .await?;
        }
        Ok(())
    }
}

async fn load_state(
    object_store: &ObjectStore,
    source: &Source,
) -> Result<SourceFetchState, Box<dyn Error>> {
    let key = state_object_key(&source.source_id);
    if !object_store.key_exists(&key).await? {
        return Ok(SourceFetchState::new(source));
    }
    let raw = object_store.get_bytes(&key).await?;
    let mut state = serde_json::from_slice::<SourceFetchState>(&raw)?;
    if state.schema_version != SOURCE_FETCH_STATE_SCHEMA || state.source_id != source.source_id {
        state = SourceFetchState::new(source);
    }
    Ok(state)
}
