use super::assets_exchange_rate::AssetsExchangeRate;
use super::assets_exchange_rate::Usd;
use super::erg_xau::KgAu;
use super::DataPointSourceError;

#[derive(Debug, Clone)]
pub struct BitPanda {}

pub async fn get_kgau_usd() -> Result<AssetsExchangeRate<KgAu, Usd>, DataPointSourceError> {
    let url = "https://api.bitpanda.com/v1/ticker";
    let resp = reqwest::get(url).await?;
    let json = json::parse(&resp.text().await?)?;
    if let Some(p) = json["XAU"]["USD"].as_str() {
        // USD price of 1 XAU
        let p_float = p
            .parse::<f64>()
            .map_err(|_| DataPointSourceError::JsonMissingField {
                field: "XAU.USD as f64".to_string(),
                json: json.dump(),
            })?;
        let usd_per_kgau = KgAu::from_xau(p_float);
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
mod tests {
    use super::*;

    #[test]
    fn test_kgau_usd_price() {
        let pair: AssetsExchangeRate<KgAu, Usd> = tokio_test::block_on(get_kgau_usd()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
