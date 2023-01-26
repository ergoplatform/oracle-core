//! Obtains the nanoErg/USD rate

use std::pin::Pin;

use futures::Future;

use super::assets_exchange_rate::AssetsExchangeRate;
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::coincap;
use super::coingecko;
use super::DataPointSourceError;

#[allow(clippy::type_complexity)]
pub fn nanoerg_usd_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError>>>>,
> {
    vec![
        Box::pin(coincap::get_usd_nanoerg()),
        Box::pin(coingecko::get_usd_nanoerg()),
    ]
}
