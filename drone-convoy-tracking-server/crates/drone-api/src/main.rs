//! # Drone API Server
//!
//! Main entry point for the Drone Convoy Tracking System.
//! Provides REST API endpoints for drone management and coordinates
//! all backend services including WebSocket, CV tracking, and database.

mod config;
mod error;
mod handlers;
mod routes;
mod state;

use crate::config::ApiConfig;
use crate::routes::create_router;
use crate::state::AppState;

use std::net::SocketAddr;
use tokio::signal;
use tracing::{info, error, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use drone_core::{DroneId, GeoPosition, Telemetry, Event};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    init_logging();

    info!("🚁 Starting Drone Convoy Tracking Server v0.1.0");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Load configuration
    let config = ApiConfig::from_env();
    info!("Configuration loaded");
    info!("   API Port: {}", config.api_port);
    info!("   WebSocket Port: {}", config.ws_port);
    info!("   ScyllaDB Hosts: {:?}", config.db.hosts);

    // Initialize application state
    info!("Initializing application state...");
    let state = match AppState::new(config.clone()).await {
        Ok(state) => {
            info!("Application state initialized");
            state
        }
        Err(e) => {
            error!("Failed to initialize application state: {}", e);
            // Continue without database for development
            info!("Running in degraded mode (no database)");
            AppState::new_without_db(config.clone()).await?
        }
    };

    // Create router
    let app = create_router(state.clone());
    info!("Routes configured");

    // Start WebSocket server in background
    let ws_state = state.clone();
    let ws_port = config.ws_port;
    tokio::spawn(async move {
        info!("Starting WebSocket server on port {}...", ws_port);
        if let Err(e) = drone_websocket::start_server(ws_state.ws_hub.clone(), ws_port).await {
            error!("WebSocket server error: {}", e);
        }
    });

    // Start simulation task (generates fake drone data for PoC)
    let sim_state = state.clone();
    tokio::spawn(async move {
        info!("Starting drone simulation...");
        run_simulation(sim_state).await;
    });

    // Start API server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.api_port));
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🚀 API server listening on http://{}", addr);
    info!("WebSocket server on ws://0.0.0.0:{}", config.ws_port);
    info!("Metrics available at http://{}/metrics", addr);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // axum::serve(listener, app)
    //     .with_graceful_shutdown(shutdown_signal())
    //     .await?;

    axum::serve(listener, app.into_make_service())
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    info!("🛑 Server shutdown complete");
    Ok(())
}

/// Initialize logging with tracing
fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("info,drone_api=debug,drone_websocket=debug")
            //EnvFilter::new("info,drone_api=debug,drone_cv=debug,drone_websocket=debug")
        });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();
}

/// Graceful shutdown handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down...");
        }
    }
}

/// Run drone simulation for demo purposes
async fn run_simulation(state: AppState) {
    use drone_core::{
        Drone, DroneId, DroneStatus, GeoPosition, Telemetry, Waypoint,
        Event, EventType, EventPayload, DronePositionEvent,
    };
    use chrono::Utc;
    use std::time::Duration;

    // Afghanistan waypoints (same as frontend)
    let waypoints = vec![
        ("Base Alpha", 34.5553, 69.2075),
        ("Checkpoint Bravo", 34.6234, 69.1123),
        ("Outpost Charlie", 34.7012, 69.0456),
        ("Firebase Delta", 34.7891, 68.9234),
        ("Sector Echo", 34.8567, 68.8012),
        ("Point Foxtrot", 34.9234, 68.6789),
        ("Zone Golf", 34.9901, 68.5567),
        ("Camp Hotel", 35.0567, 68.4234),
        ("Station India", 35.1234, 68.3012),
        ("Forward Juliet", 35.1901, 68.1789),
        ("Base Kilo", 35.2567, 68.0567),
        ("Terminal Lima", 35.3234, 67.9234),
    ];

    // Initialize 12 drones
    let mut drones: Vec<SimDrone> = (1..=12)
        .map(|i| SimDrone {
            id: DroneId::new(format!("REAPER-{:02}", i)),
            waypoint_index: 0,
            progress: 0.0,
            speed: 0.8 + (i as f64 * 0.02), // Slight speed variation
            battery: 100,
            fuel: 100,
        })
        .collect();

    let mut interval = tokio::time::interval(Duration::from_millis(500));
    let speed_multiplier = 0.001; // Adjust for demo speed

    loop {
        interval.tick().await;

        for drone in &mut drones {
            // Update progress
            drone.progress += speed_multiplier * drone.speed;

            // Check waypoint transition
            if drone.progress >= 1.0 {
                drone.progress = 0.0;
                drone.waypoint_index = (drone.waypoint_index + 1) % waypoints.len();
            }

            // Interpolate position between waypoints
            let current_wp = &waypoints[drone.waypoint_index];
            let next_wp = &waypoints[(drone.waypoint_index + 1) % waypoints.len()];

            let lat = current_wp.1 + (next_wp.1 - current_wp.1) * drone.progress;
            let lng = current_wp.2 + (next_wp.2 - current_wp.2) * drone.progress;
            let alt = 3000.0 + (drone.id.0.chars().last().unwrap().to_digit(10).unwrap_or(0) as f64 * 100.0);

            // Calculate heading
            let heading = calculate_bearing(current_wp.1, current_wp.2, next_wp.1, next_wp.2);

            // Drain battery/fuel slowly
            drone.battery = (drone.battery as f64 - 0.001).max(20.0) as u8;
            drone.fuel = (drone.fuel as f64 - 0.002).max(15.0) as u8;

            // Create position update
            let position = GeoPosition::new(lat, lng, alt);
            let telemetry = Telemetry {
                battery_level: drone.battery,
                fuel_level: drone.fuel,
                system_health: 95 + (drone.id.0.len() % 5) as u8,
                speed: 350.0 + (drone.speed * 50.0),
                heading,
                signal_strength: 90 + (drone.waypoint_index % 10) as u8,
                temperature: 42.0,
                timestamp: Utc::now(),
            };

            // Broadcast via WebSocket
            let event = Event::drone_position_updated(
                drone.id.clone(),
                position,
                telemetry,
            );

            state.ws_hub.broadcast(event).await;
        }
    }
}

/// Simple simulation drone state
struct SimDrone {
    id: DroneId,
    waypoint_index: usize,
    progress: f64,
    speed: f64,
    battery: u8,
    fuel: u8,
}

/// Calculate bearing between two coordinates
fn calculate_bearing(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();
    let delta_lng = (lng2 - lng1).to_radians();

    let y = delta_lng.sin() * lat2.cos();
    let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * delta_lng.cos();

    let bearing = y.atan2(x).to_degrees();
    (bearing + 360.0) % 360.0
}
