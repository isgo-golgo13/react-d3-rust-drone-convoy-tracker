//! API request handlers

use crate::error::ApiError;
use crate::state::AppState;

use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use drone_core::{
    Drone, DroneId, DroneStatus, GeoPosition, Mission, MissionStatus,
    Telemetry, TrackingResult, Alert, AlertSeverity, AlertType,
    DroneCommand, DroneCommandType,
};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use tracing::{info, debug};

// ============================================================================
// RESPONSE TYPES
// ============================================================================

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub api: String,
    pub database: String,
    pub cv_engine: String,
    pub websocket_clients: usize,
    pub active_drones: usize,
    pub mission_status: String,
}

#[derive(Serialize)]
pub struct DroneListResponse {
    pub drones: Vec<DroneResponse>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct DroneResponse {
    pub id: String,
    pub callsign: String,
    pub status: String,
    pub position: PositionResponse,
    pub telemetry: TelemetryResponse,
    pub armed: bool,
    pub current_waypoint: usize,
}

#[derive(Serialize)]
pub struct PositionResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

#[derive(Serialize)]
pub struct TelemetryResponse {
    pub battery_level: u8,
    pub fuel_level: u8,
    pub system_health: u8,
    pub speed: f64,
    pub heading: f64,
    pub signal_strength: u8,
}

#[derive(Serialize)]
pub struct MissionResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub waypoint_count: usize,
    pub assigned_drones: usize,
    pub total_distance_km: f64,
}

#[derive(Serialize)]
pub struct WaypointResponse {
    pub id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub waypoint_type: String,
}

#[derive(Serialize)]
pub struct WebSocketInfoResponse {
    pub url: String,
    pub connected_clients: usize,
    pub supported_events: Vec<String>,
}

#[derive(Serialize)]
pub struct FullStateResponse {
    pub drones: Vec<DroneResponse>,
    pub mission: Option<MissionResponse>,
    pub waypoints: Vec<WaypointResponse>,
}

#[derive(Serialize)]
pub struct TrackingStatsResponse {
    pub active_tracks: usize,
    pub cv_enabled: bool,
    pub frames_processed: u64,
}

#[derive(Serialize)]
pub struct AlertResponse {
    pub id: String,
    pub severity: String,
    pub alert_type: String,
    pub message: String,
    pub drone_id: Option<String>,
    pub acknowledged: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub command: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

// ============================================================================
// HEALTH & STATUS HANDLERS
// ============================================================================

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        timestamp: Utc::now().to_rfc3339(),
    })
}

/// Readiness check (for Kubernetes)
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ready = if let Some(db) = &state.db {
        db.health_check().await.unwrap_or(false)
    } else {
        true // No DB required
    };

    if db_ready {
        (StatusCode::OK, Json(serde_json::json!({"ready": true})))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"ready": false})))
    }
}

/// System status overview
pub async fn system_status(State(state): State<AppState>) -> impl IntoResponse {
    let mission_status = state.get_mission()
        .map(|m| format!("{:?}", m.status))
        .unwrap_or_else(|| "NONE".into());

    Json(StatusResponse {
        api: "running".into(),
        database: if state.has_db() { "connected" } else { "unavailable" }.into(),
        //cv_engine: if state.has_cv() { "active" } else { "disabled" }.into(),
        cv_engine: "disabled".into(),
        websocket_clients: state.ws_client_count(),
        active_drones: state.drones.len(),
        mission_status,
    })
}

/// Prometheus metrics endpoint
pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = format!(
        r#"# HELP drone_convoy_drones_total Total number of drones
# TYPE drone_convoy_drones_total gauge
drone_convoy_drones_total {}

# HELP drone_convoy_websocket_clients Connected WebSocket clients
# TYPE drone_convoy_websocket_clients gauge
drone_convoy_websocket_clients {}

# HELP drone_convoy_cv_enabled CV tracking enabled
# TYPE drone_convoy_cv_enabled gauge
drone_convoy_cv_enabled {}

# HELP drone_convoy_db_connected Database connection status
# TYPE drone_convoy_db_connected gauge
drone_convoy_db_connected {}
"#,
        state.drones.len(),
        state.ws_client_count(),
        //if state.has_cv() { 1 } else { 0 },
        0,
        if state.has_db() { 1 } else { 0 },
    );

    (StatusCode::OK, [("content-type", "text/plain")], metrics)
}

