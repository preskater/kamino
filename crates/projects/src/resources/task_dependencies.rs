//! Task dependency edges (blocks / depends on).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use kamino_common::error::{ApiResult, AppError};
use kamino_common::extract::{ApiQuery, ValidatedJson};
use kamino_common::pagination::{ListParams, Page};
use kamino_common::sql::{self, ListSpec, Resource, SqlType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "task_dependency",
    columns: &[
        ("id", SqlType::Uuid),
        ("task_id", SqlType::Uuid),
        ("depends_on_task_id", SqlType::Uuid),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["task_id", "depends_on_task_id"],
    searchable: &[],
    sortable: &["id"],
    default_sort: "id",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct TaskDependency {
    pub id: Uuid,
    pub task_id: Uuid,
    pub depends_on_task_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateTaskDependency {
    pub depends_on_task_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateTaskDependency {
    pub depends_on_task_id: Option<Uuid>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/tasks/{task_id}/dependencies",
            get(list_for_task).post(create_for_task),
        )
        .route("/v1/tasks/{task_id}/dependents", get(list_dependents))
        .route(
            "/v1/task-dependencies/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/tasks/{task_id}/dependencies",
    tag = "tasks",
    params(
        ("task_id" = Uuid, Path, description = "Task id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query)
    ),
    responses((status = 200, body = Page<TaskDependency>))
)]
pub async fn list_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<TaskDependency>>> {
    params.with_filter("task_id", task_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    get,
    path = "/v1/tasks/{task_id}/dependents",
    tag = "tasks",
    params(
        ("task_id" = Uuid, Path, description = "Task id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query)
    ),
    responses((status = 200, body = Page<TaskDependency>))
)]
pub async fn list_dependents(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<TaskDependency>>> {
    params.with_filter("depends_on_task_id", task_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/tasks/{task_id}/dependencies",
    tag = "tasks",
    params(("task_id" = Uuid, Path, description = "Task id")),
    request_body = CreateTaskDependency,
    responses(
        (status = 201, body = TaskDependency),
        (status = 409, description = "Dependency already exists")
    )
)]
pub async fn create_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateTaskDependency>,
) -> ApiResult<(StatusCode, Json<TaskDependency>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("task_id".to_string(), json!(task_id.to_string()));
    let created: TaskDependency = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/task-dependencies/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Dependency id")),
    responses((status = 200, body = TaskDependency), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TaskDependency>> {
    let dependency = sql::get_as::<TaskDependency>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("task dependency", id))?;
    Ok(Json(dependency))
}

#[utoipa::path(
    patch,
    path = "/v1/task-dependencies/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Dependency id")),
    request_body = UpdateTaskDependency,
    responses((status = 200, body = TaskDependency), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateTaskDependency>,
) -> ApiResult<Json<TaskDependency>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<TaskDependency>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("task dependency", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/task-dependencies/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Dependency id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("task dependency", id))
    }
}
