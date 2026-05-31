//! Integration tests for the AI Gateway.
//! These tests use testcontainers to spin up real PostgreSQL and Redis instances.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use gateway::build_router;
use gateway::config::AppConfig;
use gateway::state::AppState;
use shared::providers::ollama::OllamaProvider;
use shared::providers::Provider;

fn test_metrics_handle() -> metrics_exporter_prometheus::PrometheusHandle {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .build_recorder()
        .handle()
}

/// Helper to create a test app state with a real database.
async fn setup_test_state(database_url: &str, redis_url: Option<&str>) -> AppState {
    use sqlx::postgres::PgPoolOptions;

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to connect to test database");

    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    let redis = if let Some(url) = redis_url {
        match redis::Client::open(url) {
            Ok(client) => redis::aio::ConnectionManager::new(client).await.ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let providers: Vec<Box<dyn Provider>> = vec![Box::new(OllamaProvider::new(Some(
        "http://localhost:11434".to_string(),
    )))];

    let config = AppConfig {
        database_url: database_url.to_string(),
        redis_url: redis_url.unwrap_or("redis://localhost:6379").to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        openai_api_key: None,
        ollama_base_url: Some("http://localhost:11434".to_string()),
        jwt_secret: "test-secret".to_string(),
        global_rate_limit: 1000,
    };

    AppState {
        db,
        redis,
        config: Arc::new(config),
        providers: Arc::new(providers),
    }
}

/// Get DATABASE_URL from env or skip test
fn get_test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/ai_gateway_test".to_string()
        })
    })
}

fn get_test_redis_url() -> Option<String> {
    std::env::var("TEST_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .ok()
}

#[tokio::test]
async fn test_health_endpoint() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;
    let app = build_router(state, test_metrics_handle());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert_eq!(json["db_connected"], true);
}

#[tokio::test]
async fn test_register_user() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;

    // Clean up from previous runs
    let _ = sqlx::query("DELETE FROM users WHERE email = 'test-register@example.com'")
        .execute(&state.db)
        .await;

    let app = build_router(state, test_metrics_handle());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": "test-register@example.com",
                        "password": "securepassword123"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["message"], "User registered successfully");
    assert!(json["user_id"].as_str().is_some());
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;

    // Clean up and create a user
    let _ = sqlx::query("DELETE FROM users WHERE email = 'duplicate@example.com'")
        .execute(&state.db)
        .await;

    let app = build_router(state, test_metrics_handle());

    let body = serde_json::to_string(&serde_json::json!({
        "email": "duplicate@example.com",
        "password": "password123"
    }))
    .unwrap();

    // First registration should succeed
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Second registration should fail with conflict
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_api_key() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;

    // Clean up
    let _ = sqlx::query("DELETE FROM users WHERE email = 'apikey-test@example.com'")
        .execute(&state.db)
        .await;

    let app = build_router(state, test_metrics_handle());

    // Register user first
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": "apikey-test@example.com",
                        "password": "password123"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Create API key
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/keys")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": "apikey-test@example.com",
                        "password": "password123",
                        "key_name": "test-key"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let api_key = json["api_key"].as_str().unwrap();
    assert!(api_key.starts_with("aig_"));
    assert!(api_key.len() > 10);
}

#[tokio::test]
async fn test_chat_completions_requires_auth() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;
    let app = build_router(state, test_metrics_handle());

    // Request without auth header
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "model": "gpt-4",
                        "messages": [{"role": "user", "content": "hi"}]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Request with invalid auth header
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer invalid-key-12345678")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "model": "gpt-4",
                        "messages": [{"role": "user", "content": "hi"}]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_stats_endpoint() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;
    let app = build_router(state, test_metrics_handle());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["total_requests"].is_number());
    assert!(json["total_tokens"].is_number());
    assert!(json["avg_latency_ms"].is_number());
    assert!(json["error_rate"].is_number());
}

#[tokio::test]
async fn test_register_empty_fields() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;
    let app = build_router(state, test_metrics_handle());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": "",
                        "password": ""
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_key_wrong_password() {
    let db_url = get_test_database_url();
    let state = setup_test_state(&db_url, get_test_redis_url().as_deref()).await;

    // Clean up and register
    let _ = sqlx::query("DELETE FROM users WHERE email = 'wrongpw@example.com'")
        .execute(&state.db)
        .await;

    let app = build_router(state, test_metrics_handle());

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": "wrongpw@example.com",
                        "password": "correct-password"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try to create key with wrong password
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/keys")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "email": "wrongpw@example.com",
                        "password": "wrong-password",
                        "key_name": "test"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
