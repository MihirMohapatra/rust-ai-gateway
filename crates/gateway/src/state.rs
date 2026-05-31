use std::sync::Arc;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use shared::providers::Provider;
use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: Option<ConnectionManager>,
    pub config: Arc<AppConfig>,
    pub providers: Arc<Vec<Box<dyn Provider>>>,
}

