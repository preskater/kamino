//! Skills activated during a specific run.

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
    table: "task_skill_usage",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_run_id", SqlType::Uuid),
        ("skill_id", SqlType::Uuid),
        ("activation_reason", SqlType::Text),
        ("loaded_files", SqlType::Jsonb),
        ("activated_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_run_id", "skill_id"],
    searchable: &["activation_reason"],
    sortable: &["activated_at"],
    default_sort: "activated_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct TaskSkillUsage {
    pub id: Uuid,
    pub agent_run_id: Uuid,
    pub skill_id: Uuid,
    pub activation_reason: String,
    pub loaded_files: Value,
    pub activated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateTaskSkillUsage {
    pub skill_id: Uuid,
    pub activation_reason: Option<String>,
    pub loaded_files: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateTaskSkillUsage {
    pub skill_id: Option<Uuid>,
    pub activation_reason: Option<String>,
    pub loaded_files: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/agent-runs/{run_id}/skill-usages",
            get(list_for_run).post(create_for_run),
        )
        .route(
            "/v1/skill-usages/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agent-runs/{run_id}/skill-usages",
    tag = "runs",
    params(
        ("run_id" = Uuid, Path, description = "Agent run id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("skill_id" = Option<Uuid>, Query)
    ),
    responses((status = 200, body = Page<TaskSkillUsage>))
)]
pub async fn list_for_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<TaskSkillUsage>>> {
    params.with_filter("agent_run_id", run_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agent-runs/{run_id}/skill-usages",
    tag = "runs",
    params(("run_id" = Uuid, Path, description = "Agent run id")),
    request_body = CreateTaskSkillUsage,
    responses((status = 201, body = TaskSkillUsage))
)]
pub async fn create_for_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateTaskSkillUsage>,
) -> ApiResult<(StatusCode, Json<TaskSkillUsage>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("agent_run_id".to_string(), json!(run_id.to_string()));
    let created: TaskSkillUsage = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/skill-usages/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Skill usage id")),
    responses((status = 200, body = TaskSkillUsage), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TaskSkillUsage>> {
    let usage = sql::get_as::<TaskSkillUsage>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("skill usage", id))?;
    Ok(Json(usage))
}

#[utoipa::path(
    patch,
    path = "/v1/skill-usages/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Skill usage id")),
    request_body = UpdateTaskSkillUsage,
    responses((status = 200, body = TaskSkillUsage), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateTaskSkillUsage>,
) -> ApiResult<Json<TaskSkillUsage>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<TaskSkillUsage>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("skill usage", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/skill-usages/{id}",
    tag = "runs",
    params(("id" = Uuid, Path, description = "Skill usage id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("skill usage", id))
    }
}
