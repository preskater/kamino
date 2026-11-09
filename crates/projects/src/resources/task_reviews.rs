//! Task reviews.

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
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::activity;
use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "task_review",
    columns: &[
        ("id", SqlType::Uuid),
        ("task_id", SqlType::Uuid),
        ("decision", SqlType::Text),
        ("feedback", SqlType::Text),
        ("status", SqlType::Text),
        ("created_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["task_id", "decision", "status"],
    searchable: &["feedback"],
    sortable: &["created_at", "decision", "status"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct TaskReview {
    pub id: Uuid,
    pub task_id: Uuid,
    pub decision: String,
    pub feedback: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateTaskReview {
    #[validate(length(min = 1, max = 32))]
    pub decision: String,
    pub feedback: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateTaskReview {
    #[validate(length(min = 1, max = 32))]
    pub decision: Option<String>,
    pub feedback: Option<String>,
    pub status: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/tasks/{task_id}/reviews",
            get(list_for_task).post(create_for_task),
        )
        .route(
            "/v1/task-reviews/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/tasks/{task_id}/reviews",
    tag = "tasks",
    params(
        ("task_id" = Uuid, Path, description = "Task id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("decision" = Option<String>, Query),
        ("status" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<TaskReview>))
)]
pub async fn list_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<TaskReview>>> {
    params.with_filter("task_id", task_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/tasks/{task_id}/reviews",
    tag = "tasks",
    params(("task_id" = Uuid, Path, description = "Task id")),
    request_body = CreateTaskReview,
    responses((status = 201, body = TaskReview))
)]
pub async fn create_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateTaskReview>,
) -> ApiResult<(StatusCode, Json<TaskReview>)> {
    let mut tx = state.pool.begin().await?;
    let project_id = activity::project_for_task(&mut *tx, task_id).await?;
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("task_id".to_string(), json!(task_id.to_string()));
    let created: TaskReview = sql::insert_as(&mut *tx, &RESOURCE, &payload).await?;
    activity::record(
        &mut *tx,
        project_id,
        "task.review_added",
        "task_review",
        created.id,
        json!({ "task_id": task_id, "decision": created.decision }),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/task-reviews/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Review id")),
    responses((status = 200, body = TaskReview), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TaskReview>> {
    let review = sql::get_as::<TaskReview>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("task review", id))?;
    Ok(Json(review))
}

#[utoipa::path(
    patch,
    path = "/v1/task-reviews/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Review id")),
    request_body = UpdateTaskReview,
    responses((status = 200, body = TaskReview), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateTaskReview>,
) -> ApiResult<Json<TaskReview>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<TaskReview>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("task review", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/task-reviews/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Review id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("task review", id))
    }
}
