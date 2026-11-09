//! Skill endpoints: installable skill packages.

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
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "skill",
    columns: &[
        ("id", SqlType::Uuid),
        ("slug", SqlType::Text),
        ("name", SqlType::Text),
        ("description", SqlType::Text),
        ("version", SqlType::Text),
        ("source_type", SqlType::Text),
        ("source_url", SqlType::Text),
        ("storage_path", SqlType::Text),
        ("skill_md_content", SqlType::Text),
        ("metadata", SqlType::Jsonb),
        ("required_permissions", SqlType::Jsonb),
        ("is_active", SqlType::Bool),
        ("created_at", SqlType::Timestamptz),
        ("updated_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["is_active", "source_type"],
    searchable: &["name", "slug", "description"],
    sortable: &["created_at", "updated_at", "name", "slug"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Skill {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub source_type: String,
    pub source_url: Option<String>,
    pub storage_path: Option<String>,
    pub skill_md_content: Option<String>,
    pub metadata: Value,
    pub required_permissions: Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateSkill {
    #[validate(length(min = 1, max = 160))]
    pub slug: String,
    #[validate(length(min = 1, max = 160))]
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub source_type: Option<String>,
    pub source_url: Option<String>,
    pub storage_path: Option<String>,
    pub skill_md_content: Option<String>,
    pub metadata: Option<Value>,
    pub required_permissions: Option<Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSkill {
    #[validate(length(min = 1, max = 160))]
    pub slug: Option<String>,
    #[validate(length(min = 1, max = 160))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub source_type: Option<String>,
    pub source_url: Option<String>,
    pub storage_path: Option<String>,
    pub skill_md_content: Option<String>,
    pub metadata: Option<Value>,
    pub required_permissions: Option<Value>,
    pub is_active: Option<bool>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/skills", get(list).post(create))
        .route("/v1/skills/{id}", get(fetch).patch(update).delete(remove))
}

#[utoipa::path(
    get,
    path = "/v1/skills",
    tag = "skills",
    params(
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "created_at | updated_at | name | slug"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("is_active" = Option<bool>, Query),
        ("source_type" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<Skill>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Skill>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/skills",
    tag = "skills",
    request_body = CreateSkill,
    responses((status = 201, body = Skill), (status = 409, description = "Slug already exists"))
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateSkill>,
) -> ApiResult<(StatusCode, Json<Skill>)> {
    let payload = sql::prepare_create(&body)?;
    let created: Skill = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/skills/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill id")),
    responses((status = 200, body = Skill), (status = 404, description = "Not found"))
)]
pub async fn fetch(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Skill>> {
    let skill = sql::get_as::<Skill>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("skill", id))?;
    Ok(Json(skill))
}

#[utoipa::path(
    patch,
    path = "/v1/skills/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill id")),
    request_body = UpdateSkill,
    responses((status = 200, body = Skill), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateSkill>,
) -> ApiResult<Json<Skill>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<Skill>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("skill", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/skills/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("skill", id))
    }
}
