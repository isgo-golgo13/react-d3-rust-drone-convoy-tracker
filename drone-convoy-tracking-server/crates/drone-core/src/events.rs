//! Event types for the drone convoy system
//! 
//! These events are used for real-time communication via WebSocket
//! and for recording in the database.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Alert, BoundingBox, DetectedHalo, Drone, DroneId, DroneStatus, GeoPosition, 
    Mission, MissionId, MissionStatus, Telemetry, TrackingResult, WaypointId,
};

/// Event envelope for all system events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: EventPayload,
}

impl Event {
    pub fn new(event_type: EventType, payload: EventPayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            payload,
        }
    }

    pub fn drone_position_updated(drone_id: DroneId, position: GeoPosition, telemetry: Telemetry) -> Self {
        Self::new(
            EventType::DronePositionUpdated,
            EventPayload::DronePosition(DronePositionEvent {
                drone_id,
                position,
                telemetry,
            }),
        )
    }

    pub fn drone_status_changed(drone_id: DroneId, old_status: DroneStatus, new_status: DroneStatus) -> Self {
        Self::new(
            EventType::DroneStatusChanged,
            EventPayload::DroneStatus(DroneStatusEvent {
                drone_id,
                old_status,
                new_status,
            }),
        )
    }

    pub fn waypoint_reached(drone_id: DroneId, waypoint_id: WaypointId, position: GeoPosition) -> Self {
        Self::new(
            EventType::WaypointReached,
            EventPayload::Waypoint(WaypointEvent {
                drone_id,
                waypoint_id,
                position,
                event_type: WaypointEventType::Arrived,
            }),
        )
    }

    pub fn cv_tracking_update(result: TrackingResult) -> Self {
        Self::new(
            EventType::CvTrackingUpdate,
            EventPayload::CvTracking(CvTrackingEvent {
                results: vec![result],
            }),
        )
    }

    pub fn alert(alert: Alert) -> Self {
        Self::new(
            EventType::AlertRaised,
            EventPayload::Alert(AlertEvent { alert }),
        )
    }
}

/// Type of event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    // Drone events
    DronePositionUpdated,
    DroneStatusChanged,
    DroneTelemetryUpdated,
    DroneConnected,
    DroneDisconnected,
    
    // Mission events
    MissionStarted,
    MissionCompleted,
    MissionPaused,
    MissionAborted,
    
    // Waypoint events
    WaypointReached,
    WaypointDeparted,
    
    // CV tracking events
    CvTrackingUpdate,
    HaloDetected,
    TrackingLost,
    
    // Alert events
    AlertRaised,
    AlertAcknowledged,
    AlertResolved,
    
    // System events
    SystemHealthUpdate,
    ConnectionEstablished,
    ConnectionLost,
}

/// Event payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    DronePosition(DronePositionEvent),
    DroneStatus(DroneStatusEvent),
    DroneTelemetry(DroneTelemetryEvent),
    DroneConnection(DroneConnectionEvent),
    Mission(MissionEvent),
    Waypoint(WaypointEvent),
    CvTracking(CvTrackingEvent),
    Alert(AlertEvent),
    System(SystemEvent),
    FullState(FullStateEvent),
}

/// Drone position update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DronePositionEvent {
    pub drone_id: DroneId,
    pub position: GeoPosition,
    pub telemetry: Telemetry,
}

/// Drone status change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneStatusEvent {
    pub drone_id: DroneId,
    pub old_status: DroneStatus,
    pub new_status: DroneStatus,
}

/// Drone telemetry update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneTelemetryEvent {
    pub drone_id: DroneId,
    pub telemetry: Telemetry,
}

/// Drone connection event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneConnectionEvent {
    pub drone_id: DroneId,
    pub connected: bool,
    pub peer_id: Option<String>,
}

/// Mission event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionEvent {
    pub mission_id: MissionId,
    pub status: MissionStatus,
    pub message: Option<String>,
}

/// Waypoint event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WaypointEventType {
    Arrived,
    Departed,
    Flyover,
    Skipped,
}

/// Waypoint passage event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointEvent {
    pub drone_id: DroneId,
    pub waypoint_id: WaypointId,
    pub position: GeoPosition,
    pub event_type: WaypointEventType,
}

/// Computer vision tracking event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvTrackingEvent {
    pub results: Vec<TrackingResult>,
}

/// Alert event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub alert: Alert,
}

/// System health event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub component: String,
    pub status: String,
    pub message: Option<String>,
}

/// Full state snapshot event (sent on initial connection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullStateEvent {
    pub drones: Vec<Drone>,
    pub mission: Option<Mission>,
    pub tracking_results: Vec<TrackingResult>,
}

// ============================================================================
// WEBSOCKET MESSAGE TYPES
// ============================================================================

/// Message sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Initial state on connection
    InitialState(FullStateEvent),
    /// Event update
    Event(Event),
    /// Batch of events
    EventBatch(Vec<Event>),
    /// Error message
    Error { code: String, message: String },
    /// Heartbeat/ping
    Ping { timestamp: i64 },
}

/// Message sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// Subscribe to specific drone updates
    Subscribe { drone_ids: Option<Vec<DroneId>> },
    /// Unsubscribe from updates
    Unsubscribe { drone_ids: Option<Vec<DroneId>> },
    /// Request current state
    RequestState,
    /// Send command to drone
    DroneCommand(DroneCommand),
    /// Heartbeat/pong
    Pong { timestamp: i64 },
}

/// Command sent to a drone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneCommand {
    pub drone_id: DroneId,
    pub command: DroneCommandType,
}

/// Type of drone command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum DroneCommandType {
    /// Start mission
    Start,
    /// Pause current operations
    Pause,
    /// Resume operations
    Resume,
    /// Return to base
    ReturnToBase,
    /// Emergency stop
    EmergencyStop,
    /// Go to specific waypoint
    GoToWaypoint { waypoint_id: WaypointId },
    /// Set speed
    SetSpeed { speed: f64 },
    /// Arm/disarm weapons
    SetArmed { armed: bool },
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::drone_status_changed(
            DroneId::new("REAPER-01"),
            DroneStatus::Standby,
            DroneStatus::Moving,
        );
        
        assert_eq!(event.event_type, EventType::DroneStatusChanged);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::drone_position_updated(
            DroneId::new("REAPER-01"),
            GeoPosition::new(34.5553, 69.2075, 1000.0),
            Telemetry::default(),
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.event_type, EventType::DronePositionUpdated);
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Ping { timestamp: 12345 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Ping"));
    }
}
