use futures::future::BoxFuture;

use crate::datapoint_source::DataPointSourceError;
use crate::datapoint_source::KgAu;
use crate::datapoint_source::NanoErg;
use crate::datapoint_source::Rate;
use crate::datapoint_source::RateSource;

#[derive(Debug, Clone)]
pub struct CoinGecko;

impl RateSource<KgAu, NanoErg> for CoinGecko {
    fn get_rate(&self) -> BoxFuture<Result<Rate<KgAu, NanoErg>, DataPointSourceError>> {
        Box::pin(get_kgau_nanoerg())
    }
}

// Number of nanoErgs in a single Erg
static NANO_ERG_CONVERSION: f64 = 1000000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=XAU";

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
        let pair = tokio_test::block_on(n.get_rate()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
