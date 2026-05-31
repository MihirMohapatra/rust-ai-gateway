use axum::{
    extract::State,
    response::Html,
    routing::get,
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod templates;

#[derive(Clone)]
struct DashboardState {
    db: sqlx::PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "dashboard=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ai_gateway".to_string());

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let state = DashboardState { db };

    let app = Router::new()
        .route("/", get(index))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3001";
    tracing::info!("📊 Dashboard listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index(State(state): State<DashboardState>) -> Html<String> {
    let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage_logs")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let total_tokens: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(total_tokens), 0) FROM usage_logs")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let avg_latency: f64 = sqlx::query_scalar("SELECT COALESCE(AVG(latency_ms), 0) FROM usage_logs")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0.0);

    Html(templates::render_dashboard(total_requests, total_tokens, avg_latency))
}

