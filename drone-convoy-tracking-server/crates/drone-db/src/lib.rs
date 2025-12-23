//! # Drone DB - ScyllaDB Integration
//!
//! Provides persistence layer for drone telemetry, waypoint events,
//! CV tracking results, and mission data using ScyllaDB.
//!
//! ## Features
//! - Time-series telemetry storage with TTL
//! - Waypoint event recording
//! - CV tracking result persistence
//! - Mission state management
//! - High-availability with 3-node cluster support

pub mod error;
pub mod repository;
pub mod migrations;

pub use error::{DbError, DbResult};
pub use repository::*;

use drone_core::{
    Alert, Drone, DroneId, GeoPosition, Mission, MissionId, Telemetry, 
    TrackingResult, Waypoint, WaypointId,
};
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// ScyllaDB contact points
    pub hosts: Vec<String>,
    /// Keyspace name
    pub keyspace: String,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Query timeout
    pub query_timeout: Duration,
    /// Enable SSL
    pub ssl_enabled: bool,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            hosts: vec!["127.0.0.1:9042".to_string()],
            keyspace: "drone_convoy".to_string(),
            connection_timeout: Duration::from_secs(10),
            query_timeout: Duration::from_secs(5),
            ssl_enabled: false,
        }
    }
}

impl DbConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let hosts = std::env::var("SCYLLA_HOSTS")
            .unwrap_or_else(|_| "127.0.0.1:9042".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let keyspace = std::env::var("SCYLLA_KEYSPACE")
            .unwrap_or_else(|_| "drone_convoy".to_string());

        Self {
            hosts,
            keyspace,
            ..Default::default()
        }
    }

    /// Create config for Docker Compose environment
    pub fn docker() -> Self {
        Self {
            hosts: vec![
                "scylla-node1:9042".to_string(),
                "scylla-node2:9042".to_string(),
                "scylla-node3:9042".to_string(),
            ],
            keyspace: "drone_convoy".to_string(),
            ..Default::default()
        }
    }
}

/// Main database client
pub struct DbClient {
    session: Arc<Session>,
    config: DbConfig,
    telemetry_repo: TelemetryRepository,
    waypoint_repo: WaypointRepository,
    tracking_repo: TrackingRepository,
    mission_repo: MissionRepository,
    drone_repo: DroneRepository,
    alert_repo: AlertRepository,
}

impl DbClient {
    /// Create a new database client
    pub async fn new(config: DbConfig) -> DbResult<Self> {
        info!("🗄️ Connecting to ScyllaDB cluster: {:?}", config.hosts);

        let session = SessionBuilder::new()
            .known_nodes(&config.hosts)
            .connection_timeout(config.connection_timeout)
            .use_keyspace(&config.keyspace, false)
            .build()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        let session = Arc::new(session);
        info!("✅ Connected to ScyllaDB");

        Ok(Self {
            telemetry_repo: TelemetryRepository::new(session.clone()),
            waypoint_repo: WaypointRepository::new(session.clone()),
            tracking_repo: TrackingRepository::new(session.clone()),
            mission_repo: MissionRepository::new(session.clone()),
            drone_repo: DroneRepository::new(session.clone()),
            alert_repo: AlertRepository::new(session.clone()),
            session,
            config,
        })
    }

    /// Get raw session for custom queries
    pub fn session(&self) -> Arc<Session> {
        self.session.clone()
    }

    /// Get telemetry repository
    pub fn telemetry(&self) -> &TelemetryRepository {
        &self.telemetry_repo
    }

    /// Get waypoint repository
    pub fn waypoints(&self) -> &WaypointRepository {
        &self.waypoint_repo
    }

    /// Get tracking repository
    pub fn tracking(&self) -> &TrackingRepository {
        &self.tracking_repo
    }

    /// Get mission repository
    pub fn missions(&self) -> &MissionRepository {
        &self.mission_repo
    }

    /// Get drone repository
    pub fn drones(&self) -> &DroneRepository {
        &self.drone_repo
    }

    /// Get alert repository
    pub fn alerts(&self) -> &AlertRepository {
        &self.alert_repo
    }

    /// Health check
    pub async fn health_check(&self) -> DbResult<bool> {
        let result = self.session
            .query_unpaged("SELECT now() FROM system.local", &[])
            .await;
        
        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Database health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Run migrations
    pub async fn run_migrations(&self) -> DbResult<()> {
        migrations::run_all(&self.session).await
    }
}

// ============================================================================
// REPOSITORY IMPLEMENTATIONS
// ============================================================================

/// Repository for drone telemetry data
#[derive(Clone)]
pub struct TelemetryRepository {
    session: Arc<Session>,
}

impl TelemetryRepository {
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }

