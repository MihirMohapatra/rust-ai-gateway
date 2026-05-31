use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub db_connected: bool,
    pub redis_connected: bool,
    pub db_pool_size: u32,
    pub db_pool_idle: u32,
}

/// GET /health - Full health check with dependency status
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();

    let redis_ok = match state.redis.clone() {
        Some(mut conn) => {
            let result: Result<String, _> = redis::cmd("PING").query_async(&mut conn).await;
            result.is_ok()
        }
        None => false,
    };

    let pool_size = state.db.size();
    let pool_idle = state.db.num_idle() as u32;

    Json(HealthResponse {
        status: if db_ok {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        db_connected: db_ok,
        redis_connected: redis_ok,
        db_pool_size: pool_size,
        db_pool_idle: pool_idle,
    })
}

/// GET /health/ready - Readiness probe (for Kubernetes/ECS)
/// Returns 200 only if all critical dependencies are available
pub async fn readiness_check(State(state): State<AppState>) -> StatusCode {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();

    if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

/// GET /health/live - Liveness probe
/// Returns 200 if the process is alive (always succeeds)
pub async fn liveness_check() -> StatusCode {
    StatusCode::OK
}
