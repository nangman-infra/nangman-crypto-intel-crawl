use super::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::rest_api) struct BinanceFundingRate {
    pub(in crate::rest_api) symbol: String,
    pub(in crate::rest_api) funding_rate: String,
    pub(in crate::rest_api) funding_time: i64,
    pub(in crate::rest_api) mark_price: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BinanceOpenInterest {
    pub(super) symbol: String,
    pub(super) open_interest: String,
    pub(super) time: i64,
}
