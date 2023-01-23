use futures::future::BoxFuture;

use crate::datapoint_source::assets_exchange_rate::AssetsExchangeRate;
use crate::datapoint_source::assets_exchange_rate::AssetsExchangeRateSource;
use crate::datapoint_source::assets_exchange_rate::KgAu;
use crate::datapoint_source::assets_exchange_rate::NanoErg;
use crate::datapoint_source::DataPointSourceError;

use super::assets_exchange_rate::Usd;

#[derive(Debug, Clone)]
pub struct CoinGecko;

impl AssetsExchangeRateSource<KgAu, NanoErg> for CoinGecko {
    fn get_rate(
        &self,
    ) -> BoxFuture<Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError>> {
        Box::pin(get_kgau_nanoerg())
    }
}

async fn get_kgau_nanoerg() -> Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=XAU";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["ergo"]["xau"].as_f64() {
        // Convert from price Erg/XAU to nanoErgs per 1 XAU
        let nanoerg_per_troy_ounce = NanoErg::from_erg(1.0 / p);
        let troy_ounces_in_kg = 32.1507466;
        let nanoerg_per_kg = nanoerg_per_troy_ounce * troy_ounces_in_kg;
        let rate = AssetsExchangeRate {
            per1: KgAu {},
            get: NanoErg {},
            rate: nanoerg_per_kg,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "ergo.xau as f64".to_string(),
            json: price_json.dump(),
        })
    }
}

impl AssetsExchangeRateSource<Usd, NanoErg> for CoinGecko {
    fn get_rate(
        &self,
    ) -> BoxFuture<Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError>> {
        Box::pin(get_usd_nanoerg())
    }
}

async fn get_usd_nanoerg() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["ergo"]["usd"].as_f64() {
        // Convert from price Erg/USD to nanoErgs per 1 USD
        let nanoerg_per_usd = NanoErg::from_erg(1.0 / p);
        let rate = AssetsExchangeRate {
            per1: Usd {},
            get: NanoErg {},
            rate: nanoerg_per_usd,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "ergo.usd as f64".to_string(),
            json: price_json.dump(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erg_xau_price() {
        let n = CoinGecko {};
        let pair: AssetsExchangeRate<KgAu, NanoErg> = tokio_test::block_on(n.get_rate()).unwrap();
        assert!(pair.rate > 0.0);
    }

    #[test]
    fn test_erg_usd_price() {
        let n = CoinGecko {};
        let pair: AssetsExchangeRate<Usd, NanoErg> = tokio_test::block_on(n.get_rate()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
