use super::ada_usd::Lovelace;
use super::assets_exchange_rate::Btc;
use super::assets_exchange_rate::Usd;
use super::erg_xag::KgAg;
use super::erg_xau::KgAu;
use crate::datapoint_source::assets_exchange_rate::AssetsExchangeRate;
use crate::datapoint_source::assets_exchange_rate::NanoErg;
use crate::datapoint_source::rsn_xag::Rsn;
use crate::datapoint_source::DataPointSourceError;

#[cfg(not(test))]
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

#[cfg(test)]
pub async fn get_kgau_nanoerg() -> Result<AssetsExchangeRate<KgAu, NanoErg>, DataPointSourceError> {
    let nanoerg_per_troy_ounce = NanoErg::from_erg(1.0 / 0.0482);
    let nanoerg_per_kg = KgAu::from_troy_ounce(nanoerg_per_troy_ounce);
    let rate = AssetsExchangeRate {
        per1: KgAu {},
        get: NanoErg {},
        rate: nanoerg_per_kg,
    };
    Ok(rate)
}

#[cfg(not(test))]
pub async fn get_kgag_nanoerg() -> Result<AssetsExchangeRate<KgAg, NanoErg>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=XAG";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["ergo"]["xag"].as_f64() {
        // Convert from price Erg/XAG to nanoErgs per 1 XAG
        let nanoerg_per_troy_ounce = NanoErg::from_erg(1.0 / p);
        let nanoerg_per_kg = KgAg::from_troy_ounce(nanoerg_per_troy_ounce);
        let rate = AssetsExchangeRate {
            per1: KgAg {},
            get: NanoErg {},
            rate: nanoerg_per_kg,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "ergo.xag as f64".to_string(),
            json: price_json.dump(),
        })
    }
}

#[cfg(test)]
pub async fn get_kgag_nanoerg() -> Result<AssetsExchangeRate<KgAg, NanoErg>, DataPointSourceError> {
    let nanoerg_per_troy_ounce = NanoErg::from_erg(1.0 / 0.0706);
    let nanoerg_per_kg = KgAg::from_troy_ounce(nanoerg_per_troy_ounce);
    let rate = AssetsExchangeRate {
        per1: KgAg {},
        get: NanoErg {},
        rate: nanoerg_per_kg,
    };
    Ok(rate)
}

#[cfg(not(test))]
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

#[cfg(test)]
pub async fn get_usd_nanoerg() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    // Convert from price Erg/USD to nanoErgs per 1 USD
    let nanoerg_per_usd = NanoErg::from_erg(1.0 / 1.67);
    let rate = AssetsExchangeRate {
        per1: Usd {},
        get: NanoErg {},
        rate: nanoerg_per_usd,
    };
    Ok(rate)
}

#[cfg(not(test))]
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
pub async fn get_usd_lovelace() -> Result<AssetsExchangeRate<Usd, Lovelace>, DataPointSourceError> {
    // Convert from price Erg/USD to nanoErgs per 1 USD
    let lovelace_price = Lovelace::from_ada(1.0 / 0.606545);
    let rate = AssetsExchangeRate {
        per1: Usd {},
        get: Lovelace {},
        rate: lovelace_price,
    };
    Ok(rate)
}

#[cfg(not(test))]
pub async fn get_btc_nanoerg() -> Result<AssetsExchangeRate<Btc, NanoErg>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=BTC";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["ergo"]["btc"].as_f64() {
        // Convert from price BTC/ERG to nanoERG/BTC
        let erg_per_usd = NanoErg::from_erg(1.0 / p);
        let rate = AssetsExchangeRate {
            per1: Btc {},
            get: NanoErg {},
            rate: erg_per_usd,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "ergo.btc as f64".to_string(),
            json: price_json.dump(),
        })
    }
}

#[cfg(test)]
pub async fn get_btc_nanoerg() -> Result<AssetsExchangeRate<Btc, NanoErg>, DataPointSourceError> {
    // Convert from price BTC/ERG to nanoERG/BTC
    let erg_per_usd = NanoErg::from_erg(1.0 / 0.00003791);
    let rate = AssetsExchangeRate {
        per1: Btc {},
        get: NanoErg {},
        rate: erg_per_usd,
    };
    Ok(rate)
}

pub async fn get_kgag_rsn() -> Result<AssetsExchangeRate<KgAg, Rsn>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=rosen-bridge&vs_currencies=XAG";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["rosen-bridge"]["xag"].as_f64() {
        // Convert from price RSN/XAG
        let rsn_per_ag = KgAg::from_troy_ounce(1.0 / p);
        let rate = AssetsExchangeRate {
            per1: KgAg {},
            get: Rsn {},
            rate: rsn_per_ag,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "rsn.xag as f64".to_string(),
            json: price_json.dump(),
        })
    }
}

pub async fn get_rsn_usd() -> Result<AssetsExchangeRate<Usd, Rsn>, DataPointSourceError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=rosen-bridge&vs_currencies=USD";
    let resp = reqwest::get(url).await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["rosen-bridge"]["usd"].as_f64() {
        let rate = AssetsExchangeRate {
            per1: Usd {},
            get: Rsn {},
            rate: 1.0 / p,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "rsn.usd as f64".to_string(),
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
    fn test_erg_xag_price() {
        let pair: AssetsExchangeRate<KgAg, NanoErg> =
            tokio_test::block_on(get_kgag_nanoerg()).unwrap();
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
    #[test]
    fn test_erg_btc_price() {
        let pair: AssetsExchangeRate<Btc, NanoErg> =
            tokio_test::block_on(get_btc_nanoerg()).unwrap();
        assert!(pair.rate > 0.0);
    }

    #[test]
    fn test_rsn_xag_price() {
        let pair: AssetsExchangeRate<KgAg, Rsn> = tokio_test::block_on(get_kgag_rsn()).unwrap();
        assert!(pair.rate > 0.0);
    }

    #[test]
    fn test_rsn_usd_price() {
        let pair: AssetsExchangeRate<Usd, Rsn> = tokio_test::block_on(get_rsn_usd()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
