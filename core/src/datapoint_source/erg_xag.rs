//! Obtains the nanoErg per 1 XAG (troy ounce of silver) rate

use std::pin::Pin;

use futures::Future;

use crate::datapoint_source::aggregator::fetch_aggregated;
use crate::datapoint_source::assets_exchange_rate::{
    convert_rate, Asset, AssetsExchangeRate, NanoErg,
};
use crate::datapoint_source::erg_usd::nanoerg_usd_sources;
use crate::datapoint_source::{bitpanda, coingecko, DataPointSourceError};

#[derive(Debug, Clone, Copy)]
pub struct KgAg {}

#[derive(Debug, Clone, Copy)]
pub struct Xag {}

impl Asset for KgAg {}

impl Asset for Xag {}

impl KgAg {
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
pub fn nanoerg_kgag_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<KgAg, NanoErg>, DataPointSourceError>>>>,
> {
    vec![
        Box::pin(coingecko::get_kgag_nanoerg()),
        Box::pin(combined_kgag_nanoerg()),
    ]
}

pub async fn combined_kgag_nanoerg(
) -> Result<AssetsExchangeRate<KgAg, NanoErg>, DataPointSourceError> {
    let kgag_usd_rate = bitpanda::get_kgag_usd().await?;
    let aggregated_usd_nanoerg_rate = fetch_aggregated(nanoerg_usd_sources()).await?;
    Ok(convert_rate(aggregated_usd_nanoerg_rate, kgag_usd_rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kgag_nanoerg_combined() {
        let combined = tokio_test::block_on(combined_kgag_nanoerg()).unwrap();
        let coingecko = tokio_test::block_on(coingecko::get_kgag_nanoerg()).unwrap();
        let deviation_from_coingecko = (combined.rate - coingecko.rate).abs() / coingecko.rate;
        assert!(
            deviation_from_coingecko < 0.05,
            "up to 5% deviation is allowed"
        );
    }
}
