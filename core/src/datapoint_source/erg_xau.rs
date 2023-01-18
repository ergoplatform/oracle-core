//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use super::DataPointSource;
use super::DataPointSourceAggregator;

mod coingecko;

pub fn erg_xau_aggregator() -> Box<dyn DataPointSource> {
    Box::new(DataPointSourceAggregator {
        fetchers: vec![Box::new(coingecko::NanoErgXau)],
    })
}
