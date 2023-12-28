use std::convert::From;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::box_kind::PoolBox;
use crate::monitor::{
    check_oracle_health, check_pool_health, HealthStatus, OracleHealth, PoolHealth,
};
use crate::node_interface::node_api::{NodeApi, NodeApiError};
use crate::oracle_config::{ORACLE_CONFIG, ORACLE_SECRETS};
use crate::oracle_state::{DataSourceError, LocalDatapointState, OraclePool};
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
        /oracleStatus - status of the oracle
        /oracleHealth - returns OK if our collected datapoint box height is the same as the pool box height OR our posted datapoint box height is greater than the pool box height
        /poolHealth - returns OK if the pool box height is greater or equal to (current height - epoch length)
        "
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
async fn oracle_status(oracle_pool: Arc<OraclePool>) -> Result<Json<serde_json::Value>, ApiError> {
    let json = task::spawn_blocking(|| oracle_status_sync(oracle_pool))
        .await
        .unwrap()?;
    Ok(json)
}

fn oracle_status_sync(oracle_pool: Arc<OraclePool>) -> Result<Json<serde_json::Value>, ApiError> {
    let live_epoch = oracle_pool.get_live_epoch_state()?;
    if let Some(local_datapoint_box_state) = live_epoch.local_datapoint_box_state {
        let json = match local_datapoint_box_state {
            LocalDatapointState::Collected { height } => json!( {
                "status": "collected",
                "height": height,
            }),
            LocalDatapointState::Posted { epoch_id, height } => json!( {
                "status": "posted",
                "epoch_id": epoch_id,
                "height": height,
            }),
        };
        let oracle_health = oracle_health_sync(oracle_pool)?;
        Ok(Json(json!({
                "local_datapoint_box_state": json,
                "oracle_health": oracle_health,
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
async fn pool_status(oracle_pool: Arc<OraclePool>) -> Result<Json<serde_json::Value>, ApiError> {
    let json = task::spawn_blocking(|| pool_status_sync(oracle_pool))
        .await
        .unwrap()?;
    Ok(json)
}

fn pool_status_sync(oracle_pool: Arc<OraclePool>) -> Result<Json<serde_json::Value>, ApiError> {
    let node_api = NodeApi::new(
        ORACLE_SECRETS.node_api_key.clone(),
        ORACLE_SECRETS.wallet_password.clone(),
        &ORACLE_CONFIG.node_url,
    );
    let current_height = node_api.node.current_block_height()? as u32;
    let pool_box = oracle_pool.get_pool_box_source().get_pool_box()?;
    let epoch_length = POOL_CONFIG
        .refresh_box_wrapper_inputs
        .contract_inputs
        .contract_parameters()
        .epoch_length();
    let pool_box_height = pool_box.get_box().creation_height;
    let epoch_end_height = pool_box_height + epoch_length.0 as u32;
    let pool_health = pool_health_sync(oracle_pool)?;
    let active_oracle_count = pool_health.details.active_oracle_boxes.len();
    let json = Json(json!({
        "latest_pool_datapoint": pool_box.rate(),
        "latest_pool_box_height": pool_box_height,
        "pool_box_epoch_id" : pool_box.epoch_counter(),
        "current_block_height": current_height,
        "epoch_end_height": epoch_end_height,
        "reward_tokens_in_pool_box": pool_box.reward_token().amount.as_u64(),
        "number_of_oracles": active_oracle_count,
        "pool_health": pool_health,
    }));
    Ok(json)
}

/// Block height of the Ergo blockchain
async fn block_height() -> Result<impl IntoResponse, ApiError> {
    let current_height = task::spawn_blocking(move || {
        let node_api = NodeApi::new(
            ORACLE_SECRETS.node_api_key.clone(),
            ORACLE_SECRETS.wallet_password.clone(),
            &ORACLE_CONFIG.node_url,
        );
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

/// Return true if the our collected datapoint box height is the same as the pool box height
/// and our posted datapoint box height is greater than the pool box height
async fn oracle_health(oracle_pool: Arc<OraclePool>) -> impl IntoResponse {
    let pool_health = match task::spawn_blocking(|| oracle_health_sync(oracle_pool))
        .await
        .unwrap()
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e.0))),
    };
    // return 503 http error if pool_health.status is not ok
    if pool_health.status != HealthStatus::Ok {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::to_value(pool_health).unwrap()),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::to_value(pool_health).unwrap()),
    )
}

fn oracle_health_sync(oracle_pool: Arc<OraclePool>) -> Result<OracleHealth, ApiError> {
    let node_api = NodeApi::new(
        ORACLE_SECRETS.node_api_key.clone(),
        ORACLE_SECRETS.wallet_password.clone(),
        &ORACLE_CONFIG.node_url,
    );
    let current_height = (node_api.node.current_block_height()? as u32).into();
    let epoch_length = POOL_CONFIG
        .refresh_box_wrapper_inputs
        .contract_inputs
        .contract_parameters()
        .epoch_length()
        .0
        .into();
    let pool_box_height = oracle_pool
        .get_pool_box_source()
        .get_pool_box()?
        .get_box()
        .creation_height
        .into();
    let oracle_health =
        check_oracle_health(oracle_pool, pool_box_height, current_height, epoch_length)?;
    Ok(oracle_health)
}

async fn pool_health(oracle_pool: Arc<OraclePool>) -> impl IntoResponse {
    let pool_health = match task::spawn_blocking(|| pool_health_sync(oracle_pool))
        .await
        .unwrap()
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e.0))),
    };
    // return 503 http error if pool_health.status is not ok
    if pool_health.status != HealthStatus::Ok {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::to_value(pool_health).unwrap()),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::to_value(pool_health).unwrap()),
    )
}

