use std::convert::From;
use std::net::SocketAddr;

use crate::box_kind::PoolBox;
use crate::node_interface::node_api::NodeApi;
use crate::oracle_config::{get_core_api_port, ORACLE_CONFIG};
use crate::oracle_state::LocalDatapointState::{Collected, Posted};
use crate::oracle_state::{DataSourceError, OraclePool};
use crate::pool_config::POOL_CONFIG;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use crossbeam::channel::Receiver;
use ergo_lib::ergotree_ir::chain::address::{Address, AddressEncoder};
use ergo_node_interface::scanning::NodeError;
use serde_json::json;
use tokio::task;
use tower_http::cors::CorsLayer;

/// Basic welcome endpoint
async fn root() -> &'static str {
    "This is an Oracle Core. Please use one of the endpoints to interact with it: 
        /poolInfo - basic information about the oracle pool
        /poolStatus - status of the oracle pool
        /oracleInfo - basic information about the oracle
        /oracleStatus - status of the oracle"
}

/// Basic oracle information
async fn oracle_info() -> impl IntoResponse {
    let conf = &ORACLE_CONFIG;
    Json(json! ( {
        "oracle_address": conf.oracle_address.to_base58(),
        "base_fee": conf.base_fee,
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
    let conf = &POOL_CONFIG;
    let network = &ORACLE_CONFIG.oracle_address.network();
    let address_encoder = AddressEncoder::new(*network);
    let pool_box_address = Address::P2S(
        conf.pool_box_wrapper_inputs
            .contract_inputs
            .contract_parameters()
            .ergo_tree_bytes()
            .clone(),
    );
    let refresh_box_address = Address::P2S(
        conf.refresh_box_wrapper_inputs
            .contract_inputs
            .contract_parameters()
            .ergo_tree_bytes()
            .clone(),
    );
    let update_box_address = Address::P2S(
        conf.update_box_wrapper_inputs
            .contract_inputs
            .contract_parameters()
            .ergo_tree_bytes()
            .clone(),
    );
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
        "pool_box_address": address_encoder.address_to_str(&pool_box_address),
        "refresh_box_address": address_encoder.address_to_str(&refresh_box_address),
        "update_box_address": address_encoder.address_to_str(&update_box_address),
    }))
}

/// Status of the oracle pool
async fn pool_status() -> Result<Json<serde_json::Value>, ApiError> {
    let json = task::spawn_blocking(pool_status_sync).await.unwrap()?;
    Ok(json)
}

fn pool_status_sync() -> Result<Json<serde_json::Value>, ApiError> {
    let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
    let current_height = node_api.node.current_block_height()? as u32;
    let op = OraclePool::new().unwrap();
    let pool_box = op.get_pool_box_source().get_pool_box()?;
    let epoch_length = POOL_CONFIG
        .refresh_box_wrapper_inputs
        .contract_inputs
        .contract_parameters()
        .epoch_length();
    let latest_pool_box_height = pool_box.get_box().creation_height;
    let epoch_end_height = latest_pool_box_height + epoch_length.0 as u32;

    let oracle_boxes = op
        .get_datapoint_boxes_source()
        .get_oracle_datapoint_boxes()?;
    let min_oracle_box_height = current_height - epoch_length.0 as u32;
    let active_oracle_count = oracle_boxes
        .into_iter()
        .filter(|b| b.get_box().creation_height >= min_oracle_box_height)
        .count() as u32;

    let json = Json(json!({
        "latest_pool_datapoint": pool_box.rate(),
        "latest_pool_box_height": latest_pool_box_height,
        "pool_box_epoch_id" : pool_box.epoch_counter(),
        "current_block_height": current_height,
        "epoch_end_height": epoch_end_height,
        "reward_tokens_in_pool_box": pool_box.reward_token().amount.as_u64(),
        "number_of_oracles": active_oracle_count,
    }));
    Ok(json)
}

/// Block height of the Ergo blockchain
async fn block_height() -> Result<impl IntoResponse, ApiError> {
    let current_height = task::spawn_blocking(move || {
        let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
        node_api.node.current_block_height()
    })
    .await
    .unwrap()?;
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

impl From<DataSourceError> for ApiError {
    fn from(err: DataSourceError) -> Self {
        ApiError(format!("DataSourceError: {}", err))
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
