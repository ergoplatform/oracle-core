use std::convert::From;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use ergo_node_interface::scanning::NodeError;
use prometheus::Encoder;
use prometheus::Registry;
use prometheus::TextEncoder;
use reqwest::StatusCode;
use tower_http::cors::CorsLayer;

async fn serve_metrics(registry: Arc<Registry>) -> impl IntoResponse {
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

pub async fn start_metrics_server(reg: Arc<Registry>, port_num: u16) -> Result<(), anyhow::Error> {
    let app = Router::new()
        .route("/metrics", get(move || serve_metrics(reg.clone())))
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([axum::http::Method::GET]),
        );
    let addr = SocketAddr::from(([0, 0, 0, 0], port_num));
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
