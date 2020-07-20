use crate::node_interface::current_block_height;
use crate::oracle_config::{get_api_port, get_node_url, PoolParameters};
use crate::oracle_state::OraclePool;
use crossbeam::channel;
use sincere;
use std::panic::catch_unwind;

/// Starts the API server
pub fn start_api() {
    let mut app = sincere::App::new();

    // Basic welcome endpoint
    app.get("/", move |context| {
        let response_text = format!(
            "This is an Oracle Core. Please use one of the endpoints to interact with it.\n"
        );
        context.response.from_text(response_text).unwrap();
    });

    // Basic oracle information
    app.get("/oracleInfo", move |context| {
        let op = OraclePool::new();
        let response_json = object! {
            oracle_address: op.local_oracle_address,
        };

        context.response.from_json(response_json.dump()).unwrap();
    });

    // Basic information about the oracle pool
    app.get("/poolInfo", move |context| {
        let op = OraclePool::new();
        let parameters = PoolParameters::new();

        let response_json = object! {
            live_epoch_address : op.live_epoch_stage.contract_address,
            epoch_prep_address: op.epoch_preparation_stage.contract_address,
            pool_deposits_address: op.pool_deposit_stage.contract_address,
            datapoint_address: op.datapoint_stage.contract_address,
            posting_price: parameters.posting_price,
            live_epoch_length: parameters.live_epoch_length,
            epoch_prep_length: parameters.epoch_preparation_length,
            margin_of_error: parameters.margin_of_error,
            oracle_pool_nft_id: op.oracle_pool_nft,
            oracle_pool_participant_token_id: op.oracle_pool_participant_token,

        };

        context.response.from_json(response_json.dump()).unwrap();
    });

    // Basic information about node the oracle core is using
    app.get("/nodeInfo", move |context| {
        let response_json = object! {
            node_url: get_node_url(),
        };

        context.response.from_json(response_json.dump()).unwrap();
    });

    // Block height of the Ergo blockchain
    app.get("/blockHeight", move |context| {
        let current_height =
            current_block_height().expect("Please ensure that the Ergo node is running.");
        let response_text = format!("{}", current_height);
        context.response.from_text(response_text).unwrap();
    });

    // Start the API server with the port designated in the config.
    app.run(&("0.0.0.0:".to_string() + &get_api_port()), 1).ok();
}
