//! Error types for the drone convoy system

use thiserror::Error;

/// Core error type for the drone convoy system
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Drone not found: {0}")]
    DroneNotFound(String),

    #[error("Mission not found: {0}")]
    MissionNotFound(String),

    #[error("Waypoint not found: {0}")]
    WaypointNotFound(String),

    #[error("Invalid position: latitude={lat}, longitude={lng}")]
    InvalidPosition { lat: f64, lng: f64 },

    #[error("Invalid telemetry data: {0}")]
    InvalidTelemetry(String),

    #[error("Mission already active: {0}")]
    MissionAlreadyActive(String),

    #[error("Drone already assigned: {0}")]
    DroneAlreadyAssigned(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl CoreError {
    pub fn drone_not_found(id: impl Into<String>) -> Self {
        Self::DroneNotFound(id.into())
    }

    pub fn mission_not_found(id: impl Into<String>) -> Self {
        Self::MissionNotFound(id.into())
    }

    pub fn waypoint_not_found(id: impl Into<String>) -> Self {
        Self::WaypointNotFound(id.into())
    }

    pub fn invalid_position(lat: f64, lng: f64) -> Self {
        Self::InvalidPosition { lat, lng }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
