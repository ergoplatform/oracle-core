//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use super::aggregator::DataPointSourceAggregator;
use super::DataPointSource;
use super::KgAu;
use super::NanoErg;

mod coingecko;

pub fn kgau_nanoerg_aggregator() -> Box<dyn DataPointSource> {
    Box::new(DataPointSourceAggregator::<KgAu, NanoErg> {
        fetchers: vec![Box::new(coingecko::CoinGecko)],
    })
}
