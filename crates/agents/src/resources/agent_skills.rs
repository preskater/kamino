//! Skills enabled for a specific agent.

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
    table: "agent_skill",
    columns: &[
        ("id", SqlType::Uuid),
        ("agent_id", SqlType::Uuid),
        ("skill_id", SqlType::Uuid),
        ("status", SqlType::Text),
        ("configuration", SqlType::Jsonb),
        ("enabled_at", SqlType::Timestamptz),
    ],
};

pub static LIST: ListSpec = ListSpec {
    resource: &RESOURCE,
    filterable: &["agent_id", "skill_id", "status"],
    searchable: &[],
    sortable: &["enabled_at", "status"],
    default_sort: "enabled_at",
};

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AgentSkill {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub skill_id: Uuid,
    pub status: String,
    pub configuration: Value,
    pub enabled_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAgentSkill {
    pub skill_id: Uuid,
    pub status: Option<String>,
    pub configuration: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateAgentSkill {
    pub skill_id: Option<Uuid>,
    pub status: Option<String>,
    pub configuration: Option<Value>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/agents/{agent_id}/skills",
            get(list_for_agent).post(create_for_agent),
        )
        .route(
            "/v1/agent-skills/{id}",
            get(fetch).patch(update).delete(remove),
        )
}

#[utoipa::path(
    get,
    path = "/v1/agents/{agent_id}/skills",
    tag = "agents",
    params(
        ("agent_id" = Uuid, Path, description = "Agent id"),
        ("limit" = Option<i64>, Query),
        ("offset" = Option<i64>, Query),
        ("sort" = Option<String>, Query, description = "enabled_at | status"),
        ("order" = Option<String>, Query),
        ("skill_id" = Option<Uuid>, Query),
        ("status" = Option<String>, Query)
    ),
    responses((status = 200, body = Page<AgentSkill>))
)]
pub async fn list_for_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    ApiQuery(mut params): ApiQuery<ListParams>,
) -> ApiResult<Json<Page<AgentSkill>>> {
    params.with_filter("agent_id", agent_id.to_string());
    Ok(Json(sql::list_as(&state.pool, &LIST, &params).await?))
}

#[utoipa::path(
    post,
    path = "/v1/agents/{agent_id}/skills",
    tag = "agents",
    params(("agent_id" = Uuid, Path, description = "Agent id")),
    request_body = CreateAgentSkill,
    responses((status = 201, body = AgentSkill), (status = 409, description = "Skill already enabled"))
)]
pub async fn create_for_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<CreateAgentSkill>,
) -> ApiResult<(StatusCode, Json<AgentSkill>)> {
    let mut payload = sql::prepare_create(&body)?;
    payload.insert("agent_id".to_string(), json!(agent_id.to_string()));
    let created: AgentSkill = sql::insert_as(&state.pool, &RESOURCE, &payload).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    get,
    path = "/v1/agent-skills/{id}",
    tag = "agents",
    params(("id" = Uuid, Path, description = "Agent skill id")),
    responses((status = 200, body = AgentSkill), (status = 404, description = "Not found"))
)]
pub async fn fetch(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AgentSkill>> {
    let link = sql::get_as::<AgentSkill>(&state.pool, &RESOURCE, id)
        .await?
        .ok_or_else(|| AppError::not_found("agent skill", id))?;
    Ok(Json(link))
}

#[utoipa::path(
    patch,
    path = "/v1/agent-skills/{id}",
    tag = "agents",
    params(("id" = Uuid, Path, description = "Agent skill id")),
    request_body = UpdateAgentSkill,
    responses((status = 200, body = AgentSkill), (status = 404, description = "Not found"))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateAgentSkill>,
) -> ApiResult<Json<AgentSkill>> {
    let payload = sql::to_map(&body)?;
    let updated = sql::update_as::<AgentSkill>(&state.pool, &RESOURCE, id, &payload)
        .await?
        .ok_or_else(|| AppError::not_found("agent skill", id))?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/agent-skills/{id}",
    tag = "agents",
    params(("id" = Uuid, Path, description = "Agent skill id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found"))
)]
pub async fn remove(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<StatusCode> {
    if sql::delete(&state.pool, &RESOURCE, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::not_found("agent skill", id))
    }
}
