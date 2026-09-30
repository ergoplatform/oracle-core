//! Obtains the nanoErg/USD rate

use super::aggregator::{source, Source};
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::coingecko;
use super::exchanges;
use super::{bitpanda, coinpaprika};

pub fn nanoerg_usd_sources() -> Vec<Source<Usd, NanoErg>> {
    let mut sources = vec![
        source("coinpaprika", coinpaprika::get_usd_nanoerg()),
        source("coingecko", coingecko::get_usd_nanoerg()),
        source("bitpanda", bitpanda::get_usd_nanoerg()),
    ];
    // exchange tickers and more aggregators
    sources.extend(exchanges::nanoerg_usd_sources());
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
        let count = nanoerg_usd_sources().len();
        assert!(count >= 7, "LiveCoinWatch is the only optional source");
        let rates = tokio_test::block_on(fetch("ERG/USD", nanoerg_usd_sources()));
        assert_eq!(rates.len(), count);
        let median = tokio_test::block_on(fetch_aggregated(
            "ERG/USD",
            nanoerg_usd_sources(),
            MinPriceSources::default().erg_usd,
        ))
        .unwrap();
        let usd_per_erg = 1e9 / median.rate;
        assert!((usd_per_erg - 1.669).abs() < 0.001, "{usd_per_erg}");
    }
}
