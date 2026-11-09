//! Binary entry point.

use anyhow::Context;
use kamino_common::{config, db, telemetry};
use kamino_projects::{AppState, config::Config, run};
use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    config::load_dotenv();
    telemetry::init("kamino_projects");

    let config = Config::from_env().context("invalid configuration")?;
    let pool = db::connect(&config.database_url, config.db_max_connections).await?;
    db::run_migrations(&pool, &MIGRATOR).await?;

    let state = AppState::new(pool, config);
    run(state).await
}
