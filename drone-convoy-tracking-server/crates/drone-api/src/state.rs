//! Application state management

use crate::config::ApiConfig;
use drone_core::{Drone, DroneId, Mission, GeoPosition, Waypoint, WaypointType};
//use drone_cv::CvEngine;
use drone_db::DbClient;
use drone_websocket::WebSocketHub;

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tracing::{info, warn};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Configuration
    pub config: ApiConfig,
    /// Database client (optional - may not be available)
    pub db: Option<Arc<DbClient>>,
    /// WebSocket hub for real-time updates
    pub ws_hub: Arc<WebSocketHub>,
    /// CV engine for tracking
    //pub cv_engine: Option<Arc<RwLock<CvEngine>>>,
    /// In-memory drone cache
    pub drones: Arc<DashMap<DroneId, Drone>>,
    /// Active mission
    pub active_mission: Arc<RwLock<Option<Mission>>>,
    /// Simulation reset flag
    pub reset_flag: Arc<AtomicBool>,
}

impl AppState {
    /// Create new application state with all components
    pub async fn new(config: ApiConfig) -> anyhow::Result<Self> {
        // Initialize database
        let db = match DbClient::new(config.db.clone()).await {
            Ok(client) => {
                info!("Database connected");
                Some(Arc::new(client))
            }
            Err(e) => {
                warn!("Database connection failed: {}", e);
                None
            }
        };

        // Initialize CV engine
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

        // Initialize WebSocket hub
        let ws_hub = Arc::new(WebSocketHub::new());
        info!("WebSocket hub initialized");

        // Initialize drone cache with 12 REAPER drones
        let drones = Arc::new(DashMap::new());
        for i in 1..=12 {
            let id = DroneId::new(format!("REAPER-{:02}", i));
            let drone = Drone::new(id.clone(), format!("Reaper {}", i));
            drones.insert(id, drone);
        }
        info!("Initialized {} drones in cache", drones.len());

        // Create default mission
        let mission = create_default_mission();
        let active_mission = Arc::new(RwLock::new(Some(mission)));
        let reset_flag = Arc::new(AtomicBool::new(false));

        Ok(Self {
            config,
            db,
            ws_hub,
            //cv_engine,
            drones,
            active_mission,
            reset_flag,
        })
    }

    /// Create state without database (degraded mode)
    pub async fn new_without_db(config: ApiConfig) -> anyhow::Result<Self> {
        // let cv_engine = if config.cv_enabled {
        //     CvEngine::new().ok().map(|e| Arc::new(RwLock::new(e)))
        // } else {
        //     None
        // };

        let ws_hub = Arc::new(WebSocketHub::new());
        
        let drones = Arc::new(DashMap::new());
        for i in 1..=12 {
            let id = DroneId::new(format!("REAPER-{:02}", i));
            let drone = Drone::new(id.clone(), format!("Reaper {}", i));
            drones.insert(id, drone);
        }

        let mission = create_default_mission();
        let active_mission = Arc::new(RwLock::new(Some(mission)));
        let reset_flag = Arc::new(AtomicBool::new(false));

        Ok(Self {
            config,
            db: None,
            ws_hub,
            //cv_engine,
            drones,
            active_mission,
            reset_flag,
        })
    }

    /// Check if database is available
    pub fn has_db(&self) -> bool {
        self.db.is_some()
    }

    /// Check if CV engine is available
    // pub fn has_cv(&self) -> bool {
    //     self.cv_engine.is_some()
    // }

    /// Get drone by ID
    pub fn get_drone(&self, id: &DroneId) -> Option<Drone> {
        self.drones.get(id).map(|d| d.clone())
    }

    /// Update drone in cache
    pub fn update_drone(&self, drone: Drone) {
        self.drones.insert(drone.id.clone(), drone);
    }

    /// Get all drones
    pub fn get_all_drones(&self) -> Vec<Drone> {
        self.drones.iter().map(|r| r.value().clone()).collect()
    }

    /// Get active mission
    pub fn get_mission(&self) -> Option<Mission> {
        self.active_mission.read().clone()
    }

    /// Get connected WebSocket client count
    pub fn ws_client_count(&self) -> usize {
        self.ws_hub.client_count()
    }
}

/// Create default Afghanistan convoy mission
fn create_default_mission() -> Mission {
    let mut mission = Mission::new("Operation Desert Watch");
    mission.description = Some("Convoy escort mission across 12 strategic waypoints in Afghanistan".into());

    let waypoints = vec![
        ("WP01", "Base Alpha", 34.5553, 69.2075, WaypointType::Origin),
        ("WP02", "Checkpoint Bravo", 34.6234, 69.1123, WaypointType::Checkpoint),
        ("WP03", "Outpost Charlie", 34.7012, 69.0456, WaypointType::Standard),
        ("WP04", "Firebase Delta", 34.7891, 68.9234, WaypointType::Standard),
        ("WP05", "Sector Echo", 34.8567, 68.8012, WaypointType::Standard),
        ("WP06", "Point Foxtrot", 34.9234, 68.6789, WaypointType::Rally),
        ("WP07", "Zone Golf", 34.9901, 68.5567, WaypointType::Standard),
        ("WP08", "Camp Hotel", 35.0567, 68.4234, WaypointType::Standard),
        ("WP09", "Station India", 35.1234, 68.3012, WaypointType::Checkpoint),
        ("WP10", "Forward Juliet", 35.1901, 68.1789, WaypointType::Standard),
        ("WP11", "Base Kilo", 35.2567, 68.0567, WaypointType::Standard),
        ("WP12", "Terminal Lima", 35.3234, 67.9234, WaypointType::Destination),
    ];

    for (id, name, lat, lng, wp_type) in waypoints {
        let mut wp = Waypoint::new(id, name, lat, lng);
        wp.waypoint_type = wp_type;
        mission.add_waypoint(wp);
    }

    // Assign all 12 drones
    for i in 1..=12 {
        mission.assign_drone(DroneId::new(format!("REAPER-{:02}", i)));
    }

    mission
}
