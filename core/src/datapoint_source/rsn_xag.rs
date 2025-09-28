use futures::Future;
use std::pin::Pin;

use crate::datapoint_source::assets_exchange_rate::{convert_rate, Asset, AssetsExchangeRate};
use crate::datapoint_source::erg_xag::KgAg;
use crate::datapoint_source::{bitpanda, coingecko, ergodex, DataPointSourceError};

#[derive(Debug, Clone, Copy)]
pub struct Rsn {}

impl Asset for Rsn {}

impl Rsn {
    pub fn from_rsn(rsn: f64) -> f64 {
        rsn * 1_000.0
    }
}

#[allow(clippy::type_complexity)]
pub fn rsn_kgag_sources(
) -> Vec<Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<KgAg, Rsn>, DataPointSourceError>>>>>
{
    vec![
        Box::pin(coingecko::get_kgag_rsn()),
        Box::pin(get_rsn_kgag_erg()),
        Box::pin(get_rsn_kgag_usd()),
    ]
}

// Calculate RSN/KGAG through RSN/USD and KGAG/USD
async fn get_rsn_kgag_usd() -> Result<AssetsExchangeRate<KgAg, Rsn>, DataPointSourceError> {
    Ok(convert_rate(
        coingecko::get_rsn_usd().await?,
        bitpanda::get_kgag_usd().await?,
    ))
}

// Calculate KGAG/RSN through KGAG/ERG and ERG/RSN
async fn get_rsn_kgag_erg() -> Result<AssetsExchangeRate<KgAg, Rsn>, DataPointSourceError> {
    Ok(convert_rate(
        ergodex::get_rsn_nanoerg().await?,
        coingecko::get_kgag_nanoerg().await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kgag_rsn_combined() {
        let combined = tokio_test::block_on(get_rsn_kgag_usd()).unwrap();
        let coingecko = tokio_test::block_on(coingecko::get_kgag_rsn()).unwrap();
        let ergodex = tokio_test::block_on(get_rsn_kgag_erg()).unwrap();
        let deviation_from_coingecko = (combined.rate - coingecko.rate).abs() / coingecko.rate;
        assert!(
            deviation_from_coingecko < 0.05,
            "up to 5% deviation is allowed"
        );
        let ergodex_deviation_from_coingecko =
            (ergodex.rate - coingecko.rate).abs() / coingecko.rate;
        assert!(
            ergodex_deviation_from_coingecko < 0.05,
            "up to 5% deviation is allowed"
        );
        let deviation_from_ergodex = (ergodex.rate - combined.rate).abs() / combined.rate;
        assert!(
            deviation_from_ergodex < 0.05,
            "up to 5% deviation is allowed"
        );
    }
}
