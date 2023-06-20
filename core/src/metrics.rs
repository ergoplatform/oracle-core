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
use prometheus::IntGauge;
use prometheus::IntGaugeVec;
use prometheus::Opts;
use prometheus::TextEncoder;
use reqwest::StatusCode;
use tower_http::cors::CorsLayer;

use crate::box_kind::PoolBox;
use crate::monitor::check_oracle_health;
use crate::monitor::check_pool_health;
use crate::monitor::OracleHealth;
use crate::monitor::PoolHealth;
use crate::node_interface::node_api::NodeApi;
use crate::oracle_config::ORACLE_CONFIG;
use crate::oracle_config::ORACLE_SECRETS;
use crate::oracle_state::OraclePool;

static POOL_BOX_HEIGHT: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new("pool_box_height", "The height of the pool box")
            .namespace("ergo")
            .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static POOL_BOX_RATE: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new("pool_box_rate", "exchange rate from the pool box")
            .namespace("ergo")
            .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static POOL_BOX_REWARD_TOKEN_AMOUNT: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new(
            "pool_box_reward_token_amount",
            "The amount of reward token in the pool box",
        )
        .namespace("ergo")
        .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static CURRENT_HEIGHT: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new("current_height", "The current height")
            .namespace("ergo")
            .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static EPOCH_LENGTH: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new("epoch_length", "The epoch length")
            .namespace("ergo")
            .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static POOL_IS_HEALTHY: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new(
            "pool_is_healthy",
            "The health status of the pool, 1 for Ok and 0 for Down",
        )
        .namespace("ergo")
        .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static ORACLE_IS_HEALTHY: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new(
            "oracle_is_healthy",
            "The health status of the oracle, 1 for Ok and 0 for Down",
        )
        .namespace("ergo")
        .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static MY_ORACLE_BOX_HEIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    let m = IntGaugeVec::new(
        Opts::new(
            "oracle_box_height",
            "The height of the posted/collected oracle box for this oracle",
        )
        .namespace("ergo")
        .subsystem("oracle"),
        &["box_type"],
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static ALL_ORACLE_BOX_HEIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    let m = IntGaugeVec::new(
        Opts::new(
            "all_oracle_box_height",
            "The height of the posted/collected oracle box for all oracles",
        )
        .namespace("ergo")
        .subsystem("oracle"),
        &["box_type", "oracle_address"],
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static ACTIVE_ORACLE_BOX_HEIGHT: Lazy<IntGaugeVec> = Lazy::new(|| {
    let m = IntGaugeVec::new(
        Opts::new(
            "active_oracle_box_height",
            "The height of the posted/collected oracle boxes of active oracles",
        )
        .namespace("ergo")
        .subsystem("oracle"),
        &["box_type", "oracle_address"],
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static ACTIVE_ORACLE_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new("active_oracle_count", "The number of active oracles")
            .namespace("ergo")
            .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static TOTAL_ORACLE_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new("total_oracle_count", "The total number of oracle tokens")
            .namespace("ergo")
            .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static REQUIRED_ORACLE_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new(
            "required_oracle_count",
            "The minimum number of active oracles",
        )
        .namespace("ergo")
        .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static ORACLE_NODE_WALLET_BALANCE: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new(
            "oracle_node_wallet_nano_erg",
            "Coins in the oracle's node wallet",
        )
        .namespace("ergo")
        .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

static REWARD_TOKENS_IN_BUYBACK_BOX: Lazy<IntGauge> = Lazy::new(|| {
    let m = IntGauge::with_opts(
        Opts::new(
            "reward_tokens_in_buyback_box",
            "The amount of reward tokens in the buyback box",
        )
        .namespace("ergo")
        .subsystem("oracle"),
    )
    .unwrap();
    prometheus::register(Box::new(m.clone())).expect("Failed to register");
    m
});

fn update_pool_health(pool_health: &PoolHealth) {
    POOL_BOX_HEIGHT.set(pool_health.details.pool_box_height.into());
    CURRENT_HEIGHT.set(pool_health.details.current_height.into());
    EPOCH_LENGTH.set(pool_health.details.epoch_length.into());
    POOL_IS_HEALTHY.set(pool_health.status as i64);
    for oracle in &pool_health.details.all_oracle_boxes {
        let box_type = oracle.box_height.label_name();
        let box_height = oracle.box_height.oracle_box_height().into();
        ALL_ORACLE_BOX_HEIGHT
            .with_label_values(&[box_type, &oracle.address.to_base58()])
            .set(box_height);
    }
    for oracle in &pool_health.details.active_oracle_boxes {
        let box_type = oracle.box_height.label_name();
        let box_height = oracle.box_height.oracle_box_height().into();
        ACTIVE_ORACLE_BOX_HEIGHT
            .with_label_values(&[box_type, &oracle.address.to_base58()])
            .set(box_height);
    }
    ACTIVE_ORACLE_COUNT.set(pool_health.details.active_oracle_boxes.len() as i64);
    REQUIRED_ORACLE_COUNT.set(pool_health.details.min_data_points.into());
    TOTAL_ORACLE_COUNT.set(pool_health.details.total_oracle_token_count as i64);
}

fn update_oracle_health(oracle_health: &OracleHealth) {
    let box_type = oracle_health.details.box_details.label_name();
    MY_ORACLE_BOX_HEIGHT
        .with_label_values(&[box_type])
        .set(oracle_health.details.box_details.oracle_box_height().into());
    ORACLE_IS_HEALTHY.set(oracle_health.status as i64);
}

fn update_reward_tokens_in_buyback_box(oracle_pool: Arc<OraclePool>) {
    if let Some(buyback_box) = oracle_pool
        .get_buyback_box_source()
        .map(|s| s.get_buyback_box())
        .transpose()
        .ok()
        .flatten()
        .flatten()
    {
        let reward_token_amount: i64 = buyback_box
            .reward_token()
            .map(|t| t.amount.into())
            .unwrap_or(0);
        REWARD_TOKENS_IN_BUYBACK_BOX.set(reward_token_amount);
    }
}

pub fn update_metrics(oracle_pool: Arc<OraclePool>) -> Result<(), anyhow::Error> {
    let node_api = NodeApi::new(ORACLE_SECRETS.node_api_key.clone(), &ORACLE_CONFIG.node_url);
    let current_height = (node_api.node.current_block_height()? as u32).into();
    let network_prefix = node_api.get_change_address()?.network();
    let pool_box = &oracle_pool.get_pool_box_source().get_pool_box()?;
    {
        let rate = pool_box.rate();
        POOL_BOX_RATE.set(rate.into());
    };
    let pool_box_height = pool_box.get_box().creation_height.into();
    let pool_health = check_pool_health(
        current_height,
        pool_box_height,
        oracle_pool.clone(),
        network_prefix,
    )?;
    update_pool_health(&pool_health);
    let oracle_health = check_oracle_health(oracle_pool.clone(), pool_box_height)?;
    update_oracle_health(&oracle_health);
    let wallet_balance: i64 = node_api.node.wallet_nano_ergs_balance()? as i64;
    ORACLE_NODE_WALLET_BALANCE.set(wallet_balance);
    POOL_BOX_REWARD_TOKEN_AMOUNT.set(pool_box.reward_token().amount.into());
    update_reward_tokens_in_buyback_box(oracle_pool);
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
