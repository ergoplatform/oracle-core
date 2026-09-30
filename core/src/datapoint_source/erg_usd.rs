//! Obtains the nanoErg/USD rate

use super::aggregator::{source, Source};
use super::assets_exchange_rate::NanoErg;
use super::assets_exchange_rate::Usd;
use super::coingecko;
#[cfg(not(test))]
use super::exchanges;
use super::{bitpanda, coinpaprika};

pub fn nanoerg_usd_sources() -> Vec<Source<Usd, NanoErg>> {
    #[allow(unused_mut)]
    let mut sources = vec![
        source("coinpaprika", coinpaprika::get_usd_nanoerg()),
        source("coingecko", coingecko::get_usd_nanoerg()),
        source("bitpanda", bitpanda::get_usd_nanoerg()),
    ];
    // exchange tickers and more aggregators (live network, so not in tests)
    #[cfg(not(test))]
    sources.extend(exchanges::nanoerg_usd_sources());
    sources
}
