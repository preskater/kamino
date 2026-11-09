//! Shared building blocks for the Kamino services.

pub mod config;
pub mod db;
pub mod error;
pub mod extract;
pub mod middleware;
pub mod pagination;
pub mod server;
pub mod sql;
pub mod telemetry;

pub use error::{ApiResult, AppError};
pub use pagination::{ListParams, Page};
pub use sql::{ListSpec, Resource, SqlType};
