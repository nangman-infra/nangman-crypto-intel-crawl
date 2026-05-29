use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(in crate::rest_api) struct BinanceCmsResponse {
    pub(in crate::rest_api) code: String,
    pub(in crate::rest_api) data: Option<BinanceCmsData>,
}

#[derive(Debug, Deserialize)]
pub(in crate::rest_api) struct BinanceCmsData {
    pub(in crate::rest_api) catalogs: Vec<BinanceCmsCatalog>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::rest_api) struct BinanceCmsCatalog {
    pub(in crate::rest_api) catalog_id: u64,
    pub(in crate::rest_api) catalog_name: String,
    #[serde(default)]
    pub(in crate::rest_api) articles: Vec<BinanceCmsArticle>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::rest_api) struct BinanceCmsArticle {
    pub(in crate::rest_api) id: u64,
    pub(in crate::rest_api) code: String,
    pub(in crate::rest_api) title: String,
    pub(in crate::rest_api) release_date: i64,
}

#[derive(Debug, Deserialize)]
pub(in crate::rest_api) struct BinanceCmsDetailResponse {
    pub(in crate::rest_api) code: String,
    pub(in crate::rest_api) data: Option<BinanceCmsDetail>,
}

#[derive(Debug, Deserialize)]
pub(in crate::rest_api) struct BinanceCmsDetail {
    pub(in crate::rest_api) body: String,
}
