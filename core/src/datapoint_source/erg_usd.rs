//! Obtains the nanoErg per 1 USD rate

use super::{DataPointSource, DataPointSourceError};

#[derive(Debug, Clone)]
pub struct NanoErgUsd;

impl DataPointSource for NanoErgUsd {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        get_nanoerg_usd_price()
    }
}

// Number of nanoErgs in a single Erg
static NANO_ERG_CONVERSION: f64 = 1000000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD";

/// Acquires the price of Ergs in USD from CoinGecko, convert it
/// into nanoErgs per 1 USD, and return it.
fn get_nanoerg_usd_price() -> Result<i64, DataPointSourceError> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["ergo"]["usd"].as_f64() {
        // Convert from price Erg/USD to nanoErgs per 1 USD
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
    fn test_erg_usd_price() {
        let n = NanoErgUsd {};
        assert!(n.get_datapoint().unwrap() > 0);
    }
}
