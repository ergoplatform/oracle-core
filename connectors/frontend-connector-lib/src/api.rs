use crate::FrontendConnector;
use connector_lib::get_core_api_port;
use sincere;

/// Starts the Frontend GET API server which can be made publicly available
pub fn start_get_api(frontend_connector: FrontendConnector) {
    let mut app = sincere::App::new();

    // Basic welcome endpoint
    app.get("/", move |context| {
        let response_text = format!(
            "This is an Oracle Core FrontendConnector. Please use the `/frontendData` endpoint to fetch relevant data.\n"
        );
        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_text(response_text)
            .unwrap();
    });

    // All useful data for the Oracle Pool frontend
    app.get("/frontendData", move |context| {
        let res_json = frontend_connector.prepare_frontend_data_json();
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
