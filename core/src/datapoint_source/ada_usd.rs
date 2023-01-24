//! Obtains the lovelace per 1 USD rate.

use super::aggregator::DataPointSourceAggregator;
use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::Usd;
use super::coingecko;

pub struct Ada {}
pub struct Lovelace {}

impl Asset for Ada {}
impl Asset for Lovelace {}

impl Lovelace {
    pub fn from_ada(ada: f64) -> f64 {
        ada * 1_000_000.0
    }
}

pub fn usd_lovelace_aggregator() -> Box<DataPointSourceAggregator<Usd, Lovelace>> {
    Box::new(DataPointSourceAggregator::<Usd, Lovelace> {
        fetchers: vec![Box::new(coingecko::CoinGecko)],
    })
}
