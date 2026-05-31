use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use gateway::{build_router, config::AppConfig, create_state};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Structured logging: JSON in production, pretty in dev
    let is_production = std::env::var("ENVIRONMENT").unwrap_or_default() == "production";

    if is_production {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "gateway=info,tower_http=info".into()),
            )
            .with(tracing_subscriber::fmt::layer().json().with_target(true))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "gateway=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer().pretty())
            .init();
    }

    let config = AppConfig::from_env();

    // Setup Prometheus metrics
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    let state = create_state(config.clone()).await?;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %state.config.host,
        port = %state.config.port,
        "Database connected and migrations applied"
    );

    let app = build_router(state.clone(), handle);

    let addr = format!("{}:{}", state.config.host, state.config.port);
    tracing::info!(address = %addr, "🚀 AI Gateway starting");

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("👋 AI Gateway shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Received Ctrl+C, shutting down..."); },
        _ = terminate => { tracing::info!("Received SIGTERM, shutting down..."); },
    }
}
