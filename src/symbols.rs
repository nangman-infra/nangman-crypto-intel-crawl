use crate::registry::UniverseAsset;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(crate) struct SymbolMatcher {
    aliases: Vec<AssetAliases>,
}

impl SymbolMatcher {
    pub(crate) fn new(assets: &[UniverseAsset]) -> Self {
        let aliases = assets
            .iter()
            .map(|asset| {
                let mut values = BTreeSet::new();
                let asset_symbol = asset.asset.to_uppercase();
                if !requires_explicit_symbol_context(&asset_symbol) {
                    values.insert(asset_symbol.clone());
                }
                values.insert(asset.reference_symbol_native.to_uppercase());
                values.insert(
                    asset
                        .reference_symbol_native
                        .replace("USDT", "-USDT")
                        .to_uppercase(),
                );
                values.insert(format!("${asset_symbol}"));
                values.insert(format!("({asset_symbol})"));
                values.insert(format!("[{asset_symbol}]"));
                AssetAliases {
                    asset: asset_symbol,
                    values: values.into_iter().collect(),
                }
            })
            .collect();
        Self { aliases }
    }

    pub(crate) fn match_item(&self, title: &str, body: &str, url: &str) -> Vec<String> {
        let haystack = format!("{title}\n{body}\n{url}").to_uppercase();
        let mut matches = BTreeSet::new();
        for alias in &self.aliases {
            if alias
                .values
                .iter()
                .any(|value| contains_token(&haystack, value))
            {
                matches.insert(alias.asset.clone());
            }
        }
        matches.into_iter().collect()
    }
}

#[derive(Debug, Clone)]
struct AssetAliases {
    asset: String,
    values: Vec<String>,
}

fn contains_token(haystack: &str, needle: &str) -> bool {
    if needle.len() < 3 {
        return false;
    }
    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut start = 0;
    while let Some(offset) = haystack[start..].find(needle) {
        let index = start + offset;
        let end = index + needle_bytes.len();
        let left_ok = index == 0 || !is_symbol_char(bytes[index - 1]);
        let right_ok = end == bytes.len() || !is_symbol_char(bytes[end]);
        if left_ok && right_ok {
            return true;
        }
        start = end;
    }
    false
}

fn requires_explicit_symbol_context(asset: &str) -> bool {
    matches!(
        asset,
        "BIO" | "CHIP" | "DASH" | "DOGS" | "HIVE" | "MEGA" | "NEAR" | "NOT" | "TON" | "TRUMP"
    )
}

fn is_symbol_char(value: u8) -> bool {
    value.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_symbol_aliases_without_matching_short_noise() {
        let matcher = SymbolMatcher::new(&[
            UniverseAsset {
                asset: "BTC".to_owned(),
                reference_symbol_native: "BTCUSDT".to_owned(),
                rss_seed_status: None,
            },
            UniverseAsset {
                asset: "U".to_owned(),
                reference_symbol_native: "UUSDT".to_owned(),
                rss_seed_status: None,
            },
        ]);

        assert_eq!(matcher.match_item("BTC rally", "", ""), vec!["BTC"]);
        assert_eq!(matcher.match_item("BTCUSDT volume", "", ""), vec!["BTC"]);
        assert!(matcher.match_item("you and us", "", "").is_empty());
    }

    #[test]
    fn ambiguous_common_words_need_explicit_symbol_context() {
        let matcher = SymbolMatcher::new(&[
            UniverseAsset {
                asset: "NOT".to_owned(),
                reference_symbol_native: "NOTUSDT".to_owned(),
                rss_seed_status: None,
            },
            UniverseAsset {
                asset: "NEAR".to_owned(),
                reference_symbol_native: "NEARUSDT".to_owned(),
                rss_seed_status: None,
            },
        ]);

        assert!(
            matcher
                .match_item("This is not a token listing", "", "")
                .is_empty()
        );
        assert!(
            matcher
                .match_item("Near term market update", "", "")
                .is_empty()
        );
        assert_eq!(
            matcher.match_item("Notcoin (NOT) listing", "", ""),
            vec!["NOT"]
        );
        assert_eq!(
            matcher.match_item("$NEAR ecosystem update", "", ""),
            vec!["NEAR"]
        );
        assert_eq!(
            matcher.match_item("NOTUSDT futures launch", "", ""),
            vec!["NOT"]
        );
    }
}
