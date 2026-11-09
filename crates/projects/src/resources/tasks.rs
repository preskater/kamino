//! Task endpoints.

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
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::activity;
use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "task",
    columns: &[
        ("id", SqlType::Uuid),
        ("project_id", SqlType::Uuid),
        ("assigned_agent_id", SqlType::Uuid),
        ("specialty", SqlType::Text),
        ("title", SqlType::Text),
        ("initial_prompt", SqlType::Text),
        ("expected_output", SqlType::Text),
        ("status", SqlType::Text),
        ("priority", SqlType::Text),
        ("input_context", SqlType::Jsonb),
        ("acceptance_criteria", SqlType::Jsonb),
        ("created_at", SqlType::Timestamptz),
        ("started_at", SqlType::Timestamptz),
        ("completed_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &[
        "project_id",
        "assigned_agent_id",
        "status",
        "priority",
        "specialty",
    ],
    searchable: &["title", "initial_prompt", "expected_output"],
    sortable: &[
        "created_at",
        "started_at",
        "completed_at",
        "title",
        "priority",
        "status",
    ],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub assigned_agent_id: Option<Uuid>,
    pub specialty: String,
    pub title: String,
    pub initial_prompt: String,
    pub expected_output: String,
    pub status: String,
    pub priority: String,
    pub input_context: Value,
    pub acceptance_criteria: Value,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateTask {
    /// Required on `POST /v1/tasks`, ignored on the nested project route.
    pub project_id: Option<Uuid>,
    pub assigned_agent_id: Option<Uuid>,
    pub specialty: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub initial_prompt: Option<String>,
    pub expected_output: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub input_context: Option<Value>,
    pub acceptance_criteria: Option<Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateTask {
    pub assigned_agent_id: Option<Uuid>,
    pub specialty: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    pub initial_prompt: Option<String>,
    pub expected_output: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub input_context: Option<Value>,
    pub acceptance_criteria: Option<Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/tasks", get(list).post(create))
        .route("/v1/tasks/{id}", get(fetch).patch(update).delete(remove))
        .route(
            "/v1/projects/{project_id}/tasks",
            get(list_for_project).post(create_for_project),
        )
}

#[utoipa::path(
    get,
    path = "/v1/tasks",
    tag = "tasks",
    params(
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "created_at | started_at | completed_at | title | priority | status"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("project_id" = Option<Uuid>, Query),
        ("assigned_agent_id" = Option<Uuid>, Query),
        ("status" = Option<String>, Query),
        ("priority" = Option<String>, Query),
        ("specialty" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<Task>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Task>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    get,
    path = "/v1/projects/{project_id}/tasks",
    tag = "tasks",
    params(("project_id" = Uuid, Path, description = "Project id")),
    responses((status = 200, body = Page<Task>))
)]
pub async fn list_for_project(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Task>>> {
    params.with_filter("project_id", project_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/tasks",
    tag = "tasks",
    request_body = CreateTask,
    responses((status = 201, body = Task), (status = 422, description = "Validation failed"))
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateTask>,
) -> ApiResult<(StatusCode, Json<Task>)> {
    let project_id = body
        .project_id
        .ok_or_else(|| AppError::BadRequest("`project_id` is required".into()))?;
    create_task(&state, project_id, body).await
}

#[utoipa::path(
    post,
    path = "/v1/projects/{project_id}/tasks",
    tag = "tasks",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateTask,
    responses((status = 201, body = Task), (status = 422, description = "Validation failed"))
)]
pub async fn create_for_project(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateTask>,
) -> ApiResult<(StatusCode, Json<Task>)> {
    create_task(&state, project_id, body).await
}

async fn create_task(
    state: &AppState,
    project_id: Uuid,
    body: CreateTask,
) -> ApiResult<(StatusCode, Json<Task>)> {
    let mut tx = state.pool.begin().await?;
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("project_id".to_string(), json!(project_id.to_string()));
    let created: Task = sql::insert_as(&mut *tx, &RESOURCE, &payload).await?;
    activity::record(
        &mut *tx,
        created.project_id,
        "task.created",
        "task",
        created.id,
        json!({ "title": created.title, "status": created.status }),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/tasks/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Task id")),
    responses((status = 200, body = Task), (status = 404, description = "Not found"))
)]
pub async fn fetch(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Task>> {
    let task = sql::get_as::<Task>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("task", id))?;
    Ok(Json(task))
}

#[utoipa::path(
    patch,
    path = "/v1/tasks/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Task id")),
    request_body = UpdateTask,
    responses((status = 200, body = Task), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateTask>,
) -> ApiResult<Json<Task>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<Task>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("task", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/tasks/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Task id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("task", id))
    }
}
