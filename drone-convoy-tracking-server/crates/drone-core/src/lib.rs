//! # Drone Core
//!
//! Core domain models and types for the Drone Convoy Tracking System.
//! This crate provides shared types used across all microservices.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

pub mod error;
pub mod events;
pub mod geo;

pub use error::CoreError;
pub use events::*;
pub use geo::*;

// ============================================================================
// DRONE MODELS
// ============================================================================

/// Unique identifier for a drone
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DroneId(pub String);

impl DroneId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DroneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for DroneId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for DroneId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Operational status of a drone
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DroneStatus {
    /// Drone is powered on but stationary
    Standby,
    /// Drone is actively moving along route
    Moving,
    /// Drone is engaged in active operations
    Engaged,
    /// Drone is returning to base
    Rtb,
    /// Drone has lost connection or is offline
    Offline,
    /// Drone is in maintenance mode
    Maintenance,
}

impl fmt::Display for DroneStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DroneStatus::Standby => write!(f, "STANDBY"),
            DroneStatus::Moving => write!(f, "MOVING"),
            DroneStatus::Engaged => write!(f, "ENGAGED"),
            DroneStatus::Rtb => write!(f, "RTB"),
            DroneStatus::Offline => write!(f, "OFFLINE"),
            DroneStatus::Maintenance => write!(f, "MAINTENANCE"),
        }
    }
}

impl Default for DroneStatus {
    fn default() -> Self {
        Self::Standby
    }
}

/// Type of military drone
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DroneType {
    Mq9Reaper,
    Mq1Predator,
    Rq4GlobalHawk,
    Mq1CGrayEagle,
    Custom(String),
}

impl Default for DroneType {
    fn default() -> Self {
        Self::Mq9Reaper
    }
}

/// Complete drone state including position and telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drone {
    pub id: DroneId,
    pub callsign: String,
    pub drone_type: DroneType,
    pub position: GeoPosition,
    pub telemetry: Telemetry,
    pub status: DroneStatus,
    pub current_waypoint_index: usize,
    pub mission_id: Option<Uuid>,
    pub armed: bool,
    pub last_update: DateTime<Utc>,
}

impl Drone {
    pub fn new(id: impl Into<DroneId>, callsign: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            callsign: callsign.into(),
            drone_type: DroneType::default(),
            position: GeoPosition::default(),
            telemetry: Telemetry::default(),
            status: DroneStatus::default(),
            current_waypoint_index: 0,
            mission_id: None,
            armed: false,
            last_update: Utc::now(),
        }
    }

    /// Update drone position and recalculate heading
    pub fn update_position(&mut self, new_position: GeoPosition) {
        self.position = new_position;
        self.last_update = Utc::now();
    }

    /// Check if drone battery is critically low
    pub fn is_battery_critical(&self) -> bool {
        self.telemetry.battery_level < 15
    }

    /// Check if drone is operational
    pub fn is_operational(&self) -> bool {
        self.status != DroneStatus::Offline && self.status != DroneStatus::Maintenance
    }
}

// ============================================================================
// TELEMETRY MODELS
// ============================================================================

/// Real-time telemetry data from a drone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Telemetry {
    /// Battery level percentage (0-100)
    pub battery_level: u8,
    /// Fuel level percentage (0-100)
    pub fuel_level: u8,
    /// Overall system health percentage (0-100)
    pub system_health: u8,
    /// Current speed in km/h
    pub speed: f64,
    /// Heading in degrees (0-360)
    pub heading: f64,
    /// Signal strength percentage (0-100)
    pub signal_strength: u8,
    /// Internal temperature in Celsius
    pub temperature: f64,
    /// Timestamp of this telemetry reading
    pub timestamp: DateTime<Utc>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            battery_level: 100,
            fuel_level: 100,
            system_health: 100,
            speed: 0.0,
            heading: 0.0,
            signal_strength: 100,
            temperature: 25.0,
            timestamp: Utc::now(),
        }
    }
}

