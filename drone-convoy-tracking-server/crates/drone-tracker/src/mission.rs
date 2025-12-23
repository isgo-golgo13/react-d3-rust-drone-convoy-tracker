//! Mission execution and waypoint management

use drone_core::{DroneId, GeoPosition, Mission, MissionStatus, Waypoint, WaypointId};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Mission executor handles waypoint progression
pub struct MissionExecutor {
    /// Current mission
    mission: Option<Mission>,
    /// Progress per drone (waypoint index)
    drone_progress: HashMap<DroneId, WaypointProgress>,
    /// Mission start time
    start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Waypoint proximity threshold (km)
    threshold_km: f64,
}

/// Progress tracking for a drone
#[derive(Debug, Clone)]
pub struct WaypointProgress {
    pub current_index: usize,
    pub progress_to_next: f64,
    pub waypoints_completed: Vec<WaypointId>,
    pub estimated_arrival: Option<chrono::DateTime<chrono::Utc>>,
}

impl WaypointProgress {
    pub fn new() -> Self {
        Self {
            current_index: 0,
            progress_to_next: 0.0,
            waypoints_completed: Vec::new(),
            estimated_arrival: None,
        }
    }
}

impl Default for WaypointProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl MissionExecutor {
    /// Create a new mission executor
    pub fn new() -> Self {
        Self {
            mission: None,
            drone_progress: HashMap::new(),
            start_time: None,
            threshold_km: 0.5,
        }
    }

    /// Set the active mission
    pub fn set_mission(&mut self, mission: Mission) {
        info!("Mission set: {} with {} waypoints", 
              mission.name, mission.waypoints.len());
        
        // Initialize progress for assigned drones
        for drone_id in &mission.assigned_drones {
            self.drone_progress.insert(drone_id.clone(), WaypointProgress::new());
        }
        
        self.mission = Some(mission);
    }

    /// Start the mission
    pub fn start(&mut self) {
        if let Some(ref mut mission) = self.mission {
            mission.start();
            self.start_time = Some(chrono::Utc::now());
            info!("Mission started: {}", mission.name);
        }
    }

    /// Pause the mission
    pub fn pause(&mut self) {
        if let Some(ref mut mission) = self.mission {
            mission.status = MissionStatus::Paused;
            info!("Mission paused: {}", mission.name);
        }
    }

    /// Resume the mission
    pub fn resume(&mut self) {
        if let Some(ref mut mission) = self.mission {
            mission.status = MissionStatus::Active;
            info!("Mission resumed: {}", mission.name);
        }
    }

    /// Abort the mission
    pub fn abort(&mut self) {
        if let Some(ref mut mission) = self.mission {
            mission.status = MissionStatus::Aborted;
            info!("Mission aborted: {}", mission.name);
        }
    }

    /// Complete the mission
    pub fn complete(&mut self) {
        if let Some(ref mut mission) = self.mission {
            mission.complete();
            info!("Mission completed: {}", mission.name);
        }
    }

    /// Update drone position and check waypoint progress
    pub fn update_drone_position(
        &mut self,
        drone_id: &DroneId,
        position: &GeoPosition,
        speed: f64,
    ) -> Option<WaypointReached> {
        let mission = self.mission.as_ref()?;
        
        if mission.status != MissionStatus::Active {
            return None;
        }

        let progress = self.drone_progress.get_mut(drone_id)?;
        let current_wp = mission.waypoints.get(progress.current_index)?;
        
        // Calculate distance to current waypoint
        let distance = position.distance_to(&current_wp.position);
        
        // Check if waypoint reached
        if distance < self.threshold_km {
            let reached = WaypointReached {
                drone_id: drone_id.clone(),
                waypoint_id: current_wp.id.clone(),
                waypoint_name: current_wp.name.clone(),
                position: current_wp.position.clone(),
                index: progress.current_index,
            };
            
            progress.waypoints_completed.push(current_wp.id.clone());
            progress.current_index += 1;
            progress.progress_to_next = 0.0;
            
            info!("{} reached waypoint: {}", drone_id, current_wp.name);
            
            // Check if mission complete
            if progress.current_index >= mission.waypoints.len() {
                debug!("{} completed all waypoints", drone_id);
            }
            
            return Some(reached);
        }
        
        // Update progress to next waypoint
        if let Some(next_wp) = mission.waypoints.get(progress.current_index) {
            let total_distance = if progress.current_index > 0 {
                mission.waypoints[progress.current_index - 1]
                    .position
                    .distance_to(&next_wp.position)
            } else {
                distance + 1.0 // Avoid division by zero
            };
            
            progress.progress_to_next = 1.0 - (distance / total_distance).min(1.0);
            
            // Estimate arrival time
            if speed > 0.0 {
                let time_hours = distance / speed;
                let duration = chrono::Duration::seconds((time_hours * 3600.0) as i64);
                progress.estimated_arrival = Some(chrono::Utc::now() + duration);
            }
        }
        
        None
    }

