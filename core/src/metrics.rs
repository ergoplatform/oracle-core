use std::convert::From;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use ergo_node_interface::scanning::NodeError;
use once_cell::sync::Lazy;
use prometheus::Encoder;
use prometheus::IntGaugeVec;
use prometheus::Opts;
use prometheus::TextEncoder;
use reqwest::StatusCode;
use tower_http::cors::CorsLayer;

use crate::monitor::check_pool_health;
use crate::monitor::PoolHealth;
use crate::monitor::PoolStatus;
use crate::node_interface::node_api::NodeApi;
use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_state::OraclePool;

static POOL_BOX_HEIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    let m = IntGaugeVec::new(
        Opts::new("pool_box_height", "The height of the pool box")
            .namespace("ergo")
            .subsystem("oracle"),
        &["pool"],
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static CURRENT_HEIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    let m = IntGaugeVec::new(
        Opts::new("current_height", "The current height")
            .namespace("ergo")
            .subsystem("oracle"),
        &["pool"],
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static EPOCH_LENGTH: Lazy<IntGaugeVec> = Lazy::new(|| {
    let m = IntGaugeVec::new(
        Opts::new("epoch_length", "The epoch length")
            .namespace("ergo")
            .subsystem("oracle"),
        &["pool"],
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static POOL_IS_HEALTHY: Lazy<IntGaugeVec> = Lazy::new(|| {
    let m = IntGaugeVec::new(
        Opts::new(
            "pool_is_healthy",
            "The health status of the pool, 1 for Ok and 0 for Down",
        )
        .namespace("ergo")
        .subsystem("oracle"),
        &["pool"],
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

pub fn update_pool_health(pool_health: &PoolHealth) {
    let pool_name = "pool";
    POOL_BOX_HEIGHT
        .with_label_values(&[pool_name])
        .set(pool_health.details.pool_box_height.into());
    CURRENT_HEIGHT
        .with_label_values(&[pool_name])
        .set(pool_health.details.current_height.into());
    EPOCH_LENGTH
        .with_label_values(&[pool_name])
        .set(pool_health.details.epoch_length.into());

    let health = match pool_health.status {
        PoolStatus::Ok => 1,
        PoolStatus::Down => 0,
    };
    POOL_IS_HEALTHY.with_label_values(&[pool_name]).set(health);
}

pub fn update_metrics(oracle_pool: Arc<OraclePool>) -> Result<(), anyhow::Error> {
    let node_api = NodeApi::new(ORACLE_CONFIG.node_api_key.clone(), &ORACLE_CONFIG.node_url);
    let current_height = (node_api.node.current_block_height()? as u32).into();
    let pool_health = check_pool_health(oracle_pool, current_height)?;
    update_pool_health(&pool_health);
    Ok(())
}

async fn serve_metrics() -> impl IntoResponse {
    let registry = prometheus::default_registry();
    let metric_families = registry.gather();
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let metrics = String::from_utf8(buffer).unwrap();
    axum::response::Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(metrics)
        .unwrap()
}

pub async fn start_metrics_server(port_num: u16) -> Result<(), anyhow::Error> {
    let app = Router::new().route("/metrics", get(serve_metrics)).layer(
        CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods([axum::http::Method::GET]),
    );
    let addr = SocketAddr::from(([0, 0, 0, 0], port_num));
    log::info!("Starting metrics server on {}", addr);
    axum::Server::try_bind(&addr)?
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

struct MetricsError(String);

impl From<NodeError> for MetricsError {
    fn from(err: NodeError) -> Self {
        MetricsError(format!("NodeError: {}", err))
    }
}

impl IntoResponse for MetricsError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}

impl From<anyhow::Error> for MetricsError {
    fn from(err: anyhow::Error) -> Self {
        MetricsError(format!("Error: {:?}", err))
    }
}
