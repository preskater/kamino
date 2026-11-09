//! Activity log endpoints. Entries are append-only: create, read and delete.

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
    table: "activity_log",
    columns: &[
        ("id", SqlType::Uuid),
        ("project_id", SqlType::Uuid),
        ("action", SqlType::Text),
        ("entity_type", SqlType::Text),
        ("entity_id", SqlType::Uuid),
        ("metadata", SqlType::Jsonb),
        ("created_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["project_id", "action", "entity_type", "entity_id"],
    searchable: &[],
    sortable: &["created_at"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct ActivityLog {
    pub id: Uuid,
    pub project_id: Uuid,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateActivityLog {
    #[validate(length(min = 1, max = 120))]
    pub action: String,
    #[validate(length(min = 1, max = 64))]
    pub entity_type: String,
    pub entity_id: Uuid,
    pub metadata: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/projects/{project_id}/activity",
            get(list_for_project).post(create_for_project),
        )
        .route("/v1/activity-logs/{id}", get(fetch).delete(remove))
}

#[utoipa::path(
    get,
    path = "/v1/projects/{project_id}/activity",
    tag = "activity",
    params(
        ("project_id" = Uuid, Path, description = "Project id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "created_at"),
        ("order" = Option<String>, Query),
        ("action" = Option<String>, Query),
        ("entity_type" = Option<String>, Query),
        ("entity_id" = Option<Uuid>, Query)
    ),
    responses((status = 200, body = Page<ActivityLog>))
)]
pub async fn list_for_project(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<ActivityLog>>> {
    params.with_filter("project_id", project_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/projects/{project_id}/activity",
    tag = "activity",
    params(("project_id" = Uuid, Path, description = "Project id")),
    request_body = CreateActivityLog,
    responses((status = 201, body = ActivityLog))
)]
pub async fn create_for_project(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateActivityLog>,
) -> ApiResult<(StatusCode, Json<ActivityLog>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("project_id".to_string(), json!(project_id.to_string()));
    let created: ActivityLog = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/activity-logs/{id}",
    tag = "activity",
    params(("id" = Uuid, Path, description = "Activity log id")),
    responses((status = 200, body = ActivityLog), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ActivityLog>> {
    let entry = sql::get_as::<ActivityLog>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("activity log", id))?;
    Ok(Json(entry))
}

#[utoipa::path(
    delete,
    path = "/v1/activity-logs/{id}",
    tag = "activity",
    params(("id" = Uuid, Path, description = "Activity log id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("activity log", id))
    }
}
