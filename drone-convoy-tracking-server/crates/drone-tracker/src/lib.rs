//! # Drone Tracker - Main Orchestration
//!
//! Central coordination for the drone convoy tracking system.
//! Combines CV tracking, P2P networking, and database persistence
//! into a unified tracking engine.
//!
//! ## Features
//! - Real-time drone position tracking
//! - Waypoint progress monitoring
//! - Convoy formation management
//! - Alert generation and handling
//! - Integration with all subsystems

pub mod convoy;
pub mod engine;
pub mod events;

pub use convoy::ConvoyManager;
pub use engine::TrackingEngine;
pub use events::EventBus;

use drone_core::{
    Alert, AlertSeverity, AlertType, Drone, DroneId, DroneStatus,
    Event, GeoPosition, Mission, MissionId, MissionStatus, Telemetry,
    TrackingResult, Waypoint, WaypointId,
};
//use drone_cv::CvEngine;
use drone_db::DbClient;
use drone_p2p::P2pManager;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

/// Tracking system configuration
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Update interval for position tracking
    pub update_interval: Duration,
    /// Waypoint arrival threshold in meters
    pub waypoint_threshold_meters: f64,
    /// Enable CV tracking
    //pub cv_enabled: bool,
    /// Enable P2P networking
    pub p2p_enabled: bool,
    /// Enable database persistence
    pub db_enabled: bool,
    /// Alert thresholds
    pub battery_warning_threshold: u8,
    pub battery_critical_threshold: u8,
    pub fuel_warning_threshold: u8,
    pub fuel_critical_threshold: u8,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            update_interval: Duration::from_millis(100),
            waypoint_threshold_meters: 100.0,
            //cv_enabled: true,
            p2p_enabled: false, // Disabled by default for simplicity
            db_enabled: true,
            battery_warning_threshold: 30,
            battery_critical_threshold: 15,
            fuel_warning_threshold: 25,
            fuel_critical_threshold: 10,
        }
    }
}

/// Main tracking coordinator
pub struct DroneTracker {
    config: TrackerConfig,
    /// All tracked drones
    drones: Arc<DashMap<DroneId, TrackedDrone>>,
    /// Active mission
    mission: Arc<RwLock<Option<Mission>>>,
    /// CV engine (optional)
    //cv_engine: Option<Arc<RwLock<CvEngine>>>,
    /// Database client (optional)
    db: Option<Arc<DbClient>>,
    /// P2P manager (optional)
    p2p: Option<Arc<P2pManager>>,
    /// Event broadcaster
    event_tx: broadcast::Sender<Event>,
    /// Alert sender
    alert_tx: mpsc::Sender<Alert>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Extended drone tracking state
#[derive(Debug, Clone)]
pub struct TrackedDrone {
    /// Base drone data
    pub drone: Drone,
    /// Current waypoint index
    pub waypoint_index: usize,
    /// Progress to next waypoint (0.0 - 1.0)
    pub waypoint_progress: f64,
    /// Last CV tracking result
    //pub last_cv_result: Option<TrackingResult>,
    /// Last position update time
    pub last_update: DateTime<Utc>,
    /// Historical positions (last N)
    pub position_history: Vec<(DateTime<Utc>, GeoPosition)>,
    /// Alerts for this drone
    pub active_alerts: Vec<Alert>,
}

impl TrackedDrone {
    pub fn new(drone: Drone) -> Self {
        Self {
            drone,
            waypoint_index: 0,
            waypoint_progress: 0.0,
            //last_cv_result: None,
            last_update: Utc::now(),
            position_history: Vec::with_capacity(100),
            active_alerts: Vec::new(),
        }
    }

    /// Update position and add to history
    pub fn update_position(&mut self, position: GeoPosition, telemetry: Telemetry) {
        self.drone.position = position;
        self.drone.telemetry = telemetry;
        self.last_update = Utc::now();

        // Keep last 100 positions
        self.position_history.push((self.last_update, position));
        if self.position_history.len() > 100 {
            self.position_history.remove(0);
        }
    }

