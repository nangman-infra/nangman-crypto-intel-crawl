use crate::args::Args;
use crate::dedup::DedupStore;
use crate::object_store::ObjectStore;
use crate::publisher::EventPublisher;
use crate::registry::{Source, SourceRegistry};
use crate::source_state::SourceFetchStates;
use crate::storage::IntelL0Storage;
use crate::symbols::SymbolMatcher;

pub(crate) struct CrawlOutputs<'a> {
    pub(crate) dedup: &'a mut DedupStore,
}

pub(crate) struct CrawlOnceInput<'a> {
    pub(crate) args: &'a Args,
    pub(crate) registry: &'a SourceRegistry,
    pub(crate) sources: Vec<&'a Source>,
    pub(crate) object_store: Option<&'a ObjectStore>,
    pub(crate) source_states: &'a mut SourceFetchStates,
    pub(crate) matcher: &'a SymbolMatcher,
    pub(crate) client: &'a reqwest::Client,
    pub(crate) outputs: &'a mut CrawlOutputs<'a>,
    pub(crate) publisher: &'a EventPublisher,
    pub(crate) storage: Option<&'a IntelL0Storage>,
}
