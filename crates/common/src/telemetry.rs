//! Logging setup.

use tracing_subscriber::EnvFilter;

/// Installs a `tracing` subscriber, honouring `RUST_LOG`.
pub fn init(service: &'static str) {
    let default_filter = format!("info,{service}=debug,tower_http=debug");
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}
