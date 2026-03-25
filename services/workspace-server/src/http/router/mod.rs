use crate::app::state::AppState;
use crate::http::handlers;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

pub fn api_router() -> Router<AppState> {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .nest("/auth", handlers::auth::router())
        .merge(handlers::workspaces::router())
        .merge(handlers::members::router())
        .merge(handlers::credentials::router())
        .merge(handlers::sync::router())
        .merge(handlers::users::router())
        .layer(cors)
}

pub fn health_router() -> Router<AppState> {
    handlers::health::router()
}
