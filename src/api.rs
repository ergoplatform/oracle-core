use crate::encoding::serialize_int;
use crate::node_interface::current_block_height;
use crate::oracle_config::{get_core_api_port, get_node_url, PoolParameters};
use crate::oracle_state::{OraclePool, PoolBoxState};
use json;
use sincere;
use std::str::from_utf8;

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
            oracle_payout_price: parameters.oracle_payout_price,
            live_epoch_length: parameters.live_epoch_length,
            epoch_prep_length: parameters.epoch_preparation_length,
            margin_of_error: parameters.margin_of_error,
            number_of_oracles: parameters.number_of_oracles,
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

    // Status of the oracle
    app.get("/oracleStatus", move |context| {
        let op = OraclePool::new();

        // Check whether waiting for datapoint to be submit to oracle core
        let waiting_for_submit = match op.get_live_epoch_state() {
            Ok(l) => !l.commit_datapoint_in_epoch,
            Err(_) => false,
        };
        // Get latest datapoint the local oracle produced/submit
        let self_datapoint = match op.get_datapoint_state() {
            Ok(d) => d.datapoint,
            Err(_) => 0,
        };
        // Get latest datapoint submit epoch
        let datapoint_epoch = match op.get_datapoint_state() {
            Ok(d) => d.origin_epoch_id,
            Err(_) => "Null".to_string(),
        };
        // Get latest datapoint submit epoch
        let datapoint_creation = match op.get_datapoint_state() {
            Ok(d) => d.creation_height,
            Err(_) => 0,
        };

        let response_json = object! {
            waiting_for_datapoint_submit: waiting_for_submit,
            latest_datapoint: self_datapoint,
            latest_datapoint_epoch: datapoint_epoch,
            latest_datapoint_creation_height: datapoint_creation,
        };

        context.response.from_json(response_json.dump()).unwrap();
    });

    // Status of the oracle pool
    app.get("/poolStatus", move |context| {
        let op = OraclePool::new();
        let parameters = PoolParameters::new();

        // Current stage of the oracle pool box
        let current_stage = match op.check_oracle_pool_stage() {
            PoolBoxState::LiveEpoch => "Live Epoch",
            PoolBoxState::Preparation => "Epoch Preparation",
        };

        // The amount percentage that the pool is funded
        let funded_percentage = if let Ok(l) = op.get_live_epoch_state() {
            (l.funds / (parameters.number_of_oracles * parameters.oracle_payout_price)) * 100
        } else if let Ok(ep) = op.get_preparation_state() {
            (ep.funds / (parameters.number_of_oracles * parameters.oracle_payout_price)) * 100
        } else {
            0
        };

        let response_json = object! {
            funded_percentage: funded_percentage,
            current_pool_stage: current_stage,
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

    // Accept a datapoint to be posted within a "Commit Datapoint" action tx
    app.post("/submitDatapoint", move |context| {
        let op = OraclePool::new();
        let res_post_json = from_utf8(context.request.body()).map(|t| json::parse(t));

        // If the post request body is valid json
        if let Ok(Ok(post_json)) = res_post_json {
            // If the datapoint provided is a valid Integer
            if let Ok(datapoint) = post_json["datapoint"].to_string().parse() {
                // Check if in Live Epoch stage
                if let PoolBoxState::LiveEpoch = op.check_oracle_pool_stage() {
                    let action_result = op.action_commit_datapoint(datapoint);
                    // If transaction succeeded being posted
                    if let Ok(res) = action_result{
                        let tx_id: String = res.chars().filter(|&c| c != '\"').collect();
                        let resp_json = object! {tx_id: tx_id}.to_string();

                        println!("-----\n`Commit Datapoint` Transaction Has Been Posted.\n-----");

                        context.response.from_json(resp_json).unwrap();
                    }
                    // If transaction failed being posted
                    else {
                        let error_json = object! {error: "Failed to post 'Commit Datapoint' action transaction."}.to_string();

                        context.response.from_json(error_json).unwrap();
                    }
                }
                // Else if in Epoch Prep stage
                else {
                    let error_json = object! {error: "Unable to submit Datapoint. The Oracle Pool is currently in the Epoch Preparation Stage."}.to_string();

                    context.response.from_json(error_json).unwrap();
                }
            }
            // If the datapoint provided is not a valid i32 Integer
            else {
                let error_json = object! {error: "Invalid Datapoint Provided. Please ensure that your request includes a valid Integer i32 'datapoint' field."}.to_string();

                context.response.from_json(error_json).unwrap();
            }

        }
        // If the post request body is not valid json
        else {
            let error_json = object! {error: "Invalid JSON Request Body."}.to_string();

            context.response.from_json(error_json).unwrap();
        }
    });

    // Start the API server with the port designated in the config.
    app.run(&("0.0.0.0:".to_string() + &get_core_api_port()), 1)
        .ok();
}
