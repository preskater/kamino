//! Server bootstrap: listener binding plus graceful shutdown.

use std::net::SocketAddr;

use axum::Router;

/// Serves `app` until a shutdown signal arrives.
pub async fn serve(addr: SocketAddr, app: Router) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|err| anyhow::anyhow!("failed to bind {addr}: {err}"))?;

    tracing::info!(%addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(super::middleware::shutdown_signal())
        .await
        .map_err(|err| anyhow::anyhow!("server error: {err}"))
}
