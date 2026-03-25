use clap::Parser;
use console::app::state::AppState;
use console::config::{AppConfig, Cli};
use console::http::router::api_router;
use console::persistence::db::setup_db;
use console::persistence::seed::seed_dev_data;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "console=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Build configuration
    let config = AppConfig::build(cli)?;

    // Setup database
    let db = setup_db(&config.database.url).await?;

    // Seed data in development
    if cfg!(debug_assertions) {
        if let Err(e) = seed_dev_data(&db).await {
            tracing::warn!("Failed to seed dev data: {}", e);
        }
    }

    // Initialize application state
    let state = AppState::new(config.clone(), db);

    let app = axum::Router::new()
        .nest("/api", api_router())
        .fallback(axum::routing::get(
            console::http::handlers::static_files::static_handler,
        ))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Starting workspace server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
