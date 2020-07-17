use crate::node_interface::current_block_height;
use crate::oracle_config::{get_api_port, PoolParameters};
use crate::oracle_state::OraclePool;
use crossbeam::channel;
use sincere;
use std::panic::catch_unwind;

/// Starts the API server
pub fn start_api() {
    let mut app = sincere::App::new();
    let parameters = PoolParameters::new();
    let op = OraclePool::new();

    app.get("/", move |context| {
        let response_text = format!(
            "This is an Oracle Core. Please use one of the endpoints to interact with it.\n"
        );
        context.response.from_text(response_text).unwrap();
    });

    app.get("/info", move |context| {
        let response_text = format!(
            "Local Oracle Address: {}\n
            ",
            op.local_oracle_address
        );
        context.response.from_text(response_text).unwrap();
    });

    app.get("/blockheight", move |context| {
        let current_height =
            current_block_height().expect("Please ensure that the Ergo node is running.");
        let response_text = format!("{}", current_height);
        context.response.from_text(response_text).unwrap();
    });

    // Start the API server with the port designated in the config.
    app.run(&("0.0.0.0:".to_string() + &get_api_port()), 1).ok();
}
