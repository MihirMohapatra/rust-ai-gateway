pub mod config;
pub mod middleware;
pub mod routes;
pub mod state;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use metrics_exporter_prometheus::PrometheusHandle;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use shared::providers::ollama::OllamaProvider;
use shared::providers::openai::OpenAiProvider;
use shared::providers::Provider;

use crate::config::AppConfig;
use crate::state::AppState;

/// Build the application router with the given state.
pub fn build_router(state: AppState, metrics_handle: PrometheusHandle) -> Router {
    // Protected routes (with auth + rate limiting)
    let protected = Router::new()
        .route("/v1/chat/completions", post(routes::chat::chat_completions))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::rate_limit_middleware,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::auth_middleware,
        ));

    // Public routes (no auth required)
    let public = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/health/ready", get(routes::health::readiness_check))
        .route("/health/live", get(routes::health::liveness_check))
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/keys", post(routes::auth::create_api_key))
        .route("/api/stats", get(routes::stats::get_stats))
        .route(
            "/metrics",
            get(move || std::future::ready(metrics_handle.render())),
        );

    // Merge and apply global middleware
    public
        .merge(protected)
        .layer(axum_middleware::from_fn(
            middleware::request_id::request_id_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(Duration::from_secs(60)))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Create AppState from config, connecting to DB and Redis.
pub async fn create_state(config: AppConfig) -> anyhow::Result<AppState> {
    use sqlx::postgres::PgPoolOptions;

    let db = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&db).await?;

    let redis = match redis::Client::open(config.redis_url.as_str()) {
        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
            Ok(conn) => Some(conn),
            Err(e) => {
                tracing::warn!(error = %e, "Redis connection failed");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "Invalid Redis URL");
            None
        }
    };

    let mut providers: Vec<Box<dyn Provider>> = Vec::new();

    if let Some(ref api_key) = config.openai_api_key {
        providers.push(Box::new(OpenAiProvider::new(api_key.clone(), None)));
        tracing::info!("OpenAI provider enabled");
    }

    providers.push(Box::new(OllamaProvider::new(
        config.ollama_base_url.clone(),
    )));
    tracing::info!("Ollama provider enabled");

    Ok(AppState {
        db,
        redis,
        config: Arc::new(config),
        providers: Arc::new(providers),
    })
}
