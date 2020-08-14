use anyhow::Result;
use json;
use sincere;

use connector_lib::{get_core_api_port, OracleCore};

/// Starts the ERG-USD GET API server which can be made publicly available without security risk
pub fn start_get_api() {
    let mut app = sincere::App::new();
    let core_port =
        get_core_api_port().expect("Failed to read port from local `oracle-config.yaml`.");
    let oc = OracleCore::new("0.0.0.0", &core_port);

    // Basic welcome endpoint
    app.get("/", move |context| {
        let response_text = format!(
            "This is an Oracle Core Connector. Please use one of the endpoints to interact with it.\n"
        );
        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_text(response_text)
            .unwrap();
    });

    // All useful data for the Oracle Pool frontend
    app.get("/frontendData", move |context| {
        let res_json = prepare_frontend_data_json(&oc);
        // Succeeded acquiring all required data from Oracle Core
        if let Ok(response_json) = res_json {
            context
                .response
                .header(("Access-Control-Allow-Origin", "*"))
                .from_json(response_json)
                .unwrap();
        }
        // Failed to acquire all required data from Oracle Core
        else if let Err(e) = res_json {
            let error_json = object! {
                "error": e.to_string()
            };

            context
                .response
                .header(("Access-Control-Allow-Origin", "*"))
                .from_json(error_json.dump())
                .unwrap();
        }
    });

    // Start the API server with the port designated in the oracle config
    // plus two.
    let core_port = get_core_api_port().unwrap();
    let port = ((core_port
        .parse::<u16>()
        .expect("Failed to parse oracle core port from config to u16."))
        + 2)
    .to_string();
    let address = "0.0.0.0:".to_string() + &port;
    app.run(&address, 1).ok();
}

/// Get the Erg/USD price from the nanoErgs per 1 USD price
// pub fn get_usd_price() -> u64 {}

/// Prepares the json
pub fn prepare_frontend_data_json(oc: &OracleCore) -> Result<String> {
    let oinfo = oc.oracle_info();
    let pinfo = oc.pool_info();
    let pstatus = oc.pool_status();
    let block_height = oc.current_block_height();
    Ok(".".to_string())
}

//1. Latest Price
//[Show the Erg/USD price]
//2. Posting Schedule
//[Posting schedule in minutes]
//3. Epoch Ends
//[Number Of Minutes]
//4. Current Pool Stage
//[Pool Stage]
//5. Pool Funded Percentage
//[Pecentage]
