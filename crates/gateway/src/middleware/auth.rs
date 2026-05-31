use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::AppState;

/// Extracts the API key from the Authorization header and validates it.
/// Inserts the api_key_id and user_id into request extensions for downstream use.
pub async fn auth_middleware(
    state: axum::extract::State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let token = match auth_header {
        Some(ref h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Missing or invalid Authorization header"})),
            )
                .into_response();
        }
    };

    // Look up the API key by its prefix (first 8 chars) then verify the full hash
    let prefix = if token.len() >= 8 {
        &token[..8]
    } else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid API key format"})),
        )
            .into_response();
    };

    let api_key = match find_api_key_by_prefix(&state.db, prefix).await {
        Some(key) => key,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid API key"})),
            )
                .into_response();
        }
    };

    if !api_key.is_active {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "API key is deactivated"})),
        )
            .into_response();
    }

    // Verify full key hash
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let parsed_hash = match PasswordHash::new(&api_key.key_hash) {
        Ok(h) => h,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response();
        }
    };

    if Argon2::default()
        .verify_password(token.as_bytes(), &parsed_hash)
        .is_err()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid API key"})),
        )
            .into_response();
    }

    // Update last_used_at
    let _ = sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(api_key.id)
        .execute(&state.db)
        .await;

    // Insert into extensions
    request.extensions_mut().insert(AuthContext {
        api_key_id: api_key.id,
        user_id: api_key.user_id,
        rate_limit: api_key.rate_limit,
    });

    next.run(request).await
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub api_key_id: Uuid,
    pub user_id: Uuid,
    pub rate_limit: i32,
}

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: Uuid,
    user_id: Uuid,
    key_hash: String,
    is_active: bool,
    rate_limit: i32,
}

async fn find_api_key_by_prefix(pool: &PgPool, prefix: &str) -> Option<ApiKeyRow> {
    sqlx::query_as::<_, ApiKeyRow>(
        "SELECT id, user_id, key_hash, is_active, rate_limit FROM api_keys WHERE prefix = $1",
    )
    .bind(prefix)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}
