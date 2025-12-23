//! Tracker state management and snapshots

use drone_core::{Drone, DroneId, Mission, TrackingResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete tracker state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerState {
    /// All drones
    pub drones: Vec<DroneSnapshot>,
    /// Active mission
    pub mission: Option<MissionSnapshot>,
    /// CV tracking results
    pub tracking: Vec<TrackingSnapshot>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Statistics
    pub stats: TrackerStats,
}

impl TrackerState {
    /// Create empty state
    pub fn empty() -> Self {
        Self {
            drones: Vec::new(),
            mission: None,
            tracking: Vec::new(),
            timestamp: chrono::Utc::now(),
            stats: TrackerStats::default(),
        }
    }

    /// Create from drones and mission
    pub fn from_data(
        drones: Vec<Drone>,
        mission: Option<Mission>,
        tracking: Vec<TrackingResult>,
    ) -> Self {
        let stats = TrackerStats {
            drone_count: drones.len(),
            active_count: drones.iter().filter(|d| d.is_operational()).count(),
            tracking_count: tracking.len(),
            mission_active: mission.is_some(),
        };

        Self {
            drones: drones.into_iter().map(DroneSnapshot::from).collect(),
            mission: mission.map(MissionSnapshot::from),
            tracking: tracking.into_iter().map(TrackingSnapshot::from).collect(),
            timestamp: chrono::Utc::now(),
            stats,
        }
    }
}

/// Snapshot of drone state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneSnapshot {
    pub id: String,
    pub callsign: String,
    pub status: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub heading: f64,
    pub speed: f64,
    pub battery: u8,
    pub fuel: u8,
    pub health: u8,
    pub armed: bool,
    pub current_waypoint: usize,
}

impl From<Drone> for DroneSnapshot {
    fn from(drone: Drone) -> Self {
        Self {
            id: drone.id.0,
            callsign: drone.callsign,
            status: format!("{}", drone.status),
            latitude: drone.position.latitude,
            longitude: drone.position.longitude,
            altitude: drone.position.altitude,
            heading: drone.telemetry.heading,
            speed: drone.telemetry.speed,
            battery: drone.telemetry.battery_level,
            fuel: drone.telemetry.fuel_level,
            health: drone.telemetry.system_health,
            armed: drone.armed,
            current_waypoint: drone.current_waypoint_index,
        }
    }
}

/// Snapshot of mission state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionSnapshot {
    pub id: String,
    pub name: String,
    pub status: String,
    pub waypoint_count: usize,
    pub drone_count: usize,
    pub distance_km: f64,
}

impl From<Mission> for MissionSnapshot {
    fn from(mission: Mission) -> Self {
        Self {
            id: mission.id.0.to_string(),
            name: mission.name,
            status: format!("{:?}", mission.status),
            waypoint_count: mission.waypoints.len(),
            drone_count: mission.assigned_drones.len(),
            distance_km: mission.total_distance_km(),
        }
    }
}

/// Snapshot of CV tracking result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingSnapshot {
    pub drone_id: String,
    pub tracking_id: u32,
    pub bbox_x: i32,
    pub bbox_y: i32,
    pub bbox_width: i32,
    pub bbox_height: i32,
    pub confidence: f64,
    pub halo_detected: bool,
    pub estimated_lat: Option<f64>,
    pub estimated_lng: Option<f64>,
}

impl From<TrackingResult> for TrackingSnapshot {
    fn from(result: TrackingResult) -> Self {
        Self {
            drone_id: result.drone_id.0,
            tracking_id: result.tracking_id,
            bbox_x: result.bbox.x,
            bbox_y: result.bbox.y,
            bbox_width: result.bbox.width,
            bbox_height: result.bbox.height,
            confidence: result.confidence,
            halo_detected: result.halo.is_some(),
            estimated_lat: result.estimated_position.map(|p| p.latitude),
            estimated_lng: result.estimated_position.map(|p| p.longitude),
        }
    }
}

/// Tracker statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackerStats {
    pub drone_count: usize,
    pub active_count: usize,
    pub tracking_count: usize,
    pub mission_active: bool,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state() {
        let state = TrackerState::empty();
        assert!(state.drones.is_empty());
        assert!(state.mission.is_none());
    }

    #[test]
    fn test_drone_snapshot() {
        let drone = Drone::new(DroneId::new("REAPER-01"), "Alpha");
        let snapshot = DroneSnapshot::from(drone);
        
        assert_eq!(snapshot.id, "REAPER-01");
        assert_eq!(snapshot.callsign, "Alpha");
    }

    #[test]
    fn test_state_from_data() {
        let drones = vec![
            Drone::new(DroneId::new("REAPER-01"), "Alpha"),
            Drone::new(DroneId::new("REAPER-02"), "Bravo"),
        ];
        
        let state = TrackerState::from_data(drones, None, Vec::new());
        
        assert_eq!(state.drones.len(), 2);
        assert_eq!(state.stats.drone_count, 2);
    }
}
