use std::convert::From;
use std::net::SocketAddr;

use crate::node_interface::current_block_height;
use crate::oracle_config::{get_core_api_port, get_node_ip, get_node_port, ORACLE_CONFIG};
use crate::oracle_state::LocalDatapointState::{Collected, Posted};
use crate::oracle_state::{OraclePool, StageError};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use crossbeam::channel::Receiver;
use ergo_node_interface::scanning::NodeError;
use serde_json::json;
use tokio::task;
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
async fn oracle_status() -> Result<Json<serde_json::Value>, ApiError> {
    let op = OraclePool::new().unwrap();
    let live_epoch = task::spawn_blocking(move || op.get_live_epoch_state())
        .await
        .unwrap()?;
    if let Some(local_datapoint_box_state) = live_epoch.local_datapoint_box_state {
        let json = match local_datapoint_box_state {
            Collected { height } => json!( {
                "status": "collected",
                "height": height,
            }),
            Posted { epoch_id, height } => json!( {
                "status": "posted",
                "epoch_id": epoch_id,
                "height": height,
            }),
        };
        Ok(Json(json!({
                "local_datapoint_box_state": json,
        })))
    } else {
        Ok(Json(json!({
                "local_datapoint_box_state": "No local datapoint box",
        })))
    }
}

// Basic information about the oracle pool
async fn pool_info() -> impl IntoResponse {
    let conf = &ORACLE_CONFIG;
    Json(json!({
        "pool_nft_id": conf.token_ids.pool_nft_token_id,
        "oracle_token_id": conf.token_ids.oracle_token_id,
        "reward_token_id": conf.token_ids.reward_token_id,
        "refresh_token_id": conf.token_ids.refresh_nft_token_id,
        "ballot_token_id": conf.token_ids.ballot_token_id,
        "update_token_id": conf.token_ids.update_nft_token_id,
        "epoch_length": conf.refresh_box_wrapper_inputs.contract_inputs.contract_parameters().epoch_length(),
        "max_deviation_percent": conf.refresh_box_wrapper_inputs.contract_inputs.contract_parameters().max_deviation_percent(),
        "min_data_points": conf.refresh_box_wrapper_inputs.contract_inputs.contract_parameters().min_data_points(),
        "min_votes": conf.update_box_wrapper_inputs.contract_inputs.contract_parameters().min_votes(),
    }))
}

/// Basic information about node the oracle core is using
async fn node_info() -> impl IntoResponse {
    Json(json!({
        "node_url": "http://".to_string() + &get_node_ip() + ":" + &get_node_port(),
    }))
}

/// Status of the oracle pool
async fn pool_status() -> Result<Json<serde_json::Value>, ApiError> {
    let op = OraclePool::new().unwrap();
    let live_epoch = task::spawn_blocking(move || op.get_live_epoch_state())
        .await
        .unwrap()?;
    Ok(Json(json!({
            "latest_pool_datapoint": live_epoch.latest_pool_datapoint,
            "latest_pool_box_height": live_epoch.latest_pool_box_height,
            "pool_box_epoch_id" : live_epoch.pool_box_epoch_id,
    })))
}

/// Block height of the Ergo blockchain
async fn block_height() -> Result<impl IntoResponse, ApiError> {
    let current_height = task::spawn_blocking(current_block_height).await.unwrap()?;
    Ok(format!("{}", current_height))
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

struct ApiError(String);

impl From<StageError> for ApiError {
    fn from(err: StageError) -> Self {
        ApiError(format!("StageError: {}", err))
    }
}

impl From<NodeError> for ApiError {
    fn from(err: NodeError) -> Self {
        ApiError(format!("NodeError: {}", err))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}
