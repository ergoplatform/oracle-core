use anyhow::Result;
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
pub fn get_usd_price(datapoint: u64) -> f64 {
    (1.0 / datapoint as f64) * 1000000000.0
}

/// Prepares the json
pub fn prepare_frontend_data_json(oc: &OracleCore) -> Result<String> {
    let pinfo = oc.pool_info()?;
    let pstatus = oc.pool_status()?;
    let block_height = oc.current_block_height()?;

    // Posting Schedule
    let posting_sched_blocks = pinfo.live_epoch_length + pinfo.epoch_prep_length;
    let posting_sched_minutes = posting_sched_blocks * 2;

    // How Long Until Epoch Ends
    let mut epoch_ends_in_minutes = 0;
    if pstatus.epoch_ends > block_height {
        epoch_ends_in_minutes = (pstatus.epoch_ends - block_height) * 2;
    }

    let data_json = object! {
        // Tile information
        latest_price: get_usd_price(pstatus.latest_datapoint),
        posting_schedule_minutes: posting_sched_minutes,
        epoch_ends_in_minutes: epoch_ends_in_minutes,
        current_pool_stage: pstatus.current_pool_stage,
        pool_funded_percentage: pstatus.funded_percentage,

        // Summary Table
        posting_schedule_blocks: posting_sched_blocks,


        // Technical
        minimum_pool_box_value: pinfo.minimum_pool_box_value,
        latest_datapoint: pstatus.latest_datapoint,
        live_epoch_address : pinfo.live_epoch_address,
        epoch_prep_address: pinfo.epoch_prep_address,
        pool_deposits_address: pinfo.pool_deposits_address,
        datapoint_address: pinfo.datapoint_address,
        oracle_payout_price: pinfo.oracle_payout_price,
        live_epoch_length: pinfo.live_epoch_length,
        epoch_prep_length: pinfo.epoch_prep_length,
        outlier_range: pinfo.outlier_range,
        oracle_pool_nft_id: pinfo.oracle_pool_nft_id,
        oracle_pool_participant_token_id: pinfo.oracle_pool_participant_token_id,
        epoch_end_height: pstatus.epoch_ends,
    };

    Ok(data_json.dump())
}
