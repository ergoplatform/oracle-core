//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use super::aggregator::DataPointSourceAggregator;
use super::DataPointSource;

mod coingecko;

pub fn erg_xau_aggregator() -> Box<dyn DataPointSource> {
    Box::new(DataPointSourceAggregator {
        fetchers: vec![Box::new(coingecko::CoinGecko)],
    })
}
