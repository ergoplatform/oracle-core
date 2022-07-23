use crate::node_interface::current_block_height;
use crate::oracle_config::{get_core_api_port, get_node_ip, get_node_port, ORACLE_CONFIG};
use crate::oracle_state::{OraclePool, StageDataSource};
use crate::state::PoolState;
use crossbeam::Receiver;
use serde_json::json;

/// Starts the GET API server which can be made publicly available without security risk
pub fn start_get_api(repost_receiver: Receiver<bool>) {
    let mut app = sincere::App::new();
    let op = OraclePool::new().unwrap();
    let datapoint_stage = op.datapoint_stage;

    // Basic welcome endpoint
    app.get("/", move |context| {
        let response_text =
            "This is an Oracle Core. Please use one of the endpoints to interact with it.\n"
                .to_string();
        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_text(response_text)
            .unwrap();
    });

    // Basic oracle information
    app.get("/oracleInfo", move |context| {
        let response_json = json! ( {
            "oracle_address": &ORACLE_CONFIG.oracle_address,
        } );

        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_json(response_json)
            .unwrap();
    });

    // Basic information about the oracle pool
    app.get("/poolInfo", move |context| {
        let parameters = &ORACLE_CONFIG;
        let num_of_oracles = datapoint_stage.stage.number_of_boxes().unwrap_or(10);

        let response_json = json! ({
            "number_of_oracles": num_of_oracles,
            "datapoint_address": datapoint_stage.stage.contract_address,
            "live_epoch_length": parameters.refresh_contract_parameters.epoch_length,
            "deviation_range": parameters.refresh_contract_parameters.max_deviation_percent,
            "consensus_num": parameters.refresh_contract_parameters.min_data_points,
            "oracle_pool_nft_id": parameters.token_ids.pool_nft_token_id,
            "oracle_pool_participant_token_id": parameters.token_ids.oracle_token_id,

        });

        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_json(response_json)
            .unwrap();
    });

    // Basic information about node the oracle core is using
    app.get("/nodeInfo", move |context| {
        let response_json = json! ( {
            "node_url": "http://".to_string() + &get_node_ip() + ":" + &get_node_port(),
        } );

        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_json(response_json)
            .unwrap();
    });

    // Status of the oracle
    app.get("/oracleStatus", move |context| {
        let op = OraclePool::new().unwrap();

        // Check whether waiting for datapoint to be submit to oracle core
        let waiting_for_submit = match op.get_live_epoch_state() {
            Ok(l) => !l.commit_datapoint_in_epoch,
            Err(_) => false,
        };
        // Get latest datapoint the local oracle produced/submit
        let self_datapoint = match op.get_datapoint_state() {
            Ok(Some(d)) => d.datapoint,
            Ok(None) | Err(_) => 0,
        };
        // Get latest datapoint submit epoch
        let datapoint_epoch = match op.get_datapoint_state() {
            Ok(Some(d)) => d.origin_epoch_id,
            Ok(None) | Err(_) => 0,
        };
        // Get latest datapoint submit epoch
        let datapoint_creation = match op.get_datapoint_state() {
            Ok(Some(d)) => d.creation_height,
            Ok(None) | Err(_) => 0,
        };

        let response_json = json! ( {
            "waiting_for_datapoint_submit": waiting_for_submit,
            "latest_datapoint": self_datapoint,
            "latest_datapoint_epoch": datapoint_epoch,
            "latest_datapoint_creation_height": datapoint_creation,
        } );

        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_json(response_json)
            .unwrap();
    });

    // Status of the oracle pool
    app.get("/poolStatus", move |context| {
        let op = OraclePool::new().unwrap();

        // Current stage of the oracle pool box
        let current_stage = match op.check_oracle_pool_stage() {
            PoolState::LiveEpoch(_) => "Live Epoch",
            PoolState::NeedsBootstrap => "Needs bootstrap",
        };

        let mut latest_datapoint = 0;
        let mut current_epoch_id = "".to_string();
        let mut epoch_ends = 0;
        if let Ok(l) = op.get_live_epoch_state() {
            latest_datapoint = l.latest_pool_datapoint;
            current_epoch_id = l.epoch_id.to_string();
            epoch_ends = l.epoch_ends;
        }
        let response_json = json! ( {
            "current_pool_stage": current_stage,
            "latest_datapoint": latest_datapoint,
            "current_epoch_id" : current_epoch_id,
            "epoch_ends": epoch_ends,
        } );

        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_json(response_json)
            .unwrap();
    });

    // Block height of the Ergo blockchain
    app.get("/blockHeight", move |context| {
        let current_height =
            current_block_height().expect("Please ensure that the Ergo node is running.");
        let response_text = format!("{}", current_height);
        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_text(response_text)
            .unwrap();
    });

    // Whether the Core requires the Connector to repost a new Datapoint
    app.get("/requireDatapointRepost", move |context| {
        let mut response_text = "false".to_string();
        if let Ok(b) = repost_receiver.try_recv() {
            response_text = b.to_string();
        }
        context
            .response
            .header(("Access-Control-Allow-Origin", "*"))
            .from_text(response_text)
            .unwrap();
    });

    // Start the API server with the port designated in the config.
    app.run(&("0.0.0.0:".to_string() + &get_core_api_port()), 1)
        .ok();
}
