use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::AppConfig;
use shared::providers::Provider;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: Option<ConnectionManager>,
    pub config: Arc<AppConfig>,
    pub providers: Arc<Vec<Box<dyn Provider>>>,
}
