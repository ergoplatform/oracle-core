/// This Connector obtains the lovelace per 1 USD rate and submits it
/// to an oracle core. It reads the `oracle-config.yaml` to find the port
/// of the oracle core (via Connector-Lib) and submits it to the POST API
/// server on the core.
/// Note: The value that is posted on-chain is the number
/// of lovelaces per 1 USD, not the rate per lovelace.
use anyhow::{anyhow, Result};
use frontend_connector_lib::FrontendConnector;

// Number of Lovelaces in a single Ada
static LOVELACE_CONVERSION: f64 = 1000000.0;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=cardano&vs_currencies=USD";

/// Get the Ada/USD price from the Lovelaces per 1 USD datapoint price
pub fn generate_current_price(datapoint: u64) -> f64 {
    (1.0 / datapoint as f64) * LOVELACE_CONVERSION
}

/// Acquires the price of Ada in USD from CoinGecko, convert it
/// into Lovelaces per 1 USD, and return it.
fn get_lovelace_usd_price() -> Result<u64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["cardano"]["usd"].as_f64() {
        let lovelace_price = (1.0 / p) * LOVELACE_CONVERSION;
        Ok(lovelace_price as u64)
    } else {
        Err(anyhow!("Failed to parse price from json."))
    }
}

fn main() {
    // Create the FrontendConnector
    let connector = FrontendConnector::new_basic_connector(
        "ADA-USD",
        get_lovelace_usd_price,
        generate_current_price,
    );

    // Start the FrontendConnector
    connector.run();
}