    /// Insert telemetry record
    pub async fn insert(&self, drone_id: &DroneId, position: &GeoPosition, telemetry: &Telemetry, mission_id: Option<&MissionId>) -> DbResult<()> {
        let query = r#"
            INSERT INTO drone_telemetry (
                drone_id, timestamp, latitude, longitude, altitude,
                heading, speed, battery_level, fuel_level, system_health,
                status, armed, temperature, signal_strength, mission_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        self.session
            .query_unpaged(
                query,
                (
                    drone_id.as_str(),
                    telemetry.timestamp,
                    position.latitude,
                    position.longitude,
                    position.altitude,
                    telemetry.heading,
                    telemetry.speed,
                    telemetry.battery_level as i32,
                    telemetry.fuel_level as i32,
                    telemetry.system_health as i32,
                    "MOVING", // TODO: pass actual status
                    false,    // TODO: pass armed state
                    telemetry.temperature,
                    telemetry.signal_strength as i32,
                    mission_id.map(|m| m.0),
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    /// Get latest telemetry for a drone
    pub async fn get_latest(&self, drone_id: &DroneId) -> DbResult<Option<(GeoPosition, Telemetry)>> {
        let query = r#"
            SELECT latitude, longitude, altitude, heading, speed,
                   battery_level, fuel_level, system_health, temperature,
                   signal_strength, timestamp
            FROM drone_telemetry
            WHERE drone_id = ?
            LIMIT 1
        "#;

        let result = self.session
            .query_unpaged(query, (drone_id.as_str(),))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        if let Some(rows) = result.rows {
            if let Some(row) = rows.into_iter().next() {
                // Parse row into position and telemetry
                // Simplified - in production use proper row parsing
                return Ok(None); // TODO: implement row parsing
            }
        }

        Ok(None)
    }

    /// Get telemetry history for a drone
    pub async fn get_history(
        &self,
        drone_id: &DroneId,
        limit: i32,
    ) -> DbResult<Vec<(GeoPosition, Telemetry)>> {
        let query = r#"
            SELECT latitude, longitude, altitude, heading, speed,
                   battery_level, fuel_level, system_health, temperature,
                   signal_strength, timestamp
            FROM drone_telemetry
            WHERE drone_id = ?
            LIMIT ?
        "#;

        let _result = self.session
            .query_unpaged(query, (drone_id.as_str(), limit))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        // TODO: Parse rows
        Ok(Vec::new())
    }
}

/// Repository for waypoint events
#[derive(Clone)]
pub struct WaypointRepository {
    session: Arc<Session>,
}

impl WaypointRepository {
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }

    /// Record waypoint arrival
    pub async fn record_arrival(
        &self,
        mission_id: &MissionId,
        drone_id: &DroneId,
        waypoint: &Waypoint,
        speed: f64,
        altitude: f64,
        heading: f64,
    ) -> DbResult<()> {
        let query = r#"
            INSERT INTO waypoint_events (
                mission_id, event_time, drone_id, waypoint_id, waypoint_name,
                latitude, longitude, event_type, speed_at_event, 
                altitude_at_event, heading
            ) VALUES (?, toTimestamp(now()), ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        self.session
            .query_unpaged(
                query,
                (
                    mission_id.0,
                    drone_id.as_str(),
                    waypoint.id.0.as_str(),
                    waypoint.name.as_str(),
                    waypoint.position.latitude,
                    waypoint.position.longitude,
                    "ARRIVAL",
                    speed,
                    altitude,
                    heading,
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    /// Get waypoint events for a mission
    pub async fn get_mission_events(&self, mission_id: &MissionId) -> DbResult<Vec<WaypointEvent>> {
        let query = r#"
            SELECT drone_id, waypoint_id, waypoint_name, event_type, event_time
            FROM waypoint_events
            WHERE mission_id = ?
        "#;

        let _result = self.session
            .query_unpaged(query, (mission_id.0,))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        // TODO: Parse rows
        Ok(Vec::new())
    }
}

/// Waypoint event record
#[derive(Debug, Clone)]
pub struct WaypointEvent {
    pub drone_id: DroneId,
    pub waypoint_id: WaypointId,
    pub waypoint_name: String,
    pub event_type: String,
    pub event_time: chrono::DateTime<chrono::Utc>,
}

/// Repository for CV tracking results
#[derive(Clone)]
pub struct TrackingRepository {
    session: Arc<Session>,
}

impl TrackingRepository {
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }

    /// Insert CV tracking result
    pub async fn insert(&self, result: &TrackingResult) -> DbResult<()> {
        let query = r#"
            INSERT INTO cv_tracking (
                drone_id, frame_timestamp, bbox_x, bbox_y, bbox_width, bbox_height,
                tracking_id, confidence, halo_detected, halo_center_x, halo_center_y,
                halo_radius, halo_color_r, halo_color_g, halo_color_b,
                est_latitude, est_longitude
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let (halo_detected, halo_x, halo_y, halo_r, color_r, color_g, color_b) = 
            if let Some(halo) = &result.halo {
                (true, halo.center_x, halo.center_y, halo.radius, 
                 halo.color.r as i32, halo.color.g as i32, halo.color.b as i32)
            } else {
                (false, 0, 0, 0, 0, 0, 0)
            };

        let (est_lat, est_lng) = result.estimated_position
            .map(|p| (Some(p.latitude), Some(p.longitude)))
            .unwrap_or((None, None));

        self.session
            .query_unpaged(
                query,
                (
                    result.drone_id.as_str(),
                    result.frame_timestamp,
                    result.bbox.x,
                    result.bbox.y,
                    result.bbox.width,
                    result.bbox.height,
                    result.tracking_id as i32,
                    result.confidence,
                    halo_detected,
                    halo_x,
                    halo_y,
                    halo_r,
                    color_r,
                    color_g,
                    color_b,
                    est_lat,
                    est_lng,
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    /// Batch insert tracking results
    pub async fn insert_batch(&self, results: &[TrackingResult]) -> DbResult<()> {
        for result in results {
            self.insert(result).await?;
        }
        Ok(())
    }
}

/// Repository for missions
#[derive(Clone)]
pub struct MissionRepository {
    session: Arc<Session>,
}

impl MissionRepository {
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }

    /// Create a new mission
    pub async fn create(&self, mission: &Mission) -> DbResult<()> {
        let query = r#"
            INSERT INTO missions (
                mission_id, created_at, name, description, status,
                start_time, end_time, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        self.session
            .query_unpaged(
                query,
                (
                    mission.id.0,
                    mission.created_at,
                    mission.name.as_str(),
                    mission.description.as_deref(),
                    format!("{:?}", mission.status),
                    mission.start_time,
                    mission.end_time,
                    mission.updated_at,
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    /// Update mission status
    pub async fn update_status(&self, mission_id: &MissionId, status: &str) -> DbResult<()> {
        let query = r#"
            UPDATE missions SET status = ?, updated_at = toTimestamp(now())
            WHERE mission_id = ?
        "#;

        self.session
            .query_unpaged(query, (status, mission_id.0))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    /// Get mission by ID
    pub async fn get(&self, mission_id: &MissionId) -> DbResult<Option<Mission>> {
        let query = "SELECT * FROM missions WHERE mission_id = ?";
        
        let _result = self.session
            .query_unpaged(query, (mission_id.0,))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        // TODO: Parse row into Mission
        Ok(None)
    }
}

/// Repository for drone registry
#[derive(Clone)]
pub struct DroneRepository {
    session: Arc<Session>,
}

impl DroneRepository {
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }

    /// Register a drone
    pub async fn register(&self, drone: &Drone) -> DbResult<()> {
        let query = r#"
            INSERT INTO drone_registry (
                drone_id, callsign, drone_type, operational, registered_at, updated_at
            ) VALUES (?, ?, ?, ?, toTimestamp(now()), toTimestamp(now()))
        "#;

        self.session
            .query_unpaged(
                query,
                (
                    drone.id.as_str(),
                    drone.callsign.as_str(),
                    format!("{:?}", drone.drone_type),
                    true,
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    /// Get all registered drones
    pub async fn get_all(&self) -> DbResult<Vec<DroneId>> {
        let query = "SELECT drone_id FROM drone_registry WHERE operational = true ALLOW FILTERING";
        
        let _result = self.session
            .query_unpaged(query, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        // TODO: Parse rows
        Ok(Vec::new())
    }
}

/// Repository for alerts
#[derive(Clone)]
pub struct AlertRepository {
    session: Arc<Session>,
}

impl AlertRepository {
    pub fn new(session: Arc<Session>) -> Self {
        Self { session }
    }

    /// Create an alert
    pub async fn create(&self, alert: &Alert) -> DbResult<()> {
        let drone_id = alert.drone_id.as_ref().map(|d| d.as_str());
        
        let query = r#"
            INSERT INTO alerts (
                alert_id, created_at, severity, alert_type, message,
                drone_id, acknowledged, resolved
            ) VALUES (?, toTimestamp(now()), ?, ?, ?, ?, false, false)
        "#;

        self.session
            .query_unpaged(
                query,
                (
                    alert.id,
                    format!("{:?}", alert.severity),
                    format!("{:?}", alert.alert_type),
                    alert.message.as_str(),
                    drone_id.unwrap_or(""),
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    /// Acknowledge an alert
    pub async fn acknowledge(&self, drone_id: &DroneId, alert_id: uuid::Uuid, by: &str) -> DbResult<()> {
        let query = r#"
            UPDATE alerts SET acknowledged = true, acknowledged_by = ?, 
                             acknowledged_at = toTimestamp(now())
            WHERE drone_id = ? AND alert_id = ?
        "#;

        self.session
            .query_unpaged(query, (by, drone_id.as_str(), alert_id))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_config_default() {
        let config = DbConfig::default();
        assert_eq!(config.hosts.len(), 1);
        assert_eq!(config.keyspace, "drone_convoy");
    }

    #[test]
    fn test_db_config_docker() {
        let config = DbConfig::docker();
        assert_eq!(config.hosts.len(), 3);
        assert!(config.hosts[0].contains("scylla-node1"));
    }
}
