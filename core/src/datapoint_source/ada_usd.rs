//! Obtains the lovelace per 1 USD rate.

use super::{DataPointSource, DataPointSourceError};

#[derive(Debug, Clone)]
pub struct NanoAdaUsd;

impl DataPointSource for NanoAdaUsd {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        get_nanoada_usd_price()
    }
}

// Number of Lovelaces in a single Ada
static LOVELACE_CONVERSION: f64 = 1000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=cardano&vs_currencies=USD";

/// Get the Ada/USD price from the Lovelaces per 1 USD datapoint price
pub fn generate_current_price(datapoint: u64) -> f64 {
    (1.0 / datapoint as f64) * LOVELACE_CONVERSION
}

/// Acquires the price of Ada in USD from CoinGecko, convert it
/// into Lovelaces per 1 USD, and return it.
fn get_nanoada_usd_price() -> Result<i64, DataPointSourceError> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["cardano"]["usd"].as_f64() {
        let lovelace_price = (1.0 / p) * LOVELACE_CONVERSION;
        Ok(lovelace_price as i64)
    } else {
        Err(DataPointSourceError::JsonMissingField)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ada_usd_price() {
        let n = NanoAdaUsd {};
        assert!(n.get_datapoint().unwrap() > 0);
    }
}
