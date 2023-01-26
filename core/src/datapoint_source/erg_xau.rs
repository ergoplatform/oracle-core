//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use std::pin::Pin;

use futures::Future;

use super::aggregator::fetch_aggregated;
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
    pub fn from_xau(xau: f64) -> f64 {
        // https://en.wikipedia.org/wiki/Gold_bar
        // troy ounces per kg
        xau * 32.150746568627
    }
}

#[allow(clippy::type_complexity)]
pub fn nanoerg_kgau_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError>>>>,
> {
    vec![
        Box::pin(coingecko::get_kgau_nanoerg()),
        Box::pin(bitpanda_coincap_kgau_nanoerg()),
    ]
}

pub async fn bitpanda_coincap_kgau_nanoerg(
) -> Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError> {
    let kgau_usd_rate = bitpanda::get_kgau_usd().await?;
    let aggregated_usd_nanoerg_rate = fetch_aggregated(nanoerg_usd_sources()).await?;
    let rate = kgau_usd_rate.rate * aggregated_usd_nanoerg_rate.rate;
    Ok(AssetsExchangeRate {
        per1: KgAu {},
        get: NanoErg {},
        rate,
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_bitpanda_coincap_combined() {
        let pair = tokio_test::block_on(bitpanda_coincap_kgau_nanoerg()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
