//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use super::aggregator::{aggregate, fetch, fetch_aggregated};
use super::aggregator::{source, RateResult, Source};
use super::assets_exchange_rate::convert_rate;
use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::bitpanda;
use super::coingecko;
use super::erg_usd::nanoerg_usd_sources;
use super::exchanges;
use super::MinPriceSources;

#[derive(Debug, Clone, Copy)]
pub struct KgAu {}

#[derive(Debug, Clone, Copy)]
pub struct Xau {}

impl Asset for KgAu {}
impl Asset for Xau {}

impl KgAu {
    pub fn from_troy_ounce(oz: f64) -> f64 {
        // https://en.wikipedia.org/wiki/Gold_bar
        // troy ounces per kg
        oz * 32.150746568627
    }

    pub fn from_gram(g: f64) -> f64 {
        g * 1000.0
    }
}

/// The ERG/XAU datapoint: the combined rate, averaged with CoinGecko's direct
/// ERG/XAU quote when that answers. The combined rate must meet its quorums;
/// CoinGecko's single quote is never posted on its own.
pub async fn nanoerg_kgau(min_price_sources: MinPriceSources) -> RateResult<KgAu, NanoErg> {
    let (combined, mut rates) = futures::join!(
        combined_kgau_nanoerg(min_price_sources),
        fetch(
            "ERG/XAU",
            vec![source("coingecko", coingecko::get_kgau_nanoerg())]
        ),
    );
    rates.push(combined?);
    Ok(aggregate(rates))
}

/// ERG/XAU = gold/USD x ERG/USD, each leg a median over its own sources with its
/// own quorum.
pub async fn combined_kgau_nanoerg(
    min_price_sources: MinPriceSources,
) -> RateResult<KgAu, NanoErg> {
    // both legs at once, so the round takes as long as the slower leg, not both
    let (kgau_usd_rate, aggregated_usd_nanoerg_rate) = futures::join!(
        fetch_aggregated("gold/USD", kgau_usd_sources(), min_price_sources.gold_usd),
        fetch_aggregated("ERG/USD", nanoerg_usd_sources(), min_price_sources.erg_usd),
    );
    Ok(convert_rate(aggregated_usd_nanoerg_rate?, kgau_usd_rate?))
}

/// Independent gold prices: Bitpanda, COMEX futures and PAXG (see exchanges.rs)
fn kgau_usd_sources() -> Vec<Source<KgAu, Usd>> {
    let mut sources = vec![source("bitpanda", bitpanda::get_kgau_usd())];
    sources.extend(exchanges::kgau_usd_sources());
    sources
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::datapoint_source::DataPointSourceError;

    #[test]
    fn test_kgau_nanoerg_combined() {
        let combined =
            tokio_test::block_on(combined_kgau_nanoerg(MinPriceSources::default())).unwrap();
        let coingecko = tokio_test::block_on(coingecko::get_kgau_nanoerg()).unwrap();
        let deviation_from_coingecko = (combined.rate - coingecko.rate).abs() / coingecko.rate;
        assert!(
            deviation_from_coingecko < 0.05,
            "up to 5% deviation is allowed"
        );
        // the datapoint is the mean of the combined rate and CoinGecko's quote
        let datapoint = tokio_test::block_on(nanoerg_kgau(MinPriceSources::default())).unwrap();
        assert_eq!(datapoint.rate, (combined.rate + coingecko.rate) / 2.0);
    }

    #[test]
    fn test_kgau_nanoerg_combined_quorum() {
        // the mocks give 3 gold votes and at least 7 ERG/USD sources
        let combined = |erg_usd, gold_usd| {
            tokio_test::block_on(combined_kgau_nanoerg(MinPriceSources { erg_usd, gold_usd }))
        };
        assert!(combined(7, 3).is_ok());
        assert!(matches!(
            combined(3, 4),
            Err(DataPointSourceError::NotEnoughSources {
                pair: "gold/USD",
                got: 3,
                min: 4
            })
        ));
        assert!(matches!(
            combined(99, 2),
            Err(DataPointSourceError::NotEnoughSources {
                pair: "ERG/USD",
                min: 99,
                ..
            })
        ));
    }
}
