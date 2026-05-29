use crate::item::FeedItem;
use crate::registry::{Source, UniverseAsset};
use serde::Deserialize;
use serde_json::json;
use std::error::Error;

mod assets;
mod funding_history;
mod items;
mod live;
mod models;
mod query;

#[cfg(test)]
pub(super) use assets::{
    LIVE_DERIVATIVES_SELECTION_ROTATION_MS, prioritized_live_derivatives_assets_for_seed,
};
pub(super) use assets::{prioritized_derivatives_assets, prioritized_live_derivatives_assets};
pub(super) use funding_history::fetch_binance_usdm_funding_rate_history;
pub(super) use items::{binance_funding_rate_history_item, binance_funding_rate_item};
pub(super) use live::{fetch_binance_usdm_funding_rates, fetch_binance_usdm_open_interest};
pub(super) use models::BinanceFundingRate;
pub(super) use query::{binance_funding_rate_history_url, required_backfill_window, with_query};

use items::binance_open_interest_item;
use models::BinanceOpenInterest;
