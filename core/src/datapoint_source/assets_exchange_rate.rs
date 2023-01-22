use futures::future::BoxFuture;

use super::DataPointSourceError;

pub trait Asset {}

pub struct NanoErg {}
pub struct Erg {}
pub struct KgAu {}
pub struct Xau {}
pub struct Usd {}

impl Asset for Erg {}
impl Asset for NanoErg {}
impl Asset for KgAu {}
impl Asset for Xau {}
impl Asset for Usd {}

pub struct AssetsExchangeRate<PER1: Asset, GET: Asset> {
    pub per1: PER1,
    pub get: GET,
    pub rate: f64,
}

pub trait AssetsExchangeRateSource<L: Asset, R: Asset> {
    fn get_rate(&self) -> BoxFuture<Result<AssetsExchangeRate<L, R>, DataPointSourceError>>;
}
