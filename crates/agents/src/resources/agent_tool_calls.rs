//! Tool invocations recorded during a run.

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
    table: "agent_tool_call",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_run_id", SqlType::Uuid),
        ("tool_name", SqlType::Text),
        ("arguments", SqlType::Jsonb),
        ("result", SqlType::Jsonb),
        ("status", SqlType::Text),
        ("called_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_run_id", "tool_name", "status"],
    searchable: &["tool_name"],
    sortable: &["called_at", "tool_name", "status"],
    default_sort: "called_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AgentToolCall {
    pub id: Uuid,
    pub agent_run_id: Uuid,
    pub tool_name: String,
    pub arguments: Value,
    pub result: Option<Value>,
    pub status: String,
    pub called_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgentToolCall {
    #[validate(length(min = 1, max = 160))]
    pub tool_name: String,
    pub arguments: Option<Value>,
    pub result: Option<Value>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgentToolCall {
    #[validate(length(min = 1, max = 160))]
    pub tool_name: Option<String>,
    pub arguments: Option<Value>,
    pub result: Option<Value>,
    pub status: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/agent-runs/{run_id}/tool-calls",
            get(list_for_run).post(create_for_run),
        )
        .route(
            "/v1/agent-tool-calls/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agent-runs/{run_id}/tool-calls",
    tag = "runs",
    params(
        ("run_id" = Uuid, Path, description = "Agent run id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("tool_name" = Option<String>, Query),
        ("status" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<AgentToolCall>))
)]
pub async fn list_for_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<AgentToolCall>>> {
    params.with_filter("agent_run_id", run_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agent-runs/{run_id}/tool-calls",
    tag = "runs",
    params(("run_id" = Uuid, Path, description = "Agent run id")),
    request_body = CreateAgentToolCall,
    responses((status = 201, body = AgentToolCall))
)]
pub async fn create_for_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateAgentToolCall>,
) -> ApiResult<(StatusCode, Json<AgentToolCall>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("agent_run_id".to_string(), json!(run_id.to_string()));
    let created: AgentToolCall = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agent-tool-calls/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Tool call id")),
    responses((status = 200, body = AgentToolCall), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentToolCall>> {
    let call = sql::get_as::<AgentToolCall>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent tool call", id))?;
    Ok(Json(call))
}

#[utoipa::path(
    patch,
    path = "/v1/agent-tool-calls/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Tool call id")),
    request_body = UpdateAgentToolCall,
    responses((status = 200, body = AgentToolCall), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgentToolCall>,
) -> ApiResult<Json<AgentToolCall>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<AgentToolCall>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent tool call", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agent-tool-calls/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Tool call id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent tool call", id))
    }
}
