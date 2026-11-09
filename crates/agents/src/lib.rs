//! Kamino agents service.

pub mod config;
pub mod openapi;
pub mod resources;
pub mod state;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use kamino_db as db;
use kamino_http::middleware;
use kamino_http::server;
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

pub use state::AppState;

pub fn app(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/health/ready", get(ready))
        .merge(resources::router())
        .with_state(state);

    let app = api
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()));

    middleware::standard_layers(app)
}

pub async fn run(state: AppState) -> anyhow::Result<()> {
    let addr = state.config.bind_addr;
    server::serve(addr, app(state)).await
}

#[derive(Debug, Serialize, ToSchema)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    if db::is_ready(&state.pool).await {
        (StatusCode::OK, Json(Health { status: "ready" }))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(Health {
                status: "unavailable",
            }),
        )
    }
}
