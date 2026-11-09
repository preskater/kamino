//! Router level tests against a throwaway Postgres database.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kamino_projects::{AppState, app};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

fn state(pool: PgPool) -> AppState {
    AppState::new(
        pool,
        kamino_projects::config::Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            database_url: String::new(),
            db_max_connections: 1,
        },
    )
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(match body {
            Some(value) => Body::from(serde_json::to_vec(&value).unwrap()),
            None => Body::empty(),
        })
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };

    (status, payload)
}

#[sqlx::test(migrations = "./migrations")]
async fn health_and_openapi_are_exposed(pool: PgPool) {
    let app = app(state(pool));

    let (status, body) = call(&app, "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    let (status, body) = call(&app, "GET", "/api-docs/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["paths"]["/v1/projects"].is_object());
    assert!(body["paths"]["/v1/activity-logs/{id}"].is_object());
}

#[sqlx::test(migrations = "./migrations")]
async fn project_lifecycle_writes_activity(pool: PgPool) {
    let app = app(state(pool));

    let (status, project) = call(
        &app,
        "POST",
        "/v1/projects",
        Some(json!({ "name": "Launch", "status": "active" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{project}");
    let project_id = project["id"].as_str().unwrap().to_string();

    let (status, activity) = call(
        &app,
        "GET",
        &format!("/v1/projects/{project_id}/activity"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(activity["total"], 1);
    assert_eq!(activity["items"][0]["action"], "project.created");

    let (status, updated) = call(
        &app,
        "PATCH",
        &format!("/v1/projects/{project_id}"),
        Some(json!({ "status": "completed" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["status"], "completed");
    assert!(updated["updated_at"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn task_children_and_dependencies(pool: PgPool) {
    let app = app(state(pool));

    let (_, project) = call(&app, "POST", "/v1/projects", Some(json!({ "name": "P" }))).await;
    let project_id = project["id"].as_str().unwrap().to_string();

    let (status, task) = call(
        &app,
        "POST",
        &format!("/v1/projects/{project_id}/tasks"),
        Some(json!({
            "title": "Draft spec",
            "priority": "high",
            "acceptance_criteria": ["exists"]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{task}");
    assert_eq!(task["project_id"], project_id);
    let task_id = task["id"].as_str().unwrap().to_string();

    let (status, blocker) = call(
        &app,
        "POST",
        &format!("/v1/projects/{project_id}/tasks"),
        Some(json!({ "title": "Approve spec" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let blocker_id = blocker["id"].as_str().unwrap().to_string();

    let (status, dependency) = call(
        &app,
        "POST",
        &format!("/v1/tasks/{task_id}/dependencies"),
        Some(json!({ "depends_on_task_id": blocker_id })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(dependency["task_id"], task_id);

    let (status, dependents) = call(
        &app,
        "GET",
        &format!("/v1/tasks/{blocker_id}/dependents"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(dependents["total"], 1);

    let (status, _) = call(
        &app,
        "POST",
        &format!("/v1/tasks/{task_id}/comments"),
        Some(json!({ "content": "looks good" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, review) = call(
        &app,
        "POST",
        &format!("/v1/tasks/{task_id}/reviews"),
        Some(json!({ "decision": "approved", "feedback": "ship it" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(review["status"], "pending");

    let (status, artifact) = call(
        &app,
        "POST",
        &format!("/v1/tasks/{task_id}/artifacts"),
        Some(json!({ "name": "spec.md", "storage_url": "s3://bucket/spec.md" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, artifacts) = call(&app, "GET", &format!("/v1/tasks/{task_id}/artifacts"), None).await;
    assert_eq!(artifacts["total"], 1);
    assert_eq!(artifacts["items"][0]["id"], artifact["id"]);

    let (_, activity) = call(
        &app,
        "GET",
        &format!("/v1/projects/{project_id}/activity"),
        None,
    )
    .await;
    // project.created + 2 task.created + comment + review + artifact
    assert_eq!(activity["total"], 6);
}

#[sqlx::test(migrations = "./migrations")]
async fn activity_logs_are_append_only(pool: PgPool) {
    let app = app(state(pool));

    let (_, project) = call(&app, "POST", "/v1/projects", Some(json!({ "name": "P" }))).await;
    let project_id = project["id"].as_str().unwrap().to_string();

    let (status, entry) = call(
        &app,
        "POST",
        &format!("/v1/projects/{project_id}/activity"),
        Some(json!({
            "action": "manual.note",
            "entity_type": "project",
            "entity_id": project_id
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let entry_id = entry["id"].as_str().unwrap().to_string();

    let (status, _) = call(
        &app,
        "PATCH",
        &format!("/v1/activity-logs/{entry_id}"),
        Some(json!({ "action": "edited" })),
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

    let (status, _) = call(
        &app,
        "DELETE",
        &format!("/v1/activity-logs/{entry_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(&app, "GET", &format!("/v1/activity-logs/{entry_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_sort_is_rejected(pool: PgPool) {
    let app = app(state(pool));
    let (status, body) = call(&app, "GET", "/v1/tasks?sort=drop_table", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "bad_request");
}
