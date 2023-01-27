use crate::datapoint_source::assets_exchange_rate::AssetsExchangeRate;
use crate::datapoint_source::assets_exchange_rate::NanoErg;
use crate::datapoint_source::DataPointSourceError;

use super::ada_usd::Lovelace;
use super::assets_exchange_rate::Usd;
use super::erg_xau::KgAu;

pub async fn get_kgau_nanoerg() -> Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=XAU";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["ergo"]["xau"].as_f64() {
        // Convert from price Erg/XAU to nanoErgs per 1 XAU
        let nanoerg_per_troy_ounce = NanoErg::from_erg(1.0 / p);
        let nanoerg_per_kg = KgAu::from_troy_ounce(nanoerg_per_troy_ounce);
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

pub async fn get_usd_nanoerg() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
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

pub async fn get_usd_lovelace() -> Result<AssetsExchangeRate<Usd, Lovelace>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=cardano&vs_currencies=USD";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["cardano"]["usd"].as_f64() {
        // Convert from price Erg/USD to nanoErgs per 1 USD
        let lovelace_price = Lovelace::from_ada(1.0 / p);
        let rate = AssetsExchangeRate {
            per1: Usd {},
            get: Lovelace {},
            rate: lovelace_price,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "cardano.usd as f64".to_string(),
            json: price_json.dump(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erg_xau_price() {
        let pair: AssetsExchangeRate<KgAu, NanoErg> =
            tokio_test::block_on(get_kgau_nanoerg()).unwrap();
        assert!(pair.rate > 0.0);
    }

    #[test]
    fn test_erg_usd_price() {
        let pair: AssetsExchangeRate<Usd, NanoErg> =
            tokio_test::block_on(get_usd_nanoerg()).unwrap();
        assert!(pair.rate > 0.0);
    }

    #[test]
    fn test_ada_usd_price() {
        let pair: AssetsExchangeRate<Usd, Lovelace> =
            tokio_test::block_on(get_usd_lovelace()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
