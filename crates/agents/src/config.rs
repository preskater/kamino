//! Service configuration.

use std::net::SocketAddr;

use kamino_common::config;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub db_max_connections: u32,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind_addr: config::parse_or(
                "AGENTS_BIND_ADDR",
                SocketAddr::from(([0, 0, 0, 0], 8081)),
            )?,
            database_url: config::required("AGENTS_DATABASE_URL")?,
            db_max_connections: config::parse_or("AGENTS_DB_MAX_CONNECTIONS", 10_u32)?,
        })
    }
}
