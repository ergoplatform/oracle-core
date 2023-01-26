pub trait Asset: Clone + Copy + Send + Sync {}

#[derive(Debug, Clone, Copy)]
pub struct NanoErg {}

#[derive(Debug, Clone, Copy)]
pub struct Erg {}

#[derive(Debug, Clone, Copy)]
pub struct Usd {}

impl Asset for Erg {}
impl Asset for NanoErg {}
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

#[derive(Debug, Clone, Copy)]
pub struct AssetsExchangeRate<PER1: Asset, GET: Asset> {
    pub per1: PER1,
    pub get: GET,
    pub rate: f64,
}
