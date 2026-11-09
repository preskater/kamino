//! Task comments.

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
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::activity;
use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "task_comment",
    columns: &[
        ("id", SqlType::Uuid),
        ("task_id", SqlType::Uuid),
        ("content", SqlType::Text),
        ("created_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["task_id"],
    searchable: &["content"],
    sortable: &["created_at"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct TaskComment {
    pub id: Uuid,
    pub task_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateTaskComment {
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateTaskComment {
    #[validate(length(min = 1))]
    pub content: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/tasks/{task_id}/comments",
            get(list_for_task).post(create_for_task),
        )
        .route(
            "/v1/task-comments/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/tasks/{task_id}/comments",
    tag = "tasks",
    params(
        ("task_id" = Uuid, Path, description = "Task id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<TaskComment>))
)]
pub async fn list_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<TaskComment>>> {
    params.with_filter("task_id", task_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/tasks/{task_id}/comments",
    tag = "tasks",
    params(("task_id" = Uuid, Path, description = "Task id")),
    request_body = CreateTaskComment,
    responses((status = 201, body = TaskComment))
)]
pub async fn create_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateTaskComment>,
) -> ApiResult<(StatusCode, Json<TaskComment>)> {
    let mut tx = state.pool.begin().await?;
    let project_id = activity::project_for_task(&mut *tx, task_id).await?;
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("task_id".to_string(), json!(task_id.to_string()));
    let created: TaskComment = sql::insert_as(&mut *tx, &RESOURCE, &payload).await?;
    activity::record(
        &mut *tx,
        project_id,
        "task.comment_added",
        "task_comment",
        created.id,
        json!({ "task_id": task_id }),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/task-comments/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Comment id")),
    responses((status = 200, body = TaskComment), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TaskComment>> {
    let comment = sql::get_as::<TaskComment>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("task comment", id))?;
    Ok(Json(comment))
}

#[utoipa::path(
    patch,
    path = "/v1/task-comments/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Comment id")),
    request_body = UpdateTaskComment,
    responses((status = 200, body = TaskComment), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateTaskComment>,
) -> ApiResult<Json<TaskComment>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<TaskComment>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("task comment", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/task-comments/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Comment id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("task comment", id))
    }
}
