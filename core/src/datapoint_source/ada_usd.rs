//! Obtains the lovelace per 1 USD rate.

use super::aggregator::{source, Source};
use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::Usd;
use super::coingecko;

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

pub fn usd_lovelace_sources() -> Vec<Source<Usd, Lovelace>> {
    vec![source("coingecko", coingecko::get_usd_lovelace())]
}
