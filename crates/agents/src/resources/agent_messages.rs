//! Conversation messages belonging to a run.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use kamino_error::{ApiResult, AppError};
use kamino_http::extract::{ApiQuery, ValidatedJson};
use kamino_query::sql::{self, ListSpec, Resource, SqlType};
use kamino_query::{ListParams, Page};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "agent_message",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_run_id", SqlType::Uuid),
        ("role", SqlType::Text),
        ("content", SqlType::Text),
        ("tool_calls", SqlType::Jsonb),
        ("tool_results", SqlType::Jsonb),
        ("token_count", SqlType::Int),
        ("created_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_run_id", "role"],
    searchable: &["content"],
    sortable: &["created_at", "role"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AgentMessage {
    pub id: Uuid,
    pub agent_run_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_calls: Value,
    pub tool_results: Value,
    pub token_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgentMessage {
    /// One of `system`, `user`, `assistant`, `tool`.
    #[validate(length(min = 1, max = 32))]
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Value>,
    pub tool_results: Option<Value>,
    #[validate(range(min = 0))]
    pub token_count: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgentMessage {
    pub content: Option<String>,
    pub tool_calls: Option<Value>,
    pub tool_results: Option<Value>,
    #[validate(range(min = 0))]
    pub token_count: Option<i32>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/agent-runs/{run_id}/messages",
            get(list_for_run).post(create_for_run),
        )
        .route(
            "/v1/agent-messages/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agent-runs/{run_id}/messages",
    tag = "runs",
    params(
        ("run_id" = Uuid, Path, description = "Agent run id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("role" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<AgentMessage>))
)]
pub async fn list_for_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<AgentMessage>>> {
    params.with_filter("agent_run_id", run_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agent-runs/{run_id}/messages",
    tag = "runs",
    params(("run_id" = Uuid, Path, description = "Agent run id")),
    request_body = CreateAgentMessage,
    responses((status = 201, body = AgentMessage))
)]
pub async fn create_for_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateAgentMessage>,
) -> ApiResult<(StatusCode, Json<AgentMessage>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("agent_run_id".to_string(), json!(run_id.to_string()));
    let created: AgentMessage = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agent-messages/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Message id")),
    responses((status = 200, body = AgentMessage), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentMessage>> {
    let message = sql::get_as::<AgentMessage>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent message", id))?;
    Ok(Json(message))
}

#[utoipa::path(
    patch,
    path = "/v1/agent-messages/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Message id")),
    request_body = UpdateAgentMessage,
    responses((status = 200, body = AgentMessage), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgentMessage>,
) -> ApiResult<Json<AgentMessage>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<AgentMessage>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent message", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agent-messages/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Message id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent message", id))
    }
}