// ============================================================================
// DRONE HANDLERS
// ============================================================================

/// List all drones
pub async fn list_drones(State(state): State<AppState>) -> impl IntoResponse {
    let drones: Vec<DroneResponse> = state.get_all_drones()
        .into_iter()
        .map(drone_to_response)
        .collect();

    let total = drones.len();
    Json(DroneListResponse { drones, total })
}

/// Get single drone by ID
pub async fn get_drone(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let drone_id = DroneId::new(&id);
    
    state.get_drone(&drone_id)
        .map(|d| Json(drone_to_response(d)))
        .ok_or_else(|| ApiError::not_found(format!("Drone {} not found", id)))
}

/// Get drone telemetry
pub async fn get_drone_telemetry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let drone_id = DroneId::new(&id);
    
    state.get_drone(&drone_id)
        .map(|d| Json(TelemetryResponse {
            battery_level: d.telemetry.battery_level,
            fuel_level: d.telemetry.fuel_level,
            system_health: d.telemetry.system_health,
            speed: d.telemetry.speed,
            heading: d.telemetry.heading,
            signal_strength: d.telemetry.signal_strength,
        }))
        .ok_or_else(|| ApiError::not_found(format!("Drone {} not found", id)))
}

/// Get drone position
pub async fn get_drone_position(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let drone_id = DroneId::new(&id);
    
    state.get_drone(&drone_id)
        .map(|d| Json(PositionResponse {
            latitude: d.position.latitude,
            longitude: d.position.longitude,
            altitude: d.position.altitude,
        }))
        .ok_or_else(|| ApiError::not_found(format!("Drone {} not found", id)))
}

/// Send command to drone
pub async fn send_drone_command(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CommandRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let drone_id = DroneId::new(&id);
    
    if state.get_drone(&drone_id).is_none() {
        return Err(ApiError::not_found(format!("Drone {} not found", id)));
    }

    info!("Command {} sent to drone {}", req.command, id);

    // In real implementation, this would send command via P2P or queue
    Ok(Json(serde_json::json!({
        "status": "accepted",
        "drone_id": id,
        "command": req.command,
    })))
}

// ============================================================================
// MISSION HANDLERS
// ============================================================================

/// Get current mission
pub async fn get_mission(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_mission() {
        Some(mission) => Json(Some(mission_to_response(&mission))),
        None => Json(None),
    }
}

/// Start mission
pub async fn start_mission(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(mut mission) = state.active_mission.write().take() {
        mission.start();
        *state.active_mission.write() = Some(mission.clone());
        info!("Mission {} started", mission.name);
        Json(serde_json::json!({"status": "started", "mission": mission.name}))
    } else {
        Json(serde_json::json!({"status": "error", "message": "No active mission"}))
    }
}

/// Pause mission
pub async fn pause_mission(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(mission) = state.active_mission.read().as_ref() {
        info!("Mission {} paused", mission.name);
    }
    Json(serde_json::json!({"status": "paused"}))
}

/// Resume mission
pub async fn resume_mission(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(mission) = state.active_mission.read().as_ref() {
        info!("Mission {} resumed", mission.name);
    }
    Json(serde_json::json!({"status": "resumed"}))
}

/// Abort mission
pub async fn abort_mission(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(mission) = state.active_mission.read().as_ref() {
        info!("Mission {} aborted", mission.name);
    }
    Json(serde_json::json!({"status": "aborted"}))
}

/// Get mission waypoints
pub async fn get_waypoints(State(state): State<AppState>) -> impl IntoResponse {
    let waypoints: Vec<WaypointResponse> = state.get_mission()
        .map(|m| m.waypoints.iter().map(|wp| WaypointResponse {
            id: wp.id.0.clone(),
            name: wp.name.clone(),
            latitude: wp.position.latitude,
            longitude: wp.position.longitude,
            waypoint_type: format!("{:?}", wp.waypoint_type),
        }).collect())
        .unwrap_or_default();

    Json(waypoints)
}

// ============================================================================
// TRACKING HANDLERS
// ============================================================================

/// Get CV tracking results
pub async fn get_tracking_results(State(state): State<AppState>) -> impl IntoResponse {
    // In real implementation, would return actual CV tracking data
    Json(serde_json::json!({
        "tracks": [],
        "frame_timestamp": Utc::now().to_rfc3339(),
    }))
}

