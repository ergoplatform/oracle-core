/// This Connector obtains the nanoErg per 1 USD rate and submits it
/// to an oracle core. It reads the `oracle-config.yaml` to find the port
/// of the oracle core (via Connector-Lib) and submits it to the POST API
/// server on the core.
/// Note: The value that is posted on-chain is the number
/// of nanoErgs per 1 USD, not the rate per nanoErg.
use anyhow::{anyhow, Result};
use frontend_connector_lib::FrontendConnector;

// Number of nanoErgs in a single Erg
static NANO_ERG_CONVERSION: f64 = 1000000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD";

/// Get the Erg/USD price from the nanoErgs per 1 USD datapoint price
pub fn generate_current_price(datapoint: u64) -> f64 {
    (1.0 / datapoint as f64) * NANO_ERG_CONVERSION
}

/// Acquires the price of Ergs in USD from CoinGecko, convert it
/// into nanoErgs per 1 USD, and return it.
fn get_nanoerg_usd_price() -> Result<u64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["ergo"]["usd"].as_f64() {
        // Convert from price Erg/USD to nanoErgs per 1 USD
        let nanoerg_price = (1.0 / p) * NANO_ERG_CONVERSION;
        Ok(nanoerg_price as u64)
    } else {
        Err(anyhow!("Failed to parse price from json."))
    }
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
