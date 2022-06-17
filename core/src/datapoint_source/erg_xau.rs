//! Obtains the nanoErg per 1 XAU (troy ounce of gold) rate

use super::{DataPointSource, DataPointSourceError};

#[derive(Debug, Clone)]
pub struct NanoErgXau;

impl DataPointSource for NanoErgXau {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        get_nanoerg_xau_price()
    }
}

// Number of nanoErgs in a single Erg
static NANO_ERG_CONVERSION: f64 = 1000000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=XAU";

/// Acquires the price of Ergs in XAU from CoinGecko, convert it into nanoErgs per 1 XAU (troy ounce
/// of gold), and return it.
fn get_nanoerg_xau_price() -> Result<i64, DataPointSourceError> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["ergo"]["xau"].as_f64() {
        // Convert from price Erg/XAU to nanoErgs per 1 XAU
        let nanoerg_price = (1.0 / p) * NANO_ERG_CONVERSION;
        Ok(nanoerg_price as i64)
    } else {
        Err(DataPointSourceError::JsonMissingField)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erg_xau_price() {
        let n = NanoErgXau {};
        assert!(n.get_datapoint().unwrap() > 0);
    }
}