    /// Get drone progress
    pub fn get_progress(&self, drone_id: &DroneId) -> Option<&WaypointProgress> {
        self.drone_progress.get(drone_id)
    }

    /// Get current waypoint for a drone
    pub fn get_current_waypoint(&self, drone_id: &DroneId) -> Option<&Waypoint> {
        let mission = self.mission.as_ref()?;
        let progress = self.drone_progress.get(drone_id)?;
        mission.waypoints.get(progress.current_index)
    }

    /// Get next waypoint for a drone
    pub fn get_next_waypoint(&self, drone_id: &DroneId) -> Option<&Waypoint> {
        let mission = self.mission.as_ref()?;
        let progress = self.drone_progress.get(drone_id)?;
        mission.waypoints.get(progress.current_index + 1)
    }

    /// Check if all drones have completed the mission
    pub fn is_complete(&self) -> bool {
        let mission = match &self.mission {
            Some(m) => m,
            None => return false,
        };

        self.drone_progress.values().all(|p| {
            p.current_index >= mission.waypoints.len()
        })
    }

    /// Get mission status
    pub fn status(&self) -> Option<MissionStatus> {
        self.mission.as_ref().map(|m| m.status)
    }

    /// Get overall mission progress (0.0 to 1.0)
    pub fn overall_progress(&self) -> f64 {
        let mission = match &self.mission {
            Some(m) if !m.waypoints.is_empty() => m,
            _ => return 0.0,
        };

        let total_waypoints = mission.waypoints.len() * self.drone_progress.len();
        if total_waypoints == 0 {
            return 0.0;
        }

        let completed: usize = self.drone_progress.values()
            .map(|p| p.waypoints_completed.len())
            .sum();

        completed as f64 / total_waypoints as f64
    }

    /// Set waypoint threshold
    pub fn set_threshold(&mut self, km: f64) {
        self.threshold_km = km.max(0.1);
    }
}

impl Default for MissionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Event indicating a drone reached a waypoint
#[derive(Debug, Clone)]
pub struct WaypointReached {
    pub drone_id: DroneId,
    pub waypoint_id: WaypointId,
    pub waypoint_name: String,
    pub position: GeoPosition,
    pub index: usize,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mission() -> Mission {
        let mut mission = Mission::new("Test Mission");
        mission.add_waypoint(Waypoint::new("WP1", "Start", 34.5, 69.2));
        mission.add_waypoint(Waypoint::new("WP2", "Middle", 34.6, 69.1));
        mission.add_waypoint(Waypoint::new("WP3", "End", 34.7, 69.0));
        mission.assign_drone(DroneId::new("REAPER-01"));
        mission
    }

    #[test]
    fn test_executor_creation() {
        let executor = MissionExecutor::new();
        assert!(executor.mission.is_none());
    }

    #[test]
    fn test_set_mission() {
        let mut executor = MissionExecutor::new();
        executor.set_mission(create_test_mission());
        
        assert!(executor.mission.is_some());
        assert!(executor.drone_progress.contains_key(&DroneId::new("REAPER-01")));
    }

    #[test]
    fn test_start_mission() {
        let mut executor = MissionExecutor::new();
        executor.set_mission(create_test_mission());
        executor.start();
        
        assert_eq!(executor.status(), Some(MissionStatus::Active));
    }

    #[test]
    fn test_waypoint_reached() {
        let mut executor = MissionExecutor::new();
        executor.set_mission(create_test_mission());
        executor.start();
        executor.set_threshold(1.0); // 1km threshold
        
        // Position at first waypoint
        let result = executor.update_drone_position(
            &DroneId::new("REAPER-01"),
            &GeoPosition::new(34.5, 69.2, 3000.0),
            400.0,
        );
        
        assert!(result.is_some());
        assert_eq!(result.unwrap().waypoint_name, "Start");
    }

    #[test]
    fn test_overall_progress() {
        let mut executor = MissionExecutor::new();
        executor.set_mission(create_test_mission());
        executor.start();
        
        assert_eq!(executor.overall_progress(), 0.0);
    }
}
