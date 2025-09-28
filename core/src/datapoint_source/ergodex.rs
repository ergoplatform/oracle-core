use crate::datapoint_source::assets_exchange_rate::{AssetsExchangeRate, NanoErg};
use crate::datapoint_source::rsn_xag::Rsn;
use crate::datapoint_source::DataPointSourceError;

pub async fn get_rsn_nanoerg() -> Result<AssetsExchangeRate<NanoErg, Rsn>, DataPointSourceError> {
    let url = "https://api.spectrum.fi/v1/amm/pool/1b694b15467c62f0cd4525e368dbdea2329c713aa200b73df4a622e950551b40/stats";
    let resp = reqwest::get(url).await?;
    let pool_json = json::parse(&resp.text().await?)?;
    let locked_erg = pool_json["lockedX"]["amount"].as_f64().ok_or_else(|| {
        DataPointSourceError::JsonMissingField {
            field: "lockedX.amount as f64".to_string(),
            json: pool_json.dump(),
        }
    })?;

    let locked_rsn = pool_json["lockedY"]["amount"].as_f64().ok_or_else(|| {
        DataPointSourceError::JsonMissingField {
            field: "lockedY.amount as f64".to_string(),
            json: pool_json.dump(),
        }
    })?;
    let price = Rsn::from_rsn(Rsn::from_rsn(locked_rsn) / NanoErg::from_erg(locked_erg));
    let rate = AssetsExchangeRate {
        per1: NanoErg {},
        get: Rsn {},
        rate: price,
    };
    Ok(rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsn_nanoerg_price() {
        let pair: AssetsExchangeRate<NanoErg, Rsn> =
            tokio_test::block_on(get_rsn_nanoerg()).unwrap();
        assert!(pair.rate > 0.0);
    }
}
