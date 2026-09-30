//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use super::aggregator::fetch_aggregated;
use super::aggregator::{source, RateResult, Source};
use super::assets_exchange_rate::convert_rate;
use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::bitpanda;
use super::coingecko;
use super::erg_usd::nanoerg_usd_sources;
#[cfg(not(test))]
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

pub fn nanoerg_kgau_sources(min_price_sources: MinPriceSources) -> Vec<Source<KgAu, NanoErg>> {
    vec![
        source("coingecko", coingecko::get_kgau_nanoerg()),
        source("combined", combined_kgau_nanoerg(min_price_sources)),
    ]
}

pub async fn combined_kgau_nanoerg(
    min_price_sources: MinPriceSources,
) -> RateResult<KgAu, NanoErg> {
    let kgau_usd_rate =
        fetch_aggregated("gold/USD", kgau_usd_sources(), min_price_sources.gold_usd).await?;
    let aggregated_usd_nanoerg_rate =
        fetch_aggregated("ERG/USD", nanoerg_usd_sources(), min_price_sources.erg_usd).await?;
    Ok(convert_rate(aggregated_usd_nanoerg_rate, kgau_usd_rate))
}

/// Independent gold prices: Bitpanda, COMEX futures and PAXG (see exchanges.rs)
fn kgau_usd_sources() -> Vec<Source<KgAu, Usd>> {
    #[allow(unused_mut)]
    let mut sources = vec![source("bitpanda", bitpanda::get_kgau_usd())];
    // live network, so not in tests
    #[cfg(not(test))]
    sources.extend(exchanges::kgau_usd_sources());
    sources
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_kgau_nanoerg_combined() {
        // test builds have only the Bitpanda gold mock
        let min_price_sources = MinPriceSources {
            erg_usd: 3,
            gold_usd: 1,
        };
        let combined = tokio_test::block_on(combined_kgau_nanoerg(min_price_sources)).unwrap();
        let coingecko = tokio_test::block_on(coingecko::get_kgau_nanoerg()).unwrap();
        let deviation_from_coingecko = (combined.rate - coingecko.rate).abs() / coingecko.rate;
        assert!(
            deviation_from_coingecko < 0.05,
            "up to 5% deviation is allowed"
        );
    }
}
