use std::net::SocketAddr;

use crate::box_kind::OracleBox;
use crate::node_interface::current_block_height;
use crate::oracle_config::{get_core_api_port, get_node_ip, get_node_port, ORACLE_CONFIG};
use crate::oracle_state::{OraclePool, StageDataSource};
use crate::state::PoolState;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use crossbeam::channel::Receiver;
use serde_json::json;
use tower_http::cors::CorsLayer;

/// Basic welcome endpoint
async fn root() -> &'static str {
    "This is an Oracle Core. Please use one of the endpoints to interact with it.\n"
}

/// Basic oracle information
async fn oracle_info() -> impl IntoResponse {
    Json(json! ( {
            "oracle_address": &ORACLE_CONFIG.oracle_address.to_base58(),
        } ))
}

/// Status of the oracle
async fn oracle_status() -> impl IntoResponse {
    let op = OraclePool::new().unwrap();

    // Check whether waiting for datapoint to be submit to oracle core
    let waiting_for_submit = match op.get_live_epoch_state() {
        Ok(l) => !l.commit_datapoint_in_epoch,
        Err(_) => false,
    };
    // Get latest datapoint the local oracle produced/submit
    let latest_oracle_box = op
        .get_local_datapoint_box_source()
        .get_local_oracle_datapoint_box();
    let self_datapoint = match latest_oracle_box {
        Ok(Some(ref d)) => d.rate(),
        Ok(None) | Err(_) => 0,
    };
    // Get latest datapoint submit epoch
    let datapoint_epoch = match latest_oracle_box {
        Ok(Some(ref d)) => d.epoch_counter(),
        Ok(None) | Err(_) => 0,
    };
    // Get latest datapoint submit epoch
    let datapoint_creation = match latest_oracle_box {
        Ok(Some(ref d)) => d.get_box().creation_height,
        Ok(None) | Err(_) => 0,
    };

    Json(json! ({
        "waiting_for_datapoint_submit": waiting_for_submit,
        "latest_datapoint": self_datapoint,
        "latest_datapoint_epoch": datapoint_epoch,
        "latest_datapoint_creation_height": datapoint_creation,
    }))
}

// Basic information about the oracle pool
async fn pool_info() -> impl IntoResponse {
    let parameters = &ORACLE_CONFIG;
    let op = OraclePool::new().unwrap();
    let datapoint_stage = op.datapoint_stage;
    let num_of_oracles = datapoint_stage.stage.number_of_boxes().unwrap_or(10);

    Json(json!({
        "number_of_oracles": num_of_oracles,
        "datapoint_address": datapoint_stage.stage.contract_address,
        "live_epoch_length": parameters.refresh_box_wrapper_inputs.contract_inputs.contract_parameters().epoch_length(),
        "deviation_range": parameters.refresh_box_wrapper_inputs.contract_inputs.contract_parameters().max_deviation_percent(),
        "consensus_num": parameters.refresh_box_wrapper_inputs.contract_inputs.contract_parameters().min_data_points(),
        "oracle_pool_nft_id": parameters.token_ids.pool_nft_token_id,
        "oracle_pool_participant_token_id": parameters.token_ids.oracle_token_id,

    }))
}

/// Basic information about node the oracle core is using
async fn node_info() -> impl IntoResponse {
    Json(json!({
        "node_url": "http://".to_string() + &get_node_ip() + ":" + &get_node_port(),
    }))
}

/// Status of the oracle pool
async fn pool_status() -> impl IntoResponse {
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
    Json(json!({
            "current_pool_stage": current_stage,
            "latest_datapoint": latest_datapoint,
            "current_epoch_id" : current_epoch_id,
            "epoch_ends": epoch_ends,
    }))
}

/// Block height of the Ergo blockchain
async fn block_height() -> impl IntoResponse {
    let current_height =
        current_block_height().expect("Please ensure that the Ergo node is running.");
    format!("{}", current_height)
}

/// Whether the Core requires the Connector to repost a new Datapoint
async fn require_datapoint_repost(repost_receiver: Receiver<bool>) -> impl IntoResponse {
    let mut response_text = "false".to_string();
    if let Ok(b) = repost_receiver.try_recv() {
        response_text = b.to_string();
    }
    response_text
}

pub async fn start_rest_server(repost_receiver: Receiver<bool>) {
    let app = Router::new()
        .route("/", get(root))
        .route("/oracleInfo", get(oracle_info))
        .route("/oracleStatus", get(oracle_status))
        .route("/poolInfo", get(pool_info))
        .route("/nodeInfo", get(node_info))
        .route("/poolStatus", get(pool_status))
        .route("/blockHeight", get(block_height))
        .route(
            "/requireDatapointRepost",
            get(|| require_datapoint_repost(repost_receiver)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([axum::http::Method::GET]),
        );
    let addr = SocketAddr::from(([0, 0, 0, 0], get_core_api_port().parse().unwrap()));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
