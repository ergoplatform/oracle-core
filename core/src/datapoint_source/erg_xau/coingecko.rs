use futures::future::BoxFuture;

use crate::datapoint_source::aggregator::DataPointFetcher;
use crate::datapoint_source::DataPointSourceError;
use crate::datapoint_source::KgAu;
use crate::datapoint_source::NanoErg;
use crate::datapoint_source::Rate;
use crate::datapoint_source::RateSource;

#[derive(Debug, Clone)]
pub struct CoinGecko;

impl DataPointFetcher for CoinGecko {
    fn get_datapoint(&self) -> BoxFuture<'static, Result<i64, DataPointSourceError>> {
        Box::pin(get_nanoerg_xau_price())
    }
}

impl RateSource<KgAu, NanoErg> for CoinGecko {
    fn get_rate(&self) -> BoxFuture<Result<Rate<KgAu, NanoErg>, DataPointSourceError>> {
        Box::pin(get_kgau_nanoerg())
    }
}

// Number of nanoErgs in a single Erg
static NANO_ERG_CONVERSION: f64 = 1000000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=XAU";

/// Acquires the price of Ergs in XAU from CoinGecko, convert it into nanoErgs per 1 XAU (troy ounce
/// of gold), and return it.
async fn get_nanoerg_xau_price() -> Result<i64, DataPointSourceError> {
    let resp = reqwest::get(CG_RATE_URL).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["ergo"]["xau"].as_f64() {
        // Convert from price Erg/XAU to nanoErgs per 1 XAU
        let nanoerg_price = (1.0 / p) * NANO_ERG_CONVERSION;
        Ok(nanoerg_price as i64)
    } else {
        Err(DataPointSourceError::JsonMissingField)
    }
}

async fn get_kgau_nanoerg() -> Result<Rate<KgAu, NanoErg>, DataPointSourceError> {
    let resp = reqwest::get(CG_RATE_URL).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["ergo"]["xau"].as_f64() {
        // Convert from price Erg/XAU to nanoErgs per 1 XAU
        let nanoerg_price = (1.0 / p) * NANO_ERG_CONVERSION;
        let per_kgau = nanoerg_price * 32.1507466;
        let rate = Rate {
            l: KgAu {},
            r: NanoErg {},
            rate: per_kgau,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erg_xau_price() {
        let n = CoinGecko {};
        let price = tokio_test::block_on(n.get_datapoint()).unwrap();
        assert!(price > 0);
    }
}
