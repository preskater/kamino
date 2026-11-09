//! Task artifacts.

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
    table: "artifact",
    columns: &[
        ("id", SqlType::Uuid),
        ("task_id", SqlType::Uuid),
        ("agent_run_id", SqlType::Uuid),
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
    filterable: &["task_id", "agent_run_id", "artifact_type"],
    searchable: &["name", "content"],
    sortable: &["created_at", "name", "artifact_type"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Artifact {
    pub id: Uuid,
    pub task_id: Uuid,
    pub agent_run_id: Option<Uuid>,
    pub artifact_type: String,
    pub name: String,
    pub storage_url: String,
    pub content: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateArtifact {
    /// Required on `POST /v1/artifacts`, ignored on the nested task route.
    pub task_id: Option<Uuid>,
    pub agent_run_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub storage_url: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateArtifact {
    pub agent_run_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub storage_url: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/artifacts", get(list).post(create))
        .route(
            "/v1/artifacts/{id}",
            get(fetch).patch(update).delete(remove),
        )
        .route(
            "/v1/tasks/{task_id}/artifacts",
            get(list_for_task).post(create_for_task),
        )
}

#[utoipa::path(
    get,
    path = "/v1/artifacts",
    tag = "tasks",
    params(
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "created_at | name | artifact_type"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("task_id" = Option<Uuid>, Query),
        ("agent_run_id" = Option<Uuid>, Query),
        ("artifact_type" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<Artifact>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Artifact>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    get,
    path = "/v1/tasks/{task_id}/artifacts",
    tag = "tasks",
    params(("task_id" = Uuid, Path, description = "Task id")),
    responses((status = 200, body = Page<Artifact>))
)]
pub async fn list_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Artifact>>> {
    params.with_filter("task_id", task_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/artifacts",
    tag = "tasks",
    request_body = CreateArtifact,
    responses((status = 201, body = Artifact))
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateArtifact>,
) -> ApiResult<(StatusCode, Json<Artifact>)> {
    let task_id = body
        .task_id
        .ok_or_else(|| AppError::BadRequest("`task_id` is required".into()))?;
    create_artifact(&state, task_id, body).await
}

#[utoipa::path(
    post,
    path = "/v1/tasks/{task_id}/artifacts",
    tag = "tasks",
    params(("task_id" = Uuid, Path, description = "Task id")),
    request_body = CreateArtifact,
    responses((status = 201, body = Artifact))
)]
pub async fn create_for_task(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateArtifact>,
) -> ApiResult<(StatusCode, Json<Artifact>)> {
    create_artifact(&state, task_id, body).await
}

async fn create_artifact(
    state: &AppState,
    task_id: Uuid,
    body: CreateArtifact,
) -> ApiResult<(StatusCode, Json<Artifact>)> {
    let mut tx = state.pool.begin().await?;
    let project_id = activity::project_for_task(&mut *tx, task_id).await?;
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("task_id".to_string(), json!(task_id.to_string()));
    let created: Artifact = sql::insert_as(&mut *tx, &RESOURCE, &payload).await?;
    activity::record(
        &mut *tx,
        project_id,
        "task.artifact_added",
        "artifact",
        created.id,
        json!({ "task_id": task_id, "name": created.name }),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/artifacts/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Artifact id")),
    responses((status = 200, body = Artifact), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Artifact>> {
    let artifact = sql::get_as::<Artifact>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("artifact", id))?;
    Ok(Json(artifact))
}

#[utoipa::path(
    patch,
    path = "/v1/artifacts/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Artifact id")),
    request_body = UpdateArtifact,
    responses((status = 200, body = Artifact), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateArtifact>,
) -> ApiResult<Json<Artifact>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<Artifact>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("artifact", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/artifacts/{id}",
    tag = "tasks",
    params(("id" = Uuid, Path, description = "Artifact id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("artifact", id))
    }
}
