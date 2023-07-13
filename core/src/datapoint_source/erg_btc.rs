use std::pin::Pin;

use futures::Future;

use super::{
    assets_exchange_rate::{convert, AssetsExchangeRate, Btc, NanoErg},
    coincap, coingecko, DataPointSourceError,
};

#[allow(clippy::type_complexity)]
pub fn nanoerg_btc_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<Btc, NanoErg>, DataPointSourceError>>>>,
> {
    vec![
        Box::pin(coingecko::get_btc_nanoerg()),
        Box::pin(get_btc_nanoerg_coincap()),
    ]
}

// Calculate ERG/BTC through ERG/USD and USD/BTC
async fn get_btc_nanoerg_coincap() -> Result<AssetsExchangeRate<Btc, NanoErg>, DataPointSourceError>
{
    Ok(convert(
        coincap::get_usd_nanoerg().await?,
        coincap::get_btc_usd().await?,
    ))
}

#[cfg(test)]
mod test {
    use super::coingecko;
    use super::get_btc_nanoerg_coincap;
    #[test]
    fn test_btc_nanoerg_combined() {
        let combined = tokio_test::block_on(get_btc_nanoerg_coincap()).unwrap();
        let coingecko = tokio_test::block_on(coingecko::get_btc_nanoerg()).unwrap();
        let deviation_from_coingecko = (combined.rate - coingecko.rate).abs() / coingecko.rate;
        assert!(
            deviation_from_coingecko < 0.05,
            "up to 5% deviation is allowed"
        );
    }
}
