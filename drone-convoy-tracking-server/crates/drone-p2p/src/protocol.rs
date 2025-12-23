//! P2P message protocol definitions

use drone_core::{DroneId, DroneStatus, GeoPosition, Telemetry};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message types in the P2P network
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageType {
    /// Heartbeat/keepalive
    Heartbeat,
    /// Position update broadcast
    PositionUpdate(PositionUpdateData),
    /// Status change notification
    StatusChange(StatusChangeData),
    /// Formation command
    FormationCommand(FormationCommandData),
    /// Emergency broadcast
    Emergency(EmergencyData),
    /// Acknowledgment
    Ack(AckData),
    /// Discovery request
    DiscoveryRequest,
    /// Discovery response
    DiscoveryResponse(DiscoveryResponseData),
}

/// Position update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdateData {
    pub drone_id: DroneId,
    pub position: GeoPosition,
    pub telemetry: Telemetry,
}

/// Status change data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusChangeData {
    pub drone_id: DroneId,
    pub old_status: DroneStatus,
    pub new_status: DroneStatus,
}

/// Formation command data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationCommandData {
    pub command_id: Uuid,
    pub formation_type: FormationType,
    pub leader_id: DroneId,
    pub positions: Vec<FormationPosition>,
}

/// Formation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormationType {
    Line,
    Vee,
    Diamond,
    Echelon,
    Column,
    Custom,
}

/// Position in formation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationPosition {
    pub drone_id: DroneId,
    pub offset_x: f64,
    pub offset_y: f64,
    pub offset_z: f64,
}

/// Emergency data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyData {
    pub drone_id: DroneId,
    pub emergency_type: EmergencyType,
    pub position: GeoPosition,
    pub message: String,
}

/// Emergency types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmergencyType {
    LowBattery,
    LowFuel,
    SystemFailure,
    LostConnection,
    HostileContact,
    WeatherAlert,
    CollisionWarning,
}

/// Acknowledgment data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckData {
    pub message_id: Uuid,
    pub drone_id: DroneId,
    pub success: bool,
}

/// Discovery response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponseData {
    pub drone_id: DroneId,
    pub capabilities: Vec<String>,
    pub formation_role: Option<String>,
}

/// Complete P2P message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneMessage {
    /// Unique message ID
    pub id: Uuid,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Sender drone ID
    pub sender: DroneId,
    /// Message type and payload
    pub message_type: MessageType,
    /// TTL for message forwarding
    pub ttl: u8,
}

impl DroneMessage {
    /// Create a new message
    pub fn new(sender: DroneId, message_type: MessageType) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            sender,
            message_type,
            ttl: 5,
        }
    }

    /// Create a heartbeat message
    pub fn heartbeat(sender: DroneId) -> Self {
        Self::new(sender, MessageType::Heartbeat)
    }

    /// Create a position update message
    pub fn position_update(
        sender: DroneId,
        position: GeoPosition,
        telemetry: Telemetry,
    ) -> Self {
        Self::new(
            sender.clone(),
            MessageType::PositionUpdate(PositionUpdateData {
                drone_id: sender,
                position,
                telemetry,
            }),
        )
    }

    /// Create a status change message
    pub fn status_change(
        sender: DroneId,
        old_status: DroneStatus,
        new_status: DroneStatus,
    ) -> Self {
        Self::new(
            sender.clone(),
            MessageType::StatusChange(StatusChangeData {
                drone_id: sender,
                old_status,
                new_status,
            }),
        )
    }

    /// Create an emergency message
    pub fn emergency(
        sender: DroneId,
        emergency_type: EmergencyType,
        position: GeoPosition,
        message: String,
    ) -> Self {
        Self::new(
            sender.clone(),
            MessageType::Emergency(EmergencyData {
                drone_id: sender,
                emergency_type,
                position,
                message,
            }),
        )
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Decrement TTL, returns false if message should be dropped
    pub fn decrement_ttl(&mut self) -> bool {
        if self.ttl > 0 {
            self.ttl -= 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = DroneMessage::heartbeat(DroneId::new("REAPER-01"));
        assert!(matches!(msg.message_type, MessageType::Heartbeat));
    }

    #[test]
    fn test_serialization() {
        let msg = DroneMessage::position_update(
            DroneId::new("REAPER-01"),
            GeoPosition::new(34.5553, 69.2075, 3000.0),
            Telemetry::default(),
        );

        let bytes = msg.to_bytes().unwrap();
        let decoded = DroneMessage::from_bytes(&bytes).unwrap();
        
        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.sender.0, msg.sender.0);
    }

    #[test]
    fn test_ttl() {
        let mut msg = DroneMessage::heartbeat(DroneId::new("REAPER-01"));
        assert_eq!(msg.ttl, 5);
        
        assert!(msg.decrement_ttl());
        assert_eq!(msg.ttl, 4);
        
        msg.ttl = 0;
        assert!(!msg.decrement_ttl());
    }
}
