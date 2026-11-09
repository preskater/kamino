//! Files bundled inside a skill version.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use kamino_common::error::{ApiResult, AppError};
use kamino_common::extract::{ApiQuery, ValidatedJson};
use kamino_common::pagination::{ListParams, Page};
use kamino_common::sql::{self, ListSpec, Resource, SqlType};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "skill_file",
    columns: &[
        ("id", SqlType::Uuid),
        ("skill_version_id", SqlType::Uuid),
        ("file_path", SqlType::Text),
        ("file_type", SqlType::Text),
        ("storage_url", SqlType::Text),
        ("file_size_bytes", SqlType::Int),
        ("checksum", SqlType::Text),
        ("metadata", SqlType::Jsonb),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["skill_version_id", "file_type"],
    searchable: &["file_path", "checksum"],
    sortable: &["file_path", "file_size_bytes"],
    default_sort: "file_path",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct SkillFile {
    pub id: Uuid,
    pub skill_version_id: Uuid,
    pub file_path: String,
    pub file_type: String,
    pub storage_url: String,
    pub file_size_bytes: i32,
    pub checksum: String,
    pub metadata: Value,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateSkillFile {
    #[validate(length(min = 1))]
    pub file_path: String,
    pub file_type: Option<String>,
    #[validate(length(min = 1))]
    pub storage_url: String,
    pub file_size_bytes: Option<i32>,
    pub checksum: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateSkillFile {
    #[validate(length(min = 1))]
    pub file_path: Option<String>,
    pub file_type: Option<String>,
    pub storage_url: Option<String>,
    pub file_size_bytes: Option<i32>,
    pub checksum: Option<String>,
    pub metadata: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/skill-versions/{version_id}/files",
            get(list_for_version).post(create_for_version),
        )
        .route(
            "/v1/skill-files/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/skill-versions/{version_id}/files",
    tag = "skills",
    params(
        ("version_id" = Uuid, Path, description = "Skill version id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "file_path | file_size_bytes"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("file_type" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<SkillFile>))
)]
pub async fn list_for_version(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<SkillFile>>> {
    params.with_filter("skill_version_id", version_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/skill-versions/{version_id}/files",
    tag = "skills",
    params(("version_id" = Uuid, Path, description = "Skill version id")),
    request_body = CreateSkillFile,
    responses((status = 201, body = SkillFile))
)]
pub async fn create_for_version(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateSkillFile>,
) -> ApiResult<(StatusCode, Json<SkillFile>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert(
        "skill_version_id".to_string(),
        json!(version_id.to_string()),
    );
    let created: SkillFile = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/skill-files/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill file id")),
    responses((status = 200, body = SkillFile), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SkillFile>> {
    let file = sql::get_as::<SkillFile>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("skill file", id))?;
    Ok(Json(file))
}

#[utoipa::path(
    patch,
    path = "/v1/skill-files/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill file id")),
    request_body = UpdateSkillFile,
    responses((status = 200, body = SkillFile), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateSkillFile>,
) -> ApiResult<Json<SkillFile>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<SkillFile>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("skill file", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/skill-files/{id}",
    tag = "skills",
    params(("id" = Uuid, Path, description = "Skill file id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("skill file", id))
    }
}
