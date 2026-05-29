use crate::args::Args;
use crate::balance::SourceBalancePolicy;
use crate::object_store::ObjectStore;
use crate::registry::SourceRegistry;
use crate::source_state::SourceFetchStates;
use crate::symbols::SymbolMatcher;

pub(in crate::crawl_loop) struct CrawlContext<'a> {
    pub(in crate::crawl_loop) args: &'a Args,
    pub(in crate::crawl_loop) registry: &'a SourceRegistry,
    pub(in crate::crawl_loop) object_store: Option<&'a ObjectStore>,
    pub(in crate::crawl_loop) source_states: &'a mut SourceFetchStates,
    pub(in crate::crawl_loop) matcher: &'a SymbolMatcher,
    pub(in crate::crawl_loop) client: &'a reqwest::Client,
    pub(in crate::crawl_loop) balance_policy: SourceBalancePolicy,
}

impl<'a> CrawlContext<'a> {
    pub(in crate::crawl_loop) fn new(
        args: &'a Args,
        registry: &'a SourceRegistry,
        object_store: Option<&'a ObjectStore>,
        source_states: &'a mut SourceFetchStates,
        matcher: &'a SymbolMatcher,
        client: &'a reqwest::Client,
        balance_policy: SourceBalancePolicy,
    ) -> Self {
        Self {
            args,
            registry,
            object_store,
            source_states,
            matcher,
            client,
            balance_policy,
        }
    }
}
