//! Agent endpoints: configured instances created from a template.

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

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "agent",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_template_id", SqlType::Uuid),
        ("name", SqlType::Text),
        ("specialty", SqlType::Text),
        ("status", SqlType::Text),
        ("configuration", SqlType::Jsonb),
        ("tool_permissions", SqlType::Jsonb),
        ("model", SqlType::Text),
        ("created_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_template_id", "status", "specialty", "model"],
    searchable: &["name", "specialty", "model"],
    sortable: &["created_at", "name", "status", "specialty"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct Agent {
    pub id: Uuid,
    pub agent_template_id: Uuid,
    pub name: String,
    pub specialty: String,
    pub status: String,
    pub configuration: Value,
    pub tool_permissions: Value,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgent {
    /// Required on `POST /v1/agents`, ignored on the nested template route.
    pub agent_template_id: Option<Uuid>,
    #[validate(length(min = 1, max = 160))]
    pub name: String,
    #[validate(length(min = 1, max = 160))]
    pub specialty: String,
    pub status: Option<String>,
    pub configuration: Option<Value>,
    pub tool_permissions: Option<Value>,
    #[validate(length(min = 1))]
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgent {
    #[validate(length(min = 1, max = 160))]
    pub name: Option<String>,
    #[validate(length(min = 1, max = 160))]
    pub specialty: Option<String>,
    pub status: Option<String>,
    pub configuration: Option<Value>,
    pub tool_permissions: Option<Value>,
    pub model: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/agents", get(list).post(create))
        .route("/v1/agents/{id}", get(fetch).patch(update).delete(remove))
        .route(
            "/v1/agent-templates/{template_id}/agents",
            get(list_for_template).post(create_for_template),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agents",
    tag = "agents",
    params(
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "created_at | name | status | specialty"),
        ("order" = Option<String>, Query),
        ("q" = Option<String>, Query),
        ("agent_template_id" = Option<Uuid>, Query),
        ("status" = Option<String>, Query),
        ("specialty" = Option<String>, Query),
        ("model" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<Agent>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Agent>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    get,
    path = "/v1/agent-templates/{template_id}/agents",
    tag = "agents",
    params(("template_id" = Uuid, Path, description = "Agent template id")),
    responses((status = 200, body = Page<Agent>))
)]
pub async fn list_for_template(
    State(state): State<AppState>,
    Path(template_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<Agent>>> {
    params.with_filter("agent_template_id", template_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agents",
    tag = "agents",
    request_body = CreateAgent,
    responses(
        (status = 201, body = Agent),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateAgent>,
) -> ApiResult<(StatusCode, Json<Agent>)> {
    if body.agent_template_id.is_none() {
        return Err(AppError::BadRequest(
            "`agent_template_id` is required when creating an agent directly".into(),
        ));
    }
    let payload = sql::prepare_create(&body)?;
    let created: Agent = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    post,
    path = "/v1/agent-templates/{template_id}/agents",
    tag = "agents",
    params(("template_id" = Uuid, Path, description = "Agent template id")),
    request_body = CreateAgent,
    responses(
        (status = 201, body = Agent),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn create_for_template(
    State(state): State<AppState>,
    Path(template_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateAgent>,
) -> ApiResult<(StatusCode, Json<Agent>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert(
        "agent_template_id".to_string(),
        json!(template_id.to_string()),
    );
    let created: Agent = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agents/{id}",
    tag = "agents",
    params(("id" = Uuid, Path, description = "Agent id")),
    responses((status = 200, body = Agent), (status = 404, description = "Not found"))
)]
pub async fn fetch(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Agent>> {
    let agent = sql::get_as::<Agent>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent", id))?;
    Ok(Json(agent))
}

#[utoipa::path(
    patch,
    path = "/v1/agents/{id}",
    tag = "agents",
    params(("id" = Uuid, Path, description = "Agent id")),
    request_body = UpdateAgent,
    responses(
        (status = 200, body = Agent),
        (status = 404, description = "Not found"),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgent>,
) -> ApiResult<Json<Agent>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<Agent>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agents/{id}",
    tag = "agents",
    params(("id" = Uuid, Path, description = "Agent id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent", id))
    }
}
