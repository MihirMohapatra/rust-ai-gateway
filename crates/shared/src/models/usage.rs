use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageLog {
    pub id: Uuid,
    pub api_key_id: Uuid,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub latency_ms: i64,
    pub status_code: i16,
    pub created_at: DateTime<Utc>,
}

