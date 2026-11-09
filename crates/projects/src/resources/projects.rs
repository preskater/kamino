//! Project endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
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
    table: "project",
    columns: &[
        ("id", SqlType::Uuid),
        ("name", SqlType::Text),
        ("description", SqlType::Text),
        ("status", SqlType::Text),
        ("start_date", SqlType::Date),
        ("target_date", SqlType::Date),
        ("created_at", SqlType::Timestamptz),
        ("updated_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["status"],
    searchable: &["name", "description"],
    sortable: &[
        "created_at",
        "updated_at",
        "name",
        "start_date",
        "target_date",
    ],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub start_date: Option<NaiveDate>,
    pub target_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateProject {
    #[validate(length(min = 1, max = 160))]
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub target_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateProject {
    #[validate(length(min = 1, max = 160))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub target_date: Option<NaiveDate>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/projects", get(list).post(create))
        .route("/v1/projects/{id}", get(fetch).patch(update).delete(remove))
}

#[utoipa::path(
    get,
    path = "/v1/projects",
    tag = "projects",
    params(
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "created_at | updated_at | name | start_date | target_date"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("status" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<Project>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Project>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/projects",
    tag = "projects",
    request_body = CreateProject,
    responses((status = 201, body = Project), (status = 422, description = "Validation failed"))
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateProject>,
) -> ApiResult<(StatusCode, Json<Project>)> {
    let mut tx = state.pool.begin().await?;
    let payload = sql::prepare_create(&body)?;
    let created: Project = sql::insert_as(&mut *tx, &RESOURCE, &payload).await?;
    activity::record(
        &mut *tx,
        created.id,
        "project.created",
        "project",
        created.id,
        json!({ "name": created.name }),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/projects/{id}",
    tag = "projects",
    params(("id" = Uuid, Path, description = "Project id")),
    responses((status = 200, body = Project), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Project>> {
    let project = sql::get_as::<Project>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("project", id))?;
    Ok(Json(project))
}

#[utoipa::path(
    patch,
    path = "/v1/projects/{id}",
    tag = "projects",
    params(("id" = Uuid, Path, description = "Project id")),
    request_body = UpdateProject,
    responses((status = 200, body = Project), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateProject>,
) -> ApiResult<Json<Project>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<Project>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("project", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/projects/{id}",
    tag = "projects",
    params(("id" = Uuid, Path, description = "Project id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("project", id))
    }
}