impl Telemetry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create telemetry with specific values
    pub fn with_values(
        battery: u8,
        fuel: u8,
        health: u8,
        speed: f64,
        heading: f64,
    ) -> Self {
        Self {
            battery_level: battery.min(100),
            fuel_level: fuel.min(100),
            system_health: health.min(100),
            speed,
            heading: heading % 360.0,
            ..Default::default()
        }
    }
}

// ============================================================================
// WAYPOINT MODELS
// ============================================================================

/// Unique identifier for a waypoint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WaypointId(pub String);

impl WaypointId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for WaypointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A waypoint in a convoy route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: WaypointId,
    pub name: String,
    pub position: GeoPosition,
    pub waypoint_type: WaypointType,
    pub expected_arrival: Option<DateTime<Utc>>,
    pub actual_arrival: Option<DateTime<Utc>>,
    pub loiter_time_seconds: Option<u32>,
}

impl Waypoint {
    pub fn new(id: impl Into<String>, name: impl Into<String>, lat: f64, lng: f64) -> Self {
        Self {
            id: WaypointId::new(id),
            name: name.into(),
            position: GeoPosition::new(lat, lng, 0.0),
            waypoint_type: WaypointType::Standard,
            expected_arrival: None,
            actual_arrival: None,
            loiter_time_seconds: None,
        }
    }
}

/// Type of waypoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WaypointType {
    /// Standard flyover waypoint
    Standard,
    /// Origin/starting point
    Origin,
    /// Final destination
    Destination,
    /// Checkpoint requiring acknowledgment
    Checkpoint,
    /// Rally point for regrouping
    Rally,
    /// Emergency landing zone
    Emergency,
}

// ============================================================================
// MISSION MODELS
// ============================================================================

/// Unique identifier for a mission
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionId(pub Uuid);

impl MissionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for MissionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MissionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Mission status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MissionStatus {
    Planning,
    Active,
    Paused,
    Completed,
    Aborted,
}

impl Default for MissionStatus {
    fn default() -> Self {
        Self::Planning
    }
}

/// A convoy mission with route and assigned drones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: MissionId,
    pub name: String,
    pub description: Option<String>,
    pub status: MissionStatus,
    pub waypoints: Vec<Waypoint>,
    pub assigned_drones: Vec<DroneId>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Mission {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: MissionId::new(),
            name: name.into(),
            description: None,
            status: MissionStatus::default(),
            waypoints: Vec::new(),
            assigned_drones: Vec::new(),
            start_time: None,
            end_time: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a waypoint to the mission route
    pub fn add_waypoint(&mut self, waypoint: Waypoint) {
        self.waypoints.push(waypoint);
        self.updated_at = Utc::now();
    }

    /// Assign a drone to this mission
    pub fn assign_drone(&mut self, drone_id: DroneId) {
        if !self.assigned_drones.contains(&drone_id) {
            self.assigned_drones.push(drone_id);
            self.updated_at = Utc::now();
        }
    }

    /// Start the mission
    pub fn start(&mut self) {
        self.status = MissionStatus::Active;
        self.start_time = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Complete the mission
    pub fn complete(&mut self) {
        self.status = MissionStatus::Completed;
        self.end_time = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Get total route distance in kilometers
    pub fn total_distance_km(&self) -> f64 {
        if self.waypoints.len() < 2 {
            return 0.0;
        }

        self.waypoints
            .windows(2)
            .map(|w| w[0].position.distance_to(&w[1].position))
            .sum()
    }
}

// ============================================================================
// OPENCV/CV TRACKING MODELS
// ============================================================================

/// Bounding box for detected objects
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl BoundingBox {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    pub fn area(&self) -> i32 {
        self.width * self.height
    }
}

/// RGB color for halo visualization
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HaloColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl HaloColor {
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };
    pub const CYAN: Self = Self { r: 0, g: 255, b: 255 };

    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_bgr(&self) -> (u8, u8, u8) {
        (self.b, self.g, self.r)
    }
}

impl Default for HaloColor {
    fn default() -> Self {
        Self::RED
    }
}

/// Detected halo around a drone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedHalo {
    pub center_x: i32,
    pub center_y: i32,
    pub radius: i32,
    pub color: HaloColor,
    pub confidence: f64,
}