/// Get tracking statistics
// pub async fn get_tracking_stats(State(state): State<AppState>) -> impl IntoResponse {
//     let active_tracks = state.cv_engine
//         .as_ref()
//         .map(|e| e.read().active_track_count())
//         .unwrap_or(0);

//     Json(TrackingStatsResponse {
//         active_tracks,
//         cv_enabled: state.has_cv(),
//         frames_processed: 0,
//     })
// }
/// Get tracking statistics
pub async fn get_tracking_stats(State(state): State<AppState>) -> impl IntoResponse {
    // CV disabled for macOS build
    let active_tracks = 0;

    Json(TrackingStatsResponse {
        active_tracks,
        cv_enabled: false,
        frames_processed: 0,
    })
}


// ============================================================================
// ALERT HANDLERS
// ============================================================================

/// List alerts
pub async fn list_alerts(State(_state): State<AppState>) -> impl IntoResponse {
    // Demo alerts
    let alerts = vec![
        AlertResponse {
            id: "alert-001".into(),
            severity: "INFO".into(),
            alert_type: "SYSTEM".into(),
            message: "System initialized successfully".into(),
            drone_id: None,
            acknowledged: true,
            created_at: Utc::now().to_rfc3339(),
        },
    ];

    Json(alerts)
}

/// Acknowledge alert
pub async fn acknowledge_alert(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    info!("Alert {} acknowledged", id);
    Json(serde_json::json!({"status": "acknowledged", "alert_id": id}))
}

// ============================================================================
// WEBSOCKET HANDLERS
// ============================================================================

/// WebSocket connection info
pub async fn websocket_info(State(state): State<AppState>) -> impl IntoResponse {
    Json(WebSocketInfoResponse {
        url: format!("ws://localhost:{}", state.config.ws_port),
        connected_clients: state.ws_client_count(),
        supported_events: vec![
            "DRONE_POSITION_UPDATED".into(),
            "DRONE_STATUS_CHANGED".into(),
            "WAYPOINT_REACHED".into(),
            "CV_TRACKING_UPDATE".into(),
            "ALERT_RAISED".into(),
        ],
    })
}

// ============================================================================
// STATE HANDLERS
// ============================================================================

/// Get full state snapshot for frontend initialization
pub async fn get_full_state(State(state): State<AppState>) -> impl IntoResponse {
    let drones: Vec<DroneResponse> = state.get_all_drones()
        .into_iter()
        .map(drone_to_response)
        .collect();

    let mission = state.get_mission().map(|m| mission_to_response(&m));
    
    let waypoints: Vec<WaypointResponse> = state.get_mission()
        .map(|m| m.waypoints.iter().map(|wp| WaypointResponse {
            id: wp.id.0.clone(),
            name: wp.name.clone(),
            latitude: wp.position.latitude,
            longitude: wp.position.longitude,
            waypoint_type: format!("{:?}", wp.waypoint_type),
        }).collect())
        .unwrap_or_default();

    Json(FullStateResponse {
        drones,
        mission,
        waypoints,
    })
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn drone_to_response(drone: Drone) -> DroneResponse {
    DroneResponse {
        id: drone.id.0,
        callsign: drone.callsign,
        status: format!("{}", drone.status),
        position: PositionResponse {
            latitude: drone.position.latitude,
            longitude: drone.position.longitude,
            altitude: drone.position.altitude,
        },
        telemetry: TelemetryResponse {
            battery_level: drone.telemetry.battery_level,
            fuel_level: drone.telemetry.fuel_level,
            system_health: drone.telemetry.system_health,
            speed: drone.telemetry.speed,
            heading: drone.telemetry.heading,
            signal_strength: drone.telemetry.signal_strength,
        },
        armed: drone.armed,
        current_waypoint: drone.current_waypoint_index,
    }
}

fn mission_to_response(mission: &Mission) -> MissionResponse {
    MissionResponse {
        id: mission.id.0.to_string(),
        name: mission.name.clone(),
        status: format!("{:?}", mission.status),
        waypoint_count: mission.waypoints.len(),
        assigned_drones: mission.assigned_drones.len(),
        total_distance_km: mission.total_distance_km(),
    }
}
