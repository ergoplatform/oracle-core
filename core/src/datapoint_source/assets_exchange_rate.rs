pub trait Asset: Clone + Copy + Send + Sync {}

#[derive(Debug, Clone, Copy)]
pub struct NanoErg {}

#[derive(Debug, Clone, Copy)]
pub struct Erg {}

#[derive(Debug, Clone, Copy)]
pub struct Usd {}

#[derive(Debug, Clone, Copy)]
pub struct Btc {}

impl Asset for Erg {}
impl Asset for NanoErg {}
impl Asset for Usd {}
impl Asset for Btc {}

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

// Calculates an Exchange Rate of GET/PER2 based on GET/PER1 and PER1/PER2
pub fn convert_rate<GET: Asset, PER1: Asset, PER2: Asset>(
    a: AssetsExchangeRate<PER1, GET>,
    b: AssetsExchangeRate<PER2, PER1>,
) -> AssetsExchangeRate<PER2, GET> {
    AssetsExchangeRate {
        per1: b.per1,
        get: a.get,
        rate: a.rate * b.rate,
    }
}
