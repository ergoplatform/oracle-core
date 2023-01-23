//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use super::aggregator::DataPointSourceAggregator;
use super::assets_exchange_rate::KgAu;
use super::assets_exchange_rate::NanoErg;

mod coingecko;

pub fn kgau_nanoerg_aggregator() -> Box<DataPointSourceAggregator<KgAu, NanoErg>> {
    Box::new(DataPointSourceAggregator::<KgAu, NanoErg> {
        fetchers: vec![Box::new(coingecko::CoinGecko)],
    })
}