impl DetectedHalo {
    pub fn new(center_x: i32, center_y: i32, radius: i32) -> Self {
        Self {
            center_x,
            center_y,
            radius,
            color: HaloColor::default(),
            confidence: 1.0,
        }
    }
}

/// Computer vision tracking result for a single drone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingResult {
    pub drone_id: DroneId,
    pub tracking_id: u32,
    pub bbox: BoundingBox,
    pub halo: Option<DetectedHalo>,
    pub estimated_position: Option<GeoPosition>,
    pub confidence: f64,
    pub frame_timestamp: DateTime<Utc>,
}

impl TrackingResult {
    pub fn new(drone_id: DroneId, tracking_id: u32, bbox: BoundingBox) -> Self {
        Self {
            drone_id,
            tracking_id,
            bbox,
            halo: None,
            estimated_position: None,
            confidence: 1.0,
            frame_timestamp: Utc::now(),
        }
    }

    pub fn with_halo(mut self, halo: DetectedHalo) -> Self {
        self.halo = Some(halo);
        self
    }

    pub fn with_position(mut self, position: GeoPosition) -> Self {
        self.estimated_position = Some(position);
        self
    }
}

// ============================================================================
// ALERT MODELS
// ============================================================================

/// Severity level of an alert
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Type of alert
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AlertType {
    BatteryLow,
    FuelLow,
    SignalLost,
    SystemFailure,
    WaypointDeviation,
    GeofenceBreach,
    CollisionWarning,
    WeatherAlert,
    Custom(String),
}

/// System alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub severity: AlertSeverity,
    pub alert_type: AlertType,
    pub message: String,
    pub drone_id: Option<DroneId>,
    pub mission_id: Option<MissionId>,
    pub created_at: DateTime<Utc>,
    pub acknowledged: bool,
    pub resolved: bool,
}

impl Alert {
    pub fn new(severity: AlertSeverity, alert_type: AlertType, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            severity,
            alert_type,
            message: message.into(),
            drone_id: None,
            mission_id: None,
            created_at: Utc::now(),
            acknowledged: false,
            resolved: false,
        }
    }

    pub fn for_drone(mut self, drone_id: DroneId) -> Self {
        self.drone_id = Some(drone_id);
        self
    }

    pub fn for_mission(mut self, mission_id: MissionId) -> Self {
        self.mission_id = Some(mission_id);
        self
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drone_creation() {
        let drone = Drone::new("REAPER-01", "Alpha Lead");
        assert_eq!(drone.id.as_str(), "REAPER-01");
        assert_eq!(drone.callsign, "Alpha Lead");
        assert_eq!(drone.status, DroneStatus::Standby);
        assert!(!drone.armed);
    }

    #[test]
    fn test_telemetry_defaults() {
        let telemetry = Telemetry::default();
        assert_eq!(telemetry.battery_level, 100);
        assert_eq!(telemetry.fuel_level, 100);
        assert_eq!(telemetry.system_health, 100);
    }

    #[test]
    fn test_mission_distance() {
        let mut mission = Mission::new("Test Mission");
        mission.add_waypoint(Waypoint::new("WP1", "Start", 34.5553, 69.2075));
        mission.add_waypoint(Waypoint::new("WP2", "End", 34.6234, 69.1123));
        
        let distance = mission.total_distance_km();
        assert!(distance > 0.0);
    }

    #[test]
    fn test_bounding_box_center() {
        let bbox = BoundingBox::new(100, 100, 50, 50);
        assert_eq!(bbox.center(), (125, 125));
        assert_eq!(bbox.area(), 2500);
    }

    #[test]
    fn test_halo_color_bgr() {
        let color = HaloColor::RED;
        assert_eq!(color.to_bgr(), (0, 0, 255));
    }
}
