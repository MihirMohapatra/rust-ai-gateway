use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::middleware::auth::AuthContext;
use crate::state::AppState;

/// Sliding window rate limiter using Redis.
/// Checks per-key rate limit stored in AuthContext.
pub async fn rate_limit_middleware(
    state: axum::extract::State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let auth_ctx = match request.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => return next.run(request).await, // No auth context = skip rate limiting
    };

    let key = format!("rate_limit:{}", auth_ctx.api_key_id);
    let limit = auth_ctx.rate_limit as u64;
    let window_secs: u64 = 60;

    // Try Redis, fall back gracefully if unavailable
    let mut redis_conn = match state.redis.clone() {
        Some(conn) => conn,
        None => return next.run(request).await,
    };

    let now = chrono::Utc::now().timestamp() as u64;
    let window_start = now - window_secs;

    // Remove old entries, count current, add new entry
    let result: Result<u64, _> = redis::pipe()
        .atomic()
        .cmd("ZREMRANGEBYSCORE")
        .arg(&key)
        .arg(0u64)
        .arg(window_start)
        .ignore()
        .cmd("ZCARD")
        .arg(&key)
        .cmd("ZADD")
        .arg(&key)
        .arg(now)
        .arg(format!("{}:{}", now, rand::random::<u32>()))
        .ignore()
        .cmd("EXPIRE")
        .arg(&key)
        .arg(window_secs + 1)
        .ignore()
        .query_async(&mut redis_conn)
        .await;

    match result {
        Ok(count) if count >= limit => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "Rate limit exceeded",
                    "limit": limit,
                    "window_seconds": window_secs,
                })),
            )
                .into_response();
        }
        Err(_) => {
            // Redis unavailable, allow request
            tracing::warn!("Redis unavailable for rate limiting, allowing request");
        }
        _ => {}
    }

    next.run(request).await
}

