use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub email: String,
    pub password: String,
    pub key_name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateKeyResponse {
    pub api_key: String,
    pub prefix: String,
    pub message: String,
}

/// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, Json<serde_json::Value>)> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;

    if body.email.is_empty() || body.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Email and password are required"})),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to hash password"})),
            )
        })?
        .to_string();

    let user_id = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, created_at, is_active) VALUES ($1, $2, $3, NOW(), true)",
    )
    .bind(user_id)
    .bind(&body.email)
    .bind(&password_hash)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate") {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "Email already registered"})),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Database error: {}", e)})),
            )
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: user_id.to_string(),
            message: "User registered successfully".to_string(),
        }),
    ))
}

/// POST /auth/keys - Create a new API key
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(body): Json<CreateKeyRequest>,
) -> Result<(StatusCode, Json<CreateKeyResponse>), (StatusCode, Json<serde_json::Value>)> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
    use rand::rngs::OsRng;

    // Authenticate user by email/password
    let user = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, password_hash FROM users WHERE email = $1 AND is_active = true",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Database error: {}", e)})),
        )
    })?;

    let (user_id, stored_hash) = match user {
        Some(u) => u,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid credentials"})),
            ));
        }
    };

    let parsed_hash = PasswordHash::new(&stored_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Internal error"})),
        )
    })?;

    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid credentials"})),
        ));
    }

    // Generate a random API key
    use rand::Rng;
    let raw_key: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();
    let full_key = format!("aig_{}", raw_key);
    let prefix = full_key[..8].to_string();

    // Hash the key for storage
    let salt = SaltString::generate(&mut OsRng);
    let key_hash = Argon2::default()
        .hash_password(full_key.as_bytes(), &salt)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to hash key"})),
            )
        })?
        .to_string();

    let key_id = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT INTO api_keys (id, user_id, key_hash, prefix, name, created_at, is_active, rate_limit) VALUES ($1, $2, $3, $4, $5, NOW(), true, 60)",
    )
    .bind(key_id)
    .bind(user_id)
    .bind(&key_hash)
    .bind(&prefix)
    .bind(&body.key_name)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Database error: {}", e)})),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateKeyResponse {
            api_key: full_key,
            prefix,
            message: "API key created. Store it securely — it cannot be retrieved again.".to_string(),
        }),
    ))
}

