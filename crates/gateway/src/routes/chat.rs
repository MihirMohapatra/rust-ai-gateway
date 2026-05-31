use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::Utc;
use uuid::Uuid;

use shared::models::chat::{ChatRequest, ChatResponse};

use crate::middleware::auth::AuthContext;
use crate::state::AppState;

/// POST /v1/chat/completions
pub async fn chat_completions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<serde_json::Value>)> {
    let start = std::time::Instant::now();
    let request_model = request.model.clone();

    tracing::info!(
        model = %request_model,
        user_id = %auth.user_id,
        message_count = request.messages.len(),
        "Processing chat completion request"
    );

    // Select provider based on model
    let provider = state
        .providers
        .iter()
        .find(|p| p.supports_model(&request.model))
        .ok_or_else(|| {
            tracing::warn!(model = %request.model, "No provider found for model");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("No provider available for model: {}", request.model)
                })),
            )
        })?;

    // Call the provider
    let result = provider.chat_completion(&request).await;

    let latency_ms = start.elapsed().as_millis() as i64;

    // Log usage regardless of success/failure
    let (status_code, prompt_tokens, completion_tokens, total_tokens) = match &result {
        Ok(resp) => {
            let usage = resp.usage.as_ref();
            (
                200i16,
                usage.map(|u| u.prompt_tokens as i32).unwrap_or(0),
                usage.map(|u| u.completion_tokens as i32).unwrap_or(0),
                usage.map(|u| u.total_tokens as i32).unwrap_or(0),
            )
        }
        Err(e) => (e.status_code() as i16, 0, 0, 0),
    };

    tracing::info!(
        model = %request_model,
        provider = %provider.name(),
        latency_ms = latency_ms,
        status = status_code,
        prompt_tokens = prompt_tokens,
        completion_tokens = completion_tokens,
        total_tokens = total_tokens,
        "Chat completion completed"
    );

    // Insert usage log (fire and forget)
    let db = state.db.clone();
    let model = request.model.clone();
    let provider_name = provider.name().to_string();
    let api_key_id = auth.api_key_id;

    tokio::spawn(async move {
        if let Err(e) = sqlx::query(
            "INSERT INTO usage_logs (id, api_key_id, model, provider, prompt_tokens, completion_tokens, total_tokens, latency_ms, status_code, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(Uuid::new_v4())
        .bind(api_key_id)
        .bind(&model)
        .bind(&provider_name)
        .bind(prompt_tokens)
        .bind(completion_tokens)
        .bind(total_tokens)
        .bind(latency_ms)
        .bind(status_code)
        .bind(Utc::now())
        .execute(&db)
        .await {
            tracing::error!(error = %e, "Failed to insert usage log");
        }
    });

    // Update metrics
    metrics::counter!("gateway_requests_total", "model" => request.model.clone(), "provider" => provider.name().to_string(), "status" => status_code.to_string()).increment(1);
    metrics::histogram!("gateway_request_duration_ms", "model" => request.model.clone())
        .record(latency_ms as f64);
    metrics::gauge!("gateway_tokens_total", "model" => request.model.clone())
        .set(total_tokens as f64);

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((
            StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}
