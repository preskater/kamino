//! Daily usage metrics per agent.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::NaiveDate;
use kamino_error::{ApiResult, AppError};
use kamino_http::extract::{ApiQuery, ValidatedJson};
use kamino_query::sql::{self, ListSpec, Resource, SqlType};
use kamino_query::{ListParams, Page};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "agent_metric",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_id", SqlType::Uuid),
        ("metric_date", SqlType::Date),
        ("runs_count", SqlType::Int),
        ("successful_runs", SqlType::Int),
        ("failed_runs", SqlType::Int),
        ("total_input_tokens", SqlType::Int),
        ("total_output_tokens", SqlType::Int),
        ("total_cost_usd", SqlType::Numeric),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_id", "metric_date"],
    searchable: &[],
    sortable: &["metric_date", "runs_count", "total_cost_usd"],
    default_sort: "metric_date",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AgentMetric {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub metric_date: NaiveDate,
    pub runs_count: i32,
    pub successful_runs: i32,
    pub failed_runs: i32,
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
    pub total_cost_usd: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgentMetric {
    pub metric_date: NaiveDate,
    #[validate(range(min = 0))]
    pub runs_count: Option<i32>,
    #[validate(range(min = 0))]
    pub successful_runs: Option<i32>,
    #[validate(range(min = 0))]
    pub failed_runs: Option<i32>,
    #[validate(range(min = 0))]
    pub total_input_tokens: Option<i32>,
    #[validate(range(min = 0))]
    pub total_output_tokens: Option<i32>,
    pub total_cost_usd: Option<Decimal>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgentMetric {
    pub metric_date: Option<NaiveDate>,
    #[validate(range(min = 0))]
    pub runs_count: Option<i32>,
    #[validate(range(min = 0))]
    pub successful_runs: Option<i32>,
    #[validate(range(min = 0))]
    pub failed_runs: Option<i32>,
    #[validate(range(min = 0))]
    pub total_input_tokens: Option<i32>,
    #[validate(range(min = 0))]
    pub total_output_tokens: Option<i32>,
    pub total_cost_usd: Option<Decimal>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/agents/{agent_id}/metrics",
            get(list_for_agent).post(create_for_agent),
        )
        .route(
            "/v1/agent-metrics/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agents/{agent_id}/metrics",
    tag = "metrics",
    params(
        ("agent_id" = Uuid, Path, description = "Agent id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "metric_date | runs_count | total_cost_usd"),
        ("order" = Option<String>, Query),
        ("metric_date" = Option<NaiveDate>, Query)
    ),
    responses((status = 200, body = Page<AgentMetric>))
)]
pub async fn list_for_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<AgentMetric>>> {
    params.with_filter("agent_id", agent_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agents/{agent_id}/metrics",
    tag = "metrics",
    params(("agent_id" = Uuid, Path, description = "Agent id")),
    request_body = CreateAgentMetric,
    responses((status = 201, body = AgentMetric), (status = 409, description = "Metric for that day already exists"))
)]
pub async fn create_for_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateAgentMetric>,
) -> ApiResult<(StatusCode, Json<AgentMetric>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("agent_id".to_string(), json!(agent_id.to_string()));
    let created: AgentMetric = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agent-metrics/{id}",
    tag = "metrics",
    params(("id" = Uuid, Path, description = "Metric id")),
    responses((status = 200, body = AgentMetric), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentMetric>> {
    let metric = sql::get_as::<AgentMetric>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent metric", id))?;
    Ok(Json(metric))
}

#[utoipa::path(
    patch,
    path = "/v1/agent-metrics/{id}",
    tag = "metrics",
    params(("id" = Uuid, Path, description = "Metric id")),
    request_body = UpdateAgentMetric,
    responses((status = 200, body = AgentMetric), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgentMetric>,
) -> ApiResult<Json<AgentMetric>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<AgentMetric>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent metric", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agent-metrics/{id}",
    tag = "metrics",
    params(("id" = Uuid, Path, description = "Metric id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent metric", id))
    }
}
