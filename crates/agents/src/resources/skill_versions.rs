//! Skill version endpoints, including the `publish` transition.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
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
    table: "skill_version",
    columns: &[
        ("id", SqlType::Uuid),
        ("skill_id", SqlType::Uuid),
        ("version", SqlType::Text),
        ("changelog", SqlType::Text),
        ("storage_path", SqlType::Text),
        ("skill_md_content", SqlType::Text),
        ("metadata", SqlType::Jsonb),
        ("is_current", SqlType::Bool),
        ("published_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["skill_id", "is_current"],
    searchable: &["version", "changelog"],
    sortable: &["published_at", "version"],
    default_sort: "published_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct SkillVersion {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version: String,
    pub changelog: Option<String>,
    pub storage_path: Option<String>,
    pub skill_md_content: Option<String>,
    pub metadata: Value,
    pub is_current: bool,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateSkillVersion {
    #[validate(length(min = 1, max = 64))]
    pub version: String,
    pub changelog: Option<String>,
    pub storage_path: Option<String>,
    pub skill_md_content: Option<String>,
    pub metadata: Option<Value>,
    pub is_current: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSkillVersion {
    #[validate(length(min = 1, max = 64))]
    pub version: Option<String>,
    pub changelog: Option<String>,
    pub storage_path: Option<String>,
    pub skill_md_content: Option<String>,
    pub metadata: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/skills/{skill_id}/versions",
            get(list_for_skill).post(create_for_skill),
        )
        .route(
            "/v1/skill-versions/{id}",
            get(fetch).patch(update).delete(remove),
        )
        .route("/v1/skill-versions/{id}/publish", post(publish))
}

#[utoipa::path(
    get,
    path = "/v1/skills/{skill_id}/versions",
    tag = "skills",
    params(
        ("skill_id" = Uuid, Path, description = "Skill id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "published_at | version"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("is_current" = Option<bool>, Query)
    ),
    responses((status = 200, body = Page<SkillVersion>))
)]
pub async fn list_for_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<SkillVersion>>> {
    params.with_filter("skill_id", skill_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/skills/{skill_id}/versions",
    tag = "skills",
    params(("skill_id" = Uuid, Path, description = "Skill id")),
    request_body = CreateSkillVersion,
    responses((status = 201, body = SkillVersion), (status = 409, description = "Version already exists"))
)]
pub async fn create_for_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateSkillVersion>,
) -> ApiResult<(StatusCode, Json<SkillVersion>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("skill_id".to_string(), json!(skill_id.to_string()));
    let created: SkillVersion = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/skill-versions/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill version id")),
    responses((status = 200, body = SkillVersion), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SkillVersion>> {
    let version = sql::get_as::<SkillVersion>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("skill version", id))?;
    Ok(Json(version))
}

#[utoipa::path(
    patch,
    path = "/v1/skill-versions/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill version id")),
    request_body = UpdateSkillVersion,
    responses((status = 200, body = SkillVersion), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateSkillVersion>,
) -> ApiResult<Json<SkillVersion>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<SkillVersion>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("skill version", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/skill-versions/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill version id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("skill version", id))
    }
}

/// Marks this version as the current one for its skill, demoting any sibling.
#[utoipa::path(
    post,
    path = "/v1/skill-versions/{id}/publish",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill version id")),
    responses((status = 200, body = SkillVersion), (status = 404, description = "Not found"))
)]
pub async fn publish(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SkillVersion>> {
    let mut tx = state.pool.begin().await?;

    let version = sqlx::query_as::<_, SkillVersion>("SELECT * FROM skill_version WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("skill version", id))?;

    sqlx::query("UPDATE skill_version SET is_current = false WHERE skill_id = $1 AND id <> $2")
        .bind(version.skill_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let published = sqlx::query_as::<_, SkillVersion>(
        "UPDATE skill_version SET is_current = true, published_at = now() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE skill SET version = $1, updated_at = now() WHERE id = $2")
        .bind(&published.version)
        .bind(version.skill_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(published))
}
