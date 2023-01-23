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

impl Erg {
    pub fn to_nanoerg(erg: f64) -> f64 {
        erg * 1_000_000_000.0
    }
}

impl NanoErg {
    /// Number of nanoErgs in a single Erg
    pub fn from_erg(erg: f64) -> f64 {
        erg * 1_000_000_000.0
    }
}

pub struct AssetsExchangeRate<PER1: Asset, GET: Asset> {
    pub per1: PER1,
    pub get: GET,
    pub rate: f64,
}

pub trait AssetsExchangeRateSource<L: Asset, R: Asset> {
    fn get_rate(&self) -> BoxFuture<Result<AssetsExchangeRate<L, R>, DataPointSourceError>>;
}
