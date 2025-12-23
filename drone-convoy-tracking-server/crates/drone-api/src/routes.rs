//! API route definitions

use crate::handlers;
use crate::state::AppState;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    compression::CompressionLayer,
};
use std::time::Duration;

/// Create the main application router
pub fn create_router(state: AppState) -> Router {
    // CORS configuration
    let cors = if state.config.cors_permissive {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
            .max_age(Duration::from_secs(3600))
    } else {
        CorsLayer::new()
            .allow_origin(["http://localhost:8080".parse().unwrap()])
            .allow_methods(Any)
            .allow_headers(Any)
    };

    Router::new()
        // Health & Status
        .route("/health", get(handlers::health_check))
        .route("/ready", get(handlers::readiness_check))
        .route("/status", get(handlers::system_status))
        
        // Metrics (Prometheus format)
        .route("/metrics", get(handlers::metrics))
        
        // Drones API
        .route("/api/v1/drones", get(handlers::list_drones))
        .route("/api/v1/drones/:id", get(handlers::get_drone))
        .route("/api/v1/drones/:id/telemetry", get(handlers::get_drone_telemetry))
        .route("/api/v1/drones/:id/position", get(handlers::get_drone_position))
        .route("/api/v1/drones/:id/command", post(handlers::send_drone_command))
        
        // Mission API
        .route("/api/v1/mission", get(handlers::get_mission))
        .route("/api/v1/mission/start", post(handlers::start_mission))
        .route("/api/v1/mission/pause", post(handlers::pause_mission))
        .route("/api/v1/mission/resume", post(handlers::resume_mission))
        .route("/api/v1/mission/abort", post(handlers::abort_mission))
        .route("/api/v1/mission/waypoints", get(handlers::get_waypoints))
        
        // CV Tracking API
        .route("/api/v1/tracking", get(handlers::get_tracking_results))
        .route("/api/v1/tracking/stats", get(handlers::get_tracking_stats))
        
        // Alerts API
        .route("/api/v1/alerts", get(handlers::list_alerts))
        .route("/api/v1/alerts/:id/acknowledge", post(handlers::acknowledge_alert))
        
        // WebSocket info
        .route("/api/v1/ws/info", get(handlers::websocket_info))
        
        // State snapshot (for frontend initialization)
        .route("/api/v1/state", get(handlers::get_full_state))
        
        // Apply middleware
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .with_state(state)
}
