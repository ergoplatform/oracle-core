use anyhow::{anyhow, Result};
/// This Connector obtains the nanoErg/USD rate and submits it
/// to an oracle core.
/// Note: The value that is posted on-chain is the number
/// of nanoErgs per 1 USD, not the rate per nanoErg.
use json;
use connector_lib::{OracleCore, get_core_api_port};

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD";

fn main() {

    let oc = OracleCore::new("0.0.0.0",);


    let price_res = get_erg_usd_price();
    if let Ok(price) = price_res {
        println!("nanoErgs Per 1 USD: {}", price);
    } else {
        println!("{:?}", price_res);
    }
}

/// Acquires the nanoErg/USD price from CoinGecko
fn get_nanoErg_usd_price() -> Result<f64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    let price = price_json["ergo"]["usd"].as_f64();
    if let Some(p) = price {
        let nanoErg_price = p as f64 * 1000000000.0;
        return Ok(nanoErg_price);
    } else {
        Err(anyhow!("Failed to parse price."))
    }
}


fn get_core_port() -> Result<String> {
    let config = &YamlLoader::load_from_str(&get_config_yaml()).unwrap()[0];
    config["core_api_port"]
        .as_str()
        .expect("No core_api_port specified in config file.")
        .to_string()
}

