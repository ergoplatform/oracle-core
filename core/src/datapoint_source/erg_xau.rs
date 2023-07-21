//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use std::pin::Pin;

use futures::Future;

use super::aggregator::fetch_aggregated;
use super::assets_exchange_rate::convert_rate;
use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::AssetsExchangeRate;
use super::assets_exchange_rate::NanoErg;
use super::bitpanda;
use super::coingecko;
use super::erg_usd::nanoerg_usd_sources;
use super::DataPointSourceError;

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

#[allow(clippy::type_complexity)]
pub fn nanoerg_kgau_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError>>>>,
> {
    vec![
        Box::pin(coingecko::get_kgau_nanoerg()),
        Box::pin(combined_kgau_nanoerg()),
    ]
}

pub async fn combined_kgau_nanoerg(
) -> Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError> {
    let kgau_usd_rate = bitpanda::get_kgau_usd().await?;
    let aggregated_usd_nanoerg_rate = fetch_aggregated(nanoerg_usd_sources()).await?;
    Ok(convert_rate(aggregated_usd_nanoerg_rate, kgau_usd_rate))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_kgau_nanoerg_combined() {
        let combined = tokio_test::block_on(combined_kgau_nanoerg()).unwrap();
        let coingecko = tokio_test::block_on(coingecko::get_kgau_nanoerg()).unwrap();
        let deviation_from_coingecko = (combined.rate - coingecko.rate).abs() / coingecko.rate;
        assert!(
            deviation_from_coingecko < 0.05,
            "up to 5% deviation is allowed"
        );
    }
}
