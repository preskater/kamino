//! Resource routers.

pub mod activity_logs;
pub mod artifacts;
pub mod projects;
pub mod task_comments;
pub mod task_dependencies;
pub mod task_reviews;
pub mod tasks;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(projects::router())
        .merge(tasks::router())
        .merge(task_dependencies::router())
        .merge(task_comments::router())
        .merge(task_reviews::router())
        .merge(artifacts::router())
        .merge(activity_logs::router())
}
