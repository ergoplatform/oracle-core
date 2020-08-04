use anyhow::{anyhow, Result};
use connector_lib::{get_core_api_port, OracleCore};
/// This Connector obtains the nanoErg/USD rate and submits it
/// to an oracle core.
/// Note: The value that is posted on-chain is the number
/// of nanoErgs per 1 USD, not the rate per nanoErg.
use json;

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

fn main() {
    // Initialization
    let core_port = get_core_api_port().expect("Failed to read local `oracle-config.yaml`.");
    let oc = OracleCore::new("0.0.0.0", &core_port);

    // Print ASCII
    println!("{}", CONNECTOR_ASCII);

    let price_res = get_nanoerg_usd_price();
    if let Ok(price) = price_res {
        let submit_result = oc.submit_datapoint(price);
        println!("nanoErgs Per 1 USD: {}", price);
        println!("Submit Result: {:?}", submit_result);
    } else {
        println!("{:?}", price_res);
    }
}

/// Acquires the nanoErg/USD price from CoinGecko
fn get_nanoerg_usd_price() -> Result<u64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    let price = price_json["ergo"]["usd"].as_f64();
    if let Some(p) = price {
        let nanoErg_price = p * 1000000000.0;
        return Ok(nanoErg_price as u64);
    } else {
        Err(anyhow!("Failed to parse price."))
    }
}
