//! Obtains the lovelace per 1 USD rate.

use std::pin::Pin;

use futures::Future;

use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::AssetsExchangeRate;
use super::assets_exchange_rate::Usd;
use super::coingecko;
use super::DataPointSourceError;

#[derive(Debug, Clone, Copy)]
pub struct Ada {}

#[derive(Debug, Clone, Copy)]
pub struct Lovelace {}

impl Asset for Ada {}
impl Asset for Lovelace {}

impl Lovelace {
    pub fn from_ada(ada: f64) -> f64 {
        ada * 1_000_000.0
    }
}

#[allow(clippy::type_complexity)]
pub fn usd_lovelace_sources() -> Vec<
    Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<Usd, Lovelace>, DataPointSourceError>>>>,
> {
    vec![Box::pin(coingecko::get_usd_lovelace())]
}
