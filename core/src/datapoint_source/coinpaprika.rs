use crate::datapoint_source::assets_exchange_rate::{AssetsExchangeRate, Btc, NanoErg, Usd};
use crate::datapoint_source::DataPointSourceError;

#[cfg(not(test))]
pub async fn get_usd_nanoerg() -> Result<AssetsExchangeRate<Usd, NanoErg>, DataPointSourceError> {
    let url = "https://api.coinpaprika.com/v1/tickers/efyt-ergo";
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["quotes"]["USD"]["price"].as_f64() {
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
    let nanoerg_per_usd = NanoErg::from_erg(1.0 / 1.669);
    let rate = AssetsExchangeRate {
        per1: Usd {},
        get: NanoErg {},
        rate: nanoerg_per_usd,
    };
    Ok(rate)
}

#[cfg(not(test))]
// Get USD/BTC. Can be used as a redundant source for ERG/BTC through ERG/USD and USD/BTC
pub async fn get_btc_usd() -> Result<AssetsExchangeRate<Btc, Usd>, DataPointSourceError> {
    let url = "https://api.coinpaprika.com/v1/tickers/btc-bitcoin";
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    let price_json = json::parse(&resp.text().await?)?;
    if let Some(p) = price_json["quotes"]["USD"]["price"].as_f64() {
        let rate = AssetsExchangeRate {
            per1: Btc {},
            get: Usd {},
            rate: p,
        };
        Ok(rate)
    } else {
        Err(DataPointSourceError::JsonMissingField {
            field: "quotes.USD.price as f64".to_string(),
            json: price_json.dump(),
        })
    }
}

#[cfg(test)]
pub async fn get_btc_usd() -> Result<AssetsExchangeRate<Btc, Usd>, DataPointSourceError> {
    let usd_per_btc = 43_712.768_005_075_37;
    let rate = AssetsExchangeRate {
        per1: Btc {},
        get: Usd {},
        rate: usd_per_btc,
    };
    Ok(rate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datapoint_source::bitpanda;

    #[test]
    fn test_erg_usd_price() {
        let pair: AssetsExchangeRate<Usd, NanoErg> =
            tokio_test::block_on(get_usd_nanoerg()).unwrap();
        assert!(pair.rate > 0.0);
    }

    #[test]
    fn test_usd_btc_price() {
        let pair = tokio_test::block_on(get_btc_usd()).unwrap();
        let bitpanda = tokio_test::block_on(bitpanda::get_btc_usd()).unwrap();
        assert!(pair.rate > 0.0);
        dbg!(pair, bitpanda);
        let deviation_from_bitpanda = (pair.rate - bitpanda.rate).abs() / bitpanda.rate;
        assert!(
            deviation_from_bitpanda < 0.05,
            "up to 5% deviation is allowed"
        );
    }
}
