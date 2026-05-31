use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_requests: i64,
    pub total_tokens: i64,
    pub avg_latency_ms: f64,
    pub requests_last_hour: i64,
    pub top_models: Vec<ModelStats>,
    pub error_rate: f64,
}

#[derive(Serialize)]
pub struct ModelStats {
    pub model: String,
    pub request_count: i64,
    pub total_tokens: i64,
}

/// GET /api/stats
pub async fn get_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let totals = sqlx::query_as::<_, (i64, i64, f64)>(
        "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0), COALESCE(AVG(latency_ms), 0) FROM usage_logs"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0, 0.0));

    let last_hour = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM usage_logs WHERE created_at > NOW() - INTERVAL '1 hour'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let errors =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM usage_logs WHERE status_code >= 400")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let error_rate = if totals.0 > 0 {
        errors as f64 / totals.0 as f64
    } else {
        0.0
    };

    let top_models = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT model, COUNT(*) as cnt, COALESCE(SUM(total_tokens), 0) FROM usage_logs GROUP BY model ORDER BY cnt DESC LIMIT 5"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(model, request_count, total_tokens)| ModelStats {
        model,
        request_count,
        total_tokens,
    })
    .collect();

    Json(StatsResponse {
        total_requests: totals.0,
        total_tokens: totals.1,
        avg_latency_ms: totals.2,
        requests_last_hour: last_hour,
        top_models,
        error_rate,
    })
}
