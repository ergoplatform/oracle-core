//! Obtains the nanoErg per 1 USD rate

use super::aggregator::DataPointSourceAggregator;
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::coincap;
use super::coingecko;

pub fn usd_nanoerg_aggregator() -> Box<DataPointSourceAggregator<Usd, NanoErg>> {
    Box::new(DataPointSourceAggregator::<Usd, NanoErg> {
        fetchers: vec![Box::new(coingecko::CoinGecko), Box::new(coincap::CoinCap)],
    })
}

#[cfg(test)]
mod tests {
    use crate::datapoint_source::DataPointSource;

    use super::*;

    #[test]
    fn test_aggegator() {
        let aggregator = usd_nanoerg_aggregator();
        let rate = aggregator.get_datapoint().unwrap();
        assert!(rate > 0);
    }
}