    /// Check if drone is stale (no updates)
    pub fn is_stale(&self, timeout: Duration) -> bool {
        Utc::now().signed_duration_since(self.last_update)
            > chrono::Duration::from_std(timeout).unwrap_or(chrono::Duration::seconds(30))
    }
}

impl DroneTracker {
    /// Create a new drone tracker
    pub async fn new(config: TrackerConfig) -> anyhow::Result<Self> {
        info!("Initializing Drone Tracker...");

        let (event_tx, _) = broadcast::channel(1024);
        let (alert_tx, _alert_rx) = mpsc::channel(256);

        // Initialize CV engine if enabled
        // let cv_engine = if config.cv_enabled {
        //     match CvEngine::new() {
        //         Ok(engine) => {
        //             info!("CV engine initialized");
        //             Some(Arc::new(RwLock::new(engine)))
        //         }
        //         Err(e) => {
        //             warn!("CV engine initialization failed: {}", e);
        //             None
        //         }
        //     }
        // } else {
        //     None
        // };

        // Initialize P2P if enabled
        let p2p = if config.p2p_enabled {
            match P2pManager::new(drone_p2p::P2pConfig::default()).await {
                Ok(manager) => {
                    info!("P2P network initialized");
                    Some(Arc::new(manager))
                }
                Err(e) => {
                    warn!("P2P initialization failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            drones: Arc::new(DashMap::new()),
            mission: Arc::new(RwLock::new(None)),
            //cv_engine,
            db: None, // Set via set_database
            p2p,
            event_tx,
            alert_tx,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Set database client
    pub fn set_database(&mut self, db: Arc<DbClient>) {
        self.db = Some(db);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_tx.subscribe()
    }

    /// Register a drone for tracking
    pub fn register_drone(&self, drone: Drone) {
        let id = drone.id.clone();
        self.drones.insert(id.clone(), TrackedDrone::new(drone));
        info!("Registered drone: {}", id);
    }

    /// Update drone position
    pub async fn update_drone_position(
        &self,
        drone_id: &DroneId,
        position: GeoPosition,
        telemetry: Telemetry,
    ) -> anyhow::Result<()> {
        if let Some(mut tracked) = self.drones.get_mut(drone_id) {
            let old_status = tracked.drone.status;
            
            tracked.update_position(position, telemetry.clone());
            
            // Check waypoint progress
            if let Some(mission) = self.mission.read().as_ref() {
                self.check_waypoint_progress(&mut tracked, mission);
            }

            // Check for alerts
            self.check_alerts(&tracked);

            // Broadcast position update
            let event = Event::drone_position_updated(
                drone_id.clone(),
                position,
                telemetry.clone(),
            );
            let _ = self.event_tx.send(event);

            // Persist to database
            if let Some(db) = &self.db {
                let mission_id = self.mission.read().as_ref().map(|m| m.id.clone());
                if let Err(e) = db.telemetry().insert(
                    drone_id,
                    &position,
                    &telemetry,
                    mission_id.as_ref(),
                ).await {
                    warn!("Failed to persist telemetry: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Check and update waypoint progress
    fn check_waypoint_progress(&self, tracked: &mut TrackedDrone, mission: &Mission) {
        if tracked.waypoint_index >= mission.waypoints.len() {
            return;
        }

        let current_wp = &mission.waypoints[tracked.waypoint_index];
        let distance = tracked.drone.position.distance_to(&current_wp.position);

        // Convert to meters
        let distance_meters = distance * 1000.0;

        if distance_meters < self.config.waypoint_threshold_meters {
            // Reached waypoint
            info!(
                "Drone {} reached waypoint {}",
                tracked.drone.id, current_wp.name
            );

            // Emit event
            let event = Event::waypoint_reached(
                tracked.drone.id.clone(),
                current_wp.id.clone(),
                tracked.drone.position,
            );
            let _ = self.event_tx.send(event);

            // Advance to next waypoint
            tracked.waypoint_index += 1;
            tracked.waypoint_progress = 0.0;
        } else if tracked.waypoint_index > 0 {
            // Calculate progress between waypoints
            let prev_wp = &mission.waypoints[tracked.waypoint_index - 1];
            let total_distance = prev_wp.position.distance_to(&current_wp.position);
            let remaining = tracked.drone.position.distance_to(&current_wp.position);
            
            if total_distance > 0.0 {
                tracked.waypoint_progress = 1.0 - (remaining / total_distance);
            }
        }
    }

    /// Check for alert conditions
    fn check_alerts(&self, tracked: &TrackedDrone) {
        let drone = &tracked.drone;
        let id = &drone.id;

        // Battery alerts
        if drone.telemetry.battery_level < self.config.battery_critical_threshold {
            let alert = Alert::new(
                AlertSeverity::Critical,
                AlertType::BatteryLow,
                format!("Battery critical: {}%", drone.telemetry.battery_level),
            ).for_drone(id.clone());
            
            let _ = self.alert_tx.try_send(alert);
        } else if drone.telemetry.battery_level < self.config.battery_warning_threshold {
            let alert = Alert::new(
                AlertSeverity::Warning,
                AlertType::BatteryLow,
                format!("Battery low: {}%", drone.telemetry.battery_level),
            ).for_drone(id.clone());
            
            let _ = self.alert_tx.try_send(alert);
        }

        // Fuel alerts
        if drone.telemetry.fuel_level < self.config.fuel_critical_threshold {
            let alert = Alert::new(
                AlertSeverity::Critical,
                AlertType::FuelLow,
                format!("Fuel critical: {}%", drone.telemetry.fuel_level),
            ).for_drone(id.clone());
            
            let _ = self.alert_tx.try_send(alert);
        }
    }

    /// Set active mission
    pub fn set_mission(&self, mission: Mission) {
        *self.mission.write() = Some(mission);
    }

    /// Get active mission
    pub fn get_mission(&self) -> Option<Mission> {
        self.mission.read().clone()
    }

    /// Get all tracked drones
    pub fn get_all_drones(&self) -> Vec<TrackedDrone> {
        self.drones.iter().map(|r| r.value().clone()).collect()
    }

    /// Get specific drone
    pub fn get_drone(&self, id: &DroneId) -> Option<TrackedDrone> {
        self.drones.get(id).map(|r| r.value().clone())
    }

    /// Get drone count
    pub fn drone_count(&self) -> usize {
        self.drones.len()
    }

    /// Start the tracking engine
    pub async fn start(&self) -> anyhow::Result<()> {
        *self.running.write() = true;
        info!("🚀 Drone Tracker started");
        
        // Start P2P if available
        if let Some(p2p) = &self.p2p {
            p2p.start().await?;
        }

        Ok(())
    }

    /// Stop the tracking engine
    pub async fn stop(&self) -> anyhow::Result<()> {
        *self.running.write() = false;
        info!("🛑 Drone Tracker stopped");

        if let Some(p2p) = &self.p2p {
            p2p.stop().await?;
        }

        Ok(())
    }

    /// Check if tracker is running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tracker_creation() {
        let config = TrackerConfig {
            //cv_enabled: false,
            p2p_enabled: false,
            db_enabled: false,
            ..Default::default()
        };
        
        let tracker = DroneTracker::new(config).await;
        assert!(tracker.is_ok());
    }

    #[tokio::test]
    async fn test_drone_registration() {
        let config = TrackerConfig {
            //cv_enabled: false,
            p2p_enabled: false,
            db_enabled: false,
            ..Default::default()
        };
        
        let tracker = DroneTracker::new(config).await.unwrap();
        
        let drone = Drone::new(DroneId::new("REAPER-01"), "Alpha Lead");
        tracker.register_drone(drone);
        
        assert_eq!(tracker.drone_count(), 1);
        assert!(tracker.get_drone(&DroneId::new("REAPER-01")).is_some());
    }

    #[tokio::test]
    async fn test_position_update() {
        let config = TrackerConfig {
            //cv_enabled: false,
            p2p_enabled: false,
            db_enabled: false,
            ..Default::default()
        };
        
        let tracker = DroneTracker::new(config).await.unwrap();
        
        let drone = Drone::new(DroneId::new("REAPER-01"), "Alpha Lead");
        tracker.register_drone(drone);
        
        let position = GeoPosition::new(34.5553, 69.2075, 3000.0);
        let telemetry = Telemetry::default();
        
        tracker.update_drone_position(
            &DroneId::new("REAPER-01"),
            position,
            telemetry,
        ).await.unwrap();
        
        let tracked = tracker.get_drone(&DroneId::new("REAPER-01")).unwrap();
        assert_eq!(tracked.drone.position.latitude, 34.5553);
    }
}