fn pool_health_sync(oracle_pool: Arc<OraclePool>) -> Result<PoolHealth, ApiError> {
    let node_api = NodeApi::new(
        ORACLE_SECRETS.node_api_key.clone(),
        ORACLE_SECRETS.wallet_password.clone(),
        &ORACLE_CONFIG.node_url,
    );
    let current_height = (node_api.node.current_block_height()? as u32).into();
    let pool_box = &oracle_pool.get_pool_box_source().get_pool_box()?;
    let pool_box_height = pool_box.get_box().creation_height.into();
    let network_prefix = node_api.get_change_address()?.network();
    let pool_health = check_pool_health(
        current_height,
        pool_box_height,
        pool_box.rate(),
        oracle_pool,
        network_prefix,
    )?;
    Ok(pool_health)
}

pub async fn start_rest_server(
    repost_receiver: Receiver<bool>,
    oracle_pool: Arc<OraclePool>,
    api_port: u16,
) -> Result<(), anyhow::Error> {
    let op_clone = oracle_pool.clone();
    let op_clone2 = oracle_pool.clone();
    let op_clone3 = oracle_pool.clone();
    let app = Router::new()
        .route("/", get(root))
        .route("/oracleInfo", get(oracle_info))
        .route("/oracleStatus", get(|| oracle_status(oracle_pool)))
        .route("/poolInfo", get(pool_info))
        .route("/poolStatus", get(|| pool_status(op_clone)))
        .route("/blockHeight", get(block_height))
        .route("/oracleHealth", get(|| oracle_health(op_clone2)))
        .route("/poolHealth", get(|| pool_health(op_clone3)))
        .route(
            "/requireDatapointRepost",
            get(|| require_datapoint_repost(repost_receiver)),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([axum::http::Method::GET]),
        );
    let addr = SocketAddr::from(([0, 0, 0, 0], api_port));
    log::info!("Starting REST server on {}", addr);
    axum::Server::try_bind(&addr)?
        .serve(app.into_make_service())
        .await?;
    Ok(())
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

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError(format!("Error: {:?}", err))
    }
}

impl From<NodeApiError> for ApiError {
    fn from(err: NodeApiError) -> Self {
        ApiError(format!("NodeApiError: {:?}", err))
    }
}
