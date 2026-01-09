use super::assets_exchange_rate::Btc;
use super::assets_exchange_rate::Usd;
use super::assets_exchange_rate::{AssetsExchangeRate, NanoErg};
use super::erg_xau::KgAu;
use super::DataPointSourceError;

#[derive(Debug, Clone)]
pub struct BitPanda {}

#[cfg(not(test))]
pub async fn get_kgau_usd() -> Result<AssetsExchangeRate<KgAu, Usd>, DataPointSourceError> {
    let url = "https://api.bitpanda.com/v1/ticker";
    let resp = reqwest::get(url).await?;
    let json = json::parse(&resp.text().await?)?;
    if let Some(p) = json["XAU"]["USD"].as_str() {
        // USD price of 1 gram of gold
        let p_float = p
            .parse::<f64>()
            .map_err(|_| DataPointSourceError::JsonMissingField {
                field: "XAU.USD as f64".to_string(),
                json: json.dump(),
            })?;
        let usd_per_kgau = KgAu::from_gram(p_float);
        let rate = AssetsExchangeRate {
            per1: KgAu {},
            get: Usd {},
            rate: usd_per_kgau,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "XAU.USD".to_string(),
            json: json.dump(),
        })
    }
}

#[cfg(test)]
pub async fn get_kgau_usd() -> Result<AssetsExchangeRate<KgAu, Usd>, DataPointSourceError> {
    // USD price of 1 gram of gold
    let p_float = 66.10;
    let usd_per_kgau = KgAu::from_gram(p_float);
    let rate = AssetsExchangeRate {
        per1: KgAu {},
        get: Usd {},
        rate: usd_per_kgau,
    };
    Ok(rate)
}

#[cfg(not(test))]
// Get USD/BTC. Can be used as a redundant source for ERG/BTC through ERG/USD and USD/BTC
pub(crate) async fn get_btc_usd() -> Result<AssetsExchangeRate<Btc, Usd>, DataPointSourceError> {
    let url = "https://api.bitpanda.com/v1/ticker";
    let resp = reqwest::get(url).await?;
    let json = json::parse(&resp.text().await?)?;
    if let Some(p) = json["BTC"]["USD"].as_str() {
        // USD price of BTC
        let usd_per_btc = p
            .parse::<f64>()
            .map_err(|_| DataPointSourceError::JsonMissingField {
                field: "BTC.USD as f64".to_string(),
                json: json.dump(),
            })?;
        let rate = AssetsExchangeRate {
            per1: Btc {},
            get: Usd {},
            rate: usd_per_btc,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "BTC.USD".to_string(),
            json: json.dump(),
        })
    }
}

#[cfg(test)]
pub(crate) async fn get_btc_usd() -> Result<AssetsExchangeRate<Btc, Usd>, DataPointSourceError> {
    // USD price of BTC
    let usd_per_btc = 43827.02;
    let rate = AssetsExchangeRate {
        per1: Btc {},
        get: Usd {},
        rate: usd_per_btc,
    };
    Ok(rate)
}

#[cfg(not(test))]
pub async fn get_usd_nanoerg() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let url = "https://api.bitpanda.com/v1/ticker";
    let resp = reqwest::get(url).await?;
    let json = json::parse(&resp.text().await?)?;
    if let Some(p) = json["ERG"]["USD"].as_str() {
        // USD price of ERG
        let usd_per_erg = p
            .parse::<f64>()
            .map_err(|_| DataPointSourceError::JsonMissingField {
                field: "ERG.USD as f64".to_string(),
                json: json.dump(),
            })?;
        let nanoerg_per_usd = NanoErg::from_erg(1.0 / usd_per_erg);
        let rate = AssetsExchangeRate {
            per1: Usd {},
            get: NanoErg {},
            rate: nanoerg_per_usd,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "ERG.USD".to_string(),
            json: json.dump(),
        })
    }
}

#[cfg(test)]
pub async fn get_usd_nanoerg() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    // Convert from price Erg/USD to nanoErgs per 1 USD
    let nanoerg_per_usd = NanoErg::from_erg(1.0 / 1.669);
    let rate = AssetsExchangeRate {
        per1: Usd {},
        get: NanoErg {},
        rate: nanoerg_per_usd,
    };
    Ok(rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kgau_usd_price() {
        let pair: AssetsExchangeRate<KgAu, Usd> = tokio_test::block_on(get_kgau_usd()).unwrap();
        assert!(pair.rate > 0.0);
    }
    #[test]
    fn test_btc_usd_price() {
        let pair: AssetsExchangeRate<Btc, Usd> = tokio_test::block_on(get_btc_usd()).unwrap();
        assert!(pair.rate > 0.0);
    }

    #[test]
    fn test_erg_usd_price() {
        let pair: AssetsExchangeRate<Usd, NanoErg> =
            tokio_test::block_on(get_usd_nanoerg()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
