//! Router level tests against a throwaway Postgres database.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kamino_agents::{AppState, app};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

fn state(pool: PgPool) -> AppState {
    AppState::new(
        pool,
        kamino_agents::config::Config {
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
    assert!(body["paths"]["/v1/agents"].is_object());
    assert!(body["paths"]["/v1/skill-versions/{id}/publish"].is_object());
}

#[sqlx::test(migrations = "./migrations")]
async fn agent_crud_lifecycle(pool: PgPool) {
    let app = app(state(pool));

    let (status, template) = call(
        &app,
        "POST",
        "/v1/agent-templates",
        Some(json!({
            "name": "Researcher",
            "specialty": "research",
            "system_prompt": "You research things.",
            "default_model": "gpt-x"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{template}");
    let template_id = template["id"].as_str().unwrap().to_string();

    // Nested create forces the template id.
    let (status, agent) = call(
        &app,
        "POST",
        &format!("/v1/agent-templates/{template_id}/agents"),
        Some(json!({ "name": "Researcher 1", "specialty": "research", "model": "gpt-x" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{agent}");
    assert_eq!(agent["agent_template_id"], template_id);
    let agent_id = agent["id"].as_str().unwrap().to_string();

    let (status, page) = call(
        &app,
        "GET",
        &format!("/v1/agent-templates/{template_id}/agents"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["total"], 1);
    assert_eq!(page["items"][0]["id"], agent_id);

    let (status, updated) = call(
        &app,
        "PATCH",
        &format!("/v1/agents/{agent_id}"),
        Some(json!({ "status": "inactive" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "patch body: {updated}");
    assert_eq!(updated["status"], "inactive");

    let (status, _) = call(&app, "DELETE", &format!("/v1/agents/{agent_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = call(&app, "GET", &format!("/v1/agents/{agent_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn skill_slug_conflicts_and_versions_publish(pool: PgPool) {
    let app = app(state(pool));

    let (status, skill) = call(
        &app,
        "POST",
        "/v1/skills",
        Some(json!({ "slug": "pdf-tools", "name": "PDF tools" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{skill}");
    let skill_id = skill["id"].as_str().unwrap().to_string();

    let (status, error) = call(
        &app,
        "POST",
        "/v1/skills",
        Some(json!({ "slug": "pdf-tools", "name": "Duplicate" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "conflict");

    let (status, v1) = call(
        &app,
        "POST",
        &format!("/v1/skills/{skill_id}/versions"),
        Some(json!({ "version": "1.0.0", "changelog": "first" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let v1_id = v1["id"].as_str().unwrap().to_string();

    let (status, v2) = call(
        &app,
        "POST",
        &format!("/v1/skills/{skill_id}/versions"),
        Some(json!({ "version": "1.1.0" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let v2_id = v2["id"].as_str().unwrap().to_string();

    let (status, published) = call(
        &app,
        "POST",
        &format!("/v1/skill-versions/{v1_id}/publish"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(published["is_current"], true);

    let (_, published) = call(
        &app,
        "POST",
        &format!("/v1/skill-versions/{v2_id}/publish"),
        None,
    )
    .await;
    assert_eq!(published["is_current"], true);

    let (_, fetched) = call(&app, "GET", &format!("/v1/skill-versions/{v1_id}"), None).await;
    assert_eq!(fetched["is_current"], false);

    let (_, skill) = call(&app, "GET", &format!("/v1/skills/{skill_id}"), None).await;
    assert_eq!(skill["version"], "1.1.0");

    let (status, file) = call(
        &app,
        "POST",
        &format!("/v1/skill-versions/{v2_id}/files"),
        Some(json!({ "file_path": "SKILL.md", "storage_url": "s3://bucket/SKILL.md" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(file["skill_version_id"], v2_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn validation_and_query_errors_use_the_envelope(pool: PgPool) {
    let app = app(state(pool));

    let (status, body) = call(
        &app,
        "POST",
        "/v1/skills",
        Some(json!({ "slug": "x", "name": "" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation_failed");
    assert!(body["error"]["details"]["name"].is_array());

    let (status, body) = call(&app, "GET", "/v1/agents?bogus=1", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "bad_request");

    let (status, body) = call(&app, "GET", "/v1/agents?sort=nope", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "bad_request");

    let (status, _) = call(&app, "GET", "/v1/agents?limit=500", None).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_pagination_reports_total(pool: PgPool) {
    let (status, _) = {
        let app = app(state(pool.clone()));
        call(
            &app,
            "POST",
            "/v1/agent-templates",
            Some(json!({
                "name": "T",
                "specialty": "s",
                "system_prompt": "p"
            })),
        )
        .await
    };
    assert_eq!(status, StatusCode::CREATED);

    let app = app(state(pool));
    let (status, page) = call(&app, "GET", "/v1/agent-templates?limit=1&offset=0", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["total"], 1);
    assert_eq!(page["limit"], 1);
    assert_eq!(page["offset"], 0);
}
