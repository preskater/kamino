//! Artifacts produced by a run (and optionally attached to a task).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use kamino_common::error::{ApiResult, AppError};
use kamino_common::extract::{ApiQuery, ValidatedJson};
use kamino_common::pagination::{ListParams, Page};
use kamino_common::sql::{self, ListSpec, Resource, SqlType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "agent_artifact",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_run_id", SqlType::Uuid),
        ("task_id", SqlType::Uuid),
        ("artifact_type", SqlType::Text),
        ("name", SqlType::Text),
        ("storage_url", SqlType::Text),
        ("content", SqlType::Text),
        ("metadata", SqlType::Jsonb),
        ("created_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_run_id", "task_id", "artifact_type"],
    searchable: &["name", "content"],
    sortable: &["created_at", "name", "artifact_type"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AgentArtifact {
    pub id: Uuid,
    pub agent_run_id: Uuid,
    pub task_id: Option<Uuid>,
    pub artifact_type: String,
    pub name: String,
    pub storage_url: String,
    pub content: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgentArtifact {
    pub agent_run_id: Uuid,
    pub task_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub storage_url: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgentArtifact {
    pub task_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub storage_url: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/agent-artifacts", get(list).post(create))
        .route(
            "/v1/agent-artifacts/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agent-artifacts",
    tag = "runs",
    params(
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "created_at | name | artifact_type"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("agent_run_id" = Option<Uuid>, Query),
        ("task_id" = Option<Uuid>, Query),
        ("artifact_type" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<AgentArtifact>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<AgentArtifact>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agent-artifacts",
    tag = "runs",
    request_body = CreateAgentArtifact,
    responses((status = 201, body = AgentArtifact))
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateAgentArtifact>,
) -> ApiResult<(StatusCode, Json<AgentArtifact>)> {
    let payload = sql::prepare_create(&body)?;
    let created: AgentArtifact = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agent-artifacts/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Artifact id")),
    responses((status = 200, body = AgentArtifact), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentArtifact>> {
    let artifact = sql::get_as::<AgentArtifact>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent artifact", id))?;
    Ok(Json(artifact))
}

#[utoipa::path(
    patch,
    path = "/v1/agent-artifacts/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Artifact id")),
    request_body = UpdateAgentArtifact,
    responses((status = 200, body = AgentArtifact), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgentArtifact>,
) -> ApiResult<Json<AgentArtifact>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<AgentArtifact>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent artifact", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agent-artifacts/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Artifact id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent artifact", id))
    }
}
