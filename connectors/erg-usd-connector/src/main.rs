/// This Connector obtains the nanoErg per 1 USD rate and submits it
/// to an oracle core. It reads the `oracle-config.yaml` to find the port
/// of the oracle core (via Connector-Lib) and submits it to the POST API
/// server on the core.
/// Note: The value that is posted on-chain is the number
/// of nanoErgs per 1 USD, not the rate per nanoErg.
#[macro_use]
mod connector_config;

use anyhow::{anyhow, Result};
use frontend_connector_lib::FrontendConnector;
use connector_config::{get_cmc_api_key};

// Number of nanoErgs in a single Erg
static NANO_ERG_CONVERSION: f64 = 1000000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD";

static CMC_RATE_URL: &str =
    "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?convert=USD&&symbol=ERG";

/// Get the Erg/USD price from the nanoErgs per 1 USD datapoint price
pub fn generate_current_price(datapoint: u64) -> f64 {
    (1.0 / datapoint as f64) * NANO_ERG_CONVERSION
}

/// Acquires the price of Ergs in USD from CoinGecko and CoinMarketCap, convert it
/// into nanoErgs per 1 USD, and return it.
fn get_nanoerg_usd_price() -> Result<u64> {
    let resp_cg = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let resp_cmc = reqwest::blocking::Client::new().get(CMC_RATE_URL).header("X-CMC_PRO_API_KEY", get_cmc_api_key()).send()?;
    let price_json_cg = json::parse(&resp_cg.text()?)?;
    let price_json_cmc = json::parse(&resp_cmc.text()?)?;
    fn convert_from_price(price: Option<f64>) -> u64 {
        if let Some(p) = price {
            // Convert from price Erg/USD to nanoErgs per 1 USD
            let nanoerg_price =  (1.0 / p) * NANO_ERG_CONVERSION;
            return nanoerg_price as u64;
        } else {
            0 as u64
        }
    }
    let price_cg = convert_from_price(price_json_cg["ergo"]["usd"].as_f64());
    let price_cmc = convert_from_price(price_json_cmc["data"]["ERG"]["quote"]["USD"]["price"].as_f64());
    if price_cg == 0 || price_cmc == 0 {
        return Err(anyhow!("Failed to parse price from json."));
    }
    let nanoerg_price = (price_cg + price_cmc) / 2;
    return Ok(nanoerg_price as u64);
}

fn main() {
    // Create the FrontendConnector
    let connector = FrontendConnector::new_basic_connector(
        "Erg-USD",
        get_nanoerg_usd_price,
        generate_current_price,
    );

    // Start the FrontendConnector
    connector.run();
}
