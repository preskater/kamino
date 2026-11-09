//! Postgres pool creation, migrations and readiness checks.

use std::time::Duration;

use anyhow::Context;
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Builds a pooled connection to Postgres.
pub async fn connect(url: &str, max_connections: u32) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await
        .context("failed to connect to postgres")?;
    Ok(pool)
}

/// Applies every pending migration embedded in the binary.
pub async fn run_migrations(pool: &PgPool, migrator: &Migrator) -> anyhow::Result<()> {
    migrator
        .run(pool)
        .await
        .context("failed to run database migrations")?;
    Ok(())
}

/// Returns true when the database answers a trivial query.
pub async fn is_ready(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}
