//! Obtains the nanoErg/USD rate

use super::aggregator::{source, Source};
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::coingecko;
use super::exchanges;
use super::{bitpanda, coinpaprika};

/// `livecoinwatch_api_key`: from oracle_config.yaml; LiveCoinWatch is left out without it.
pub fn nanoerg_usd_sources(livecoinwatch_api_key: Option<&str>) -> Vec<Source<Usd, NanoErg>> {
    let mut sources = vec![
        source("coinpaprika", coinpaprika::get_usd_nanoerg()),
        source("coingecko", coingecko::get_usd_nanoerg()),
        source("bitpanda", bitpanda::get_usd_nanoerg()),
    ];
    // exchange tickers and more aggregators
    sources.extend(exchanges::nanoerg_usd_sources(livecoinwatch_api_key));
    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datapoint_source::aggregator::{fetch, fetch_aggregated};
    use crate::datapoint_source::MinPriceSources;

    #[test]
    fn test_nanoerg_usd_sources() {
        // every source answers, from its mock or its recorded response
        let count = nanoerg_usd_sources(None).len();
        assert_eq!(count, 7);
        let rates = tokio_test::block_on(fetch("ERG/USD", nanoerg_usd_sources(None)));
        assert_eq!(rates.len(), count);
        let median = tokio_test::block_on(fetch_aggregated(
            "ERG/USD",
            nanoerg_usd_sources(None),
            MinPriceSources::default().erg_usd,
        ))
        .unwrap();
        let usd_per_erg = 1e9 / median.rate;
        assert!((usd_per_erg - 1.669).abs() < 0.001, "{usd_per_erg}");
    }

    #[test]
    fn test_livecoinwatch_only_with_key() {
        // the env var is not consulted (see datapoint_source::tests)
        std::env::set_var("LCW_API_KEY", "env-key");
        let names = |key| {
            nanoerg_usd_sources(key)
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
        };
        assert!(!names(None).contains(&"livecoinwatch"));
        assert!(names(Some("key")).contains(&"livecoinwatch"));
        assert_eq!(nanoerg_usd_sources(Some("key")).len(), 8);
        std::env::remove_var("LCW_API_KEY");
    }
}
