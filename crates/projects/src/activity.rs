//! Append-only project activity log.

use kamino_error::{ApiResult, AppError};
use serde_json::Value;
use sqlx::Postgres;
use uuid::Uuid;

/// Resolves the owning project for a task, or 404s.
pub async fn project_for_task<'e, E>(executor: E, task_id: Uuid) -> ApiResult<Uuid>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar("SELECT project_id FROM task WHERE id = $1")
        .bind(task_id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| AppError::not_found("task", task_id))
}

/// Appends one audit row. Callers pass the surrounding transaction so the entry
/// and the mutation it describes commit together.
pub async fn record<'e, E>(
    executor: E,
    project_id: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    metadata: Value,
) -> ApiResult<()>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    sqlx::query(
        "INSERT INTO activity_log (id, project_id, action, entity_type, entity_id, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(project_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(metadata)
    .execute(executor)
    .await?;

    Ok(())
}
