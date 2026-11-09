//! Cross cutting HTTP middleware and process lifecycle.

use axum::Router;
use axum::http::HeaderName;
use axum::http::header::HeaderValue;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

const REQUEST_ID_HEADER: &str = "x-request-id";

/// Applies request ids, tracing and CORS to a router.
pub fn standard_layers<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let header = HeaderName::from_static(REQUEST_ID_HEADER);
    router
        .layer(PropagateRequestIdLayer::new(header.clone()))
        .layer(SetRequestIdLayer::new(header, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
}

fn cors_layer() -> CorsLayer {
    let configured = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| "*".to_string());
    if configured.trim() == "*" {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    let origins = configured
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Resolves on SIGINT or SIGTERM so servers can drain connections.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutdown signal received, draining connections");
}
