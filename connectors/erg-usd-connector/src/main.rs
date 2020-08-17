/// This Connector obtains the nanoErg per 1 USD rate and submits it
/// to an oracle core. It reads the `oracle-config.yaml` to find the port
/// of the oracle core (via Connector-Lib) and submits it to the POST API
/// server on the core.
/// Note: The value that is posted on-chain is the number
/// of nanoErgs per 1 USD, not the rate per nanoErg.
#[macro_use]
extern crate json;

mod api;

use anyhow::{anyhow, Result};
use connector_lib::Connector;
use std::thread;

static CONNECTOR_ASCII: &str = r#"
 ______ _____   _____        _    _  _____ _____     _____                            _
|  ____|  __ \ / ____|      | |  | |/ ____|  __ \   / ____|                          | |
| |__  | |__) | |  __ ______| |  | | (___ | |  | | | |     ___  _ __  _ __   ___  ___| |_ ___  _ __
|  __| |  _  /| | |_ |______| |  | |\___ \| |  | | | |    / _ \| '_ \| '_ \ / _ \/ __| __/ _ \| '__|
| |____| | \ \| |__| |      | |__| |____) | |__| | | |___| (_) | | | | | | |  __/ (__| || (_) | |
|______|_|  \_\\_____|       \____/|_____/|_____/   \_____\___/|_| |_|_| |_|\___|\___|\__\___/|_|
==================================================================================================
"#;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD";

/// Get the Erg/USD price from the nanoErgs per 1 USD datapoint price
pub fn generate_current_price(datapoint: u64) -> f64 {
    (1.0 / datapoint as f64) * 1000000000.0
}

/// Acquires the price of Ergs in USD from CoinGecko and convert it
/// into nanoErgs per 1 USD.
fn get_nanoerg_usd_price() -> Result<u64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    if let Some(p) = price_json["ergo"]["usd"].as_f64() {
        let nanoerg_price = (1.0 / p as f64) * 1000000000.0;
        return Ok(nanoerg_price as u64);
    } else {
        Err(anyhow!("Failed to parse price from json."))
    }
}

fn main() {
    let connector = Connector::new_basic_connector("Erg-USD", get_nanoerg_usd_price);

    // Check if asked for bootstrap value
    connector.check_bootstrap();

    // Start Oracle Core GET API Server
    thread::Builder::new()
        .name("Erg-USD Connector API Thread".to_string())
        .spawn(move || {
            api::start_get_api();
        })
        .ok();

    // Start the Connector
    connector.run();
}
