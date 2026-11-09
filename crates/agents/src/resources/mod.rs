//! Resource routers.

pub mod agent_artifacts;
pub mod agent_messages;
pub mod agent_metrics;
pub mod agent_runs;
pub mod agent_skills;
pub mod agent_templates;
pub mod agent_tool_calls;
pub mod agents;
pub mod skill_files;
pub mod skill_versions;
pub mod skills;
pub mod task_skill_usage;

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(agent_templates::router())
        .merge(agents::router())
        .merge(skills::router())
        .merge(skill_versions::router())
        .merge(skill_files::router())
        .merge(agent_skills::router())
        .merge(agent_runs::router())
        .merge(task_skill_usage::router())
        .merge(agent_messages::router())
        .merge(agent_tool_calls::router())
        .merge(agent_artifacts::router())
        .merge(agent_metrics::router())
}
