//! Binary entry point.

use anyhow::Context;
use kamino_agents::{AppState, config::Config, run};
use kamino_config as config;
use kamino_db as db;
use kamino_telemetry as telemetry;
use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    config::load_dotenv();
    telemetry::init("kamino_agents");

    let config = Config::from_env().context("invalid configuration")?;
    let pool = db::connect(&config.database_url, config.db_max_connections).await?;
    db::run_migrations(&pool, &MIGRATOR).await?;

    let state = AppState::new(pool, config);
    run(state).await
}
