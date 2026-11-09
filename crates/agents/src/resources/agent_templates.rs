//! Agent template endpoints: reusable blueprints for configured agents.

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
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

pub static RESOURCE: Resource = Resource {
    table: "agent_template",
    columns: &[
        ("id", SqlType::Uuid),
        ("name", SqlType::Text),
        ("specialty", SqlType::Text),
        ("system_prompt", SqlType::Text),
        ("default_instructions", SqlType::Text),
        ("capabilities", SqlType::Jsonb),
        ("allowed_tools", SqlType::Jsonb),
        ("default_skill_ids", SqlType::Jsonb),
        ("default_model", SqlType::Text),
        ("is_active", SqlType::Bool),
        ("created_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["specialty", "default_model", "is_active"],
    searchable: &["name", "specialty", "default_model"],
    sortable: &["created_at", "name", "specialty"],
    default_sort: "created_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AgentTemplate {
    pub id: Uuid,
    pub name: String,
    pub specialty: String,
    pub system_prompt: String,
    pub default_instructions: String,
    pub capabilities: Value,
    pub allowed_tools: Value,
    pub default_skill_ids: Value,
    pub default_model: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgentTemplate {
    #[validate(length(min = 1, max = 160))]
    pub name: String,
    #[validate(length(min = 1, max = 160))]
    pub specialty: String,
    #[validate(length(min = 1))]
    pub system_prompt: String,
    pub default_instructions: Option<String>,
    pub capabilities: Option<Value>,
    pub allowed_tools: Option<Value>,
    pub default_skill_ids: Option<Value>,
    pub default_model: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgentTemplate {
    #[validate(length(min = 1, max = 160))]
    pub name: Option<String>,
    #[validate(length(min = 1, max = 160))]
    pub specialty: Option<String>,
    #[validate(length(min = 1))]
    pub system_prompt: Option<String>,
    pub default_instructions: Option<String>,
    pub capabilities: Option<Value>,
    pub allowed_tools: Option<Value>,
    pub default_skill_ids: Option<Value>,
    pub default_model: Option<String>,
    pub is_active: Option<bool>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/agent-templates", get(list).post(create))
        .route(
            "/v1/agent-templates/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agent-templates",
    tag = "agent-templates",
    params(
        ("limit" = Option<i64>, Query, description = "Page size, max 100"),
        ("offset" = Option<i64>, Query, description = "Rows to skip"),
        ("sort" = Option<String>, Query, description = "created_at | name | specialty"),
        ("order" = Option<String>, Query, description = "asc | desc"),
        ("q" = Option<String>, Query, description = "Search name, specialty and model"),
        ("specialty" = Option<String>, Query),
        ("default_model" = Option<String>, Query),
        ("is_active" = Option<bool>, Query)
    ),
    responses((status = 200, description = "Paginated agent templates", body = Page<AgentTemplate>))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<AgentTemplate>>> {
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agent-templates",
    tag = "agent-templates",
    request_body = CreateAgentTemplate,
    responses(
        (status = 201, description = "Created", body = AgentTemplate),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateAgentTemplate>,
) -> ApiResult<(StatusCode, Json<AgentTemplate>)> {
    let payload = sql::prepare_create(&body)?;
    let created: AgentTemplate = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agent-templates/{id}",
    tag = "agent-templates",
    params(("id" = Uuid, Path, description = "Agent template id")),
    responses(
        (status = 200, description = "Agent template", body = AgentTemplate),
        (status = 404, description = "Not found")
    )
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentTemplate>> {
    let template = sql::get_as::<AgentTemplate>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent template", id))?;
    Ok(Json(template))
}

#[utoipa::path(
    patch,
    path = "/v1/agent-templates/{id}",
    tag = "agent-templates",
    params(("id" = Uuid, Path, description = "Agent template id")),
    request_body = UpdateAgentTemplate,
    responses(
        (status = 200, description = "Updated", body = AgentTemplate),
        (status = 404, description = "Not found"),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgentTemplate>,
) -> ApiResult<Json<AgentTemplate>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<AgentTemplate>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent template", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agent-templates/{id}",
    tag = "agent-templates",
    params(("id" = Uuid, Path, description = "Agent template id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
        (status = 400, description = "Template is still referenced by agents")
    )
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent template", id))
    }
}
