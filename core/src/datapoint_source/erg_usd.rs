//! Obtains the nanoErg/USD rate

use std::pin::Pin;

use futures::Future;

use super::assets_exchange_rate::AssetsExchangeRate;
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::coingecko;
use super::DataPointSourceError;
use super::{bitpanda, coinpaprika};

#[cfg(not(test))]
use super::exchanges;

#[allow(clippy::type_complexity)]
pub fn nanoerg_usd_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError>>>>,
> {
    #[allow(unused_mut)]
    let mut sources: Vec<
        Pin<
            Box<
                dyn Future<Output = Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError>>,
            >,
        >,
    > = vec![
        Box::pin(coinpaprika::get_usd_nanoerg()),
        Box::pin(coingecko::get_usd_nanoerg()),
        Box::pin(bitpanda::get_usd_nanoerg()),
    ];
    // exchange tickers and more aggregators (live network, so not in tests)
    #[cfg(not(test))]
    sources.extend(exchanges::nanoerg_usd_sources());
    sources
}
