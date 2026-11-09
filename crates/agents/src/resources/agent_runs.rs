//! Agent run endpoints: one row per execution.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use kamino_error::{ApiResult, AppError};
use kamino_http::extract::{ApiQuery, ValidatedJson};
use kamino_query::sql::{self, ListSpec, Resource, SqlType};
use kamino_query::{ListParams, Page};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "agent_run",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_id", SqlType::Uuid),
        ("task_id", SqlType::Uuid),
        ("status", SqlType::Text),
        ("model", SqlType::Text),
        ("input_tokens", SqlType::Int),
        ("output_tokens", SqlType::Int),
        ("cost_usd", SqlType::Numeric),
        ("started_at", SqlType::Timestamptz),
        ("completed_at", SqlType::Timestamptz),
        ("error_message", SqlType::Text),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_id", "task_id", "status", "model"],
    searchable: &["model", "error_message"],
    sortable: &["started_at", "completed_at", "status", "model"],
    default_sort: "started_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AgentRun {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub status: String,
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cost_usd: Decimal,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgentRun {
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub status: Option<String>,
    pub model: Option<String>,
    #[validate(range(min = 0))]
    pub input_tokens: Option<i32>,
    #[validate(range(min = 0))]
    pub output_tokens: Option<i32>,
    pub cost_usd: Option<Decimal>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgentRun {
    pub status: Option<String>,
    pub model: Option<String>,
    #[validate(range(min = 0))]
    pub input_tokens: Option<i32>,
    #[validate(range(min = 0))]
    pub output_tokens: Option<i32>,
    pub cost_usd: Option<Decimal>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/agent-runs", get(list).post(create))
        .route(
            "/v1/agent-runs/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agent-runs",
    tag = "runs",
    params(
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "started_at | completed_at | status | model"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("agent_id" = Option<Uuid>, Query),
        ("task_id" = Option<Uuid>, Query),
        ("status" = Option<String>, Query),
        ("model" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<AgentRun>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<AgentRun>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agent-runs",
    tag = "runs",
    request_body = CreateAgentRun,
    responses((status = 201, body = AgentRun))
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateAgentRun>,
) -> ApiResult<(StatusCode, Json<AgentRun>)> {
    let payload = sql::prepare_create(&body)?;
    let created: AgentRun = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agent-runs/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Agent run id")),
    responses((status = 200, body = AgentRun), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentRun>> {
    let run = sql::get_as::<AgentRun>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent run", id))?;
    Ok(Json(run))
}

#[utoipa::path(
    patch,
    path = "/v1/agent-runs/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Agent run id")),
    request_body = UpdateAgentRun,
    responses((status = 200, body = AgentRun), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgentRun>,
) -> ApiResult<Json<AgentRun>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<AgentRun>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent run", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agent-runs/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Agent run id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent run", id))
    }
}
