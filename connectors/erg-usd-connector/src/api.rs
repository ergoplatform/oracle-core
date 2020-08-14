use json;
use sincere;
use std::str::from_utf8;

use connector_lib::{get_core_api_port, OracleCore};

/// Starts the ERG-USD GET API server which can be made publicly available without security risk
pub fn start_get_api() {
    let mut app = sincere::App::new();

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
