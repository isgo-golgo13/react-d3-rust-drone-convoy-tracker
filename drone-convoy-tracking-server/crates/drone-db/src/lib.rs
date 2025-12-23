//! # Drone DB - ScyllaDB Integration
//!
//! Provides persistence layer for drone telemetry, waypoint events,
//! CV tracking results, and mission data using ScyllaDB.

pub mod error;
pub mod repository;
pub mod migrations;

pub use error::{DbError, DbResult};
pub use repository::*;

use drone_core::{
    Alert, Drone, DroneId, GeoPosition, Mission, MissionId, Telemetry, 
    TrackingResult, WaypointId,
};
use scylla::{Session, SessionBuilder};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use serde::{Deserialize, Serialize};

/// Database configuration
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct DbConfig {
//     pub hosts: Vec<String>,
//     pub keyspace: String,
//     #[serde(with = "humantime_serde", default = "default_connection_timeout")]
//     pub connection_timeout: Duration,
//     #[serde(with = "humantime_serde", default = "default_query_timeout")]
//     pub query_timeout: Duration,
//     #[serde(default)]
//     pub ssl_enabled: bool,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    pub hosts: Vec<String>,
    pub keyspace: String,
    #[serde(skip)]
    pub connection_timeout: Duration,
    #[serde(skip)]
    pub query_timeout: Duration,
    #[serde(default)]
    pub ssl_enabled: bool,
}

fn default_connection_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_query_timeout() -> Duration {
    Duration::from_secs(5)
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
    pub async fn new(config: DbConfig) -> DbResult<Self> {
        info!("Connecting to ScyllaDB cluster: {:?}", config.hosts);

        let session = SessionBuilder::new()
            .known_nodes(&config.hosts)
            .connection_timeout(config.connection_timeout)
            .use_keyspace(&config.keyspace, false)
            .build()
            .await
            .map_err(|e| DbError::Connection(e.to_string()))?;

        let session = Arc::new(session);
        info!("Connected to ScyllaDB");

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

    pub fn session(&self) -> Arc<Session> {
        self.session.clone()
    }

    pub fn telemetry(&self) -> &TelemetryRepository {
        &self.telemetry_repo
    }

    pub fn waypoints(&self) -> &WaypointRepository {
        &self.waypoint_repo
    }

    pub fn tracking(&self) -> &TrackingRepository {
        &self.tracking_repo
    }

    pub fn missions(&self) -> &MissionRepository {
        &self.mission_repo
    }

    pub fn drones(&self) -> &DroneRepository {
        &self.drone_repo
    }

    pub fn alerts(&self) -> &AlertRepository {
        &self.alert_repo
    }

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

    pub async fn insert(
        &self,
        drone_id: &DroneId,
        position: &GeoPosition,
        telemetry: &Telemetry,
        mission_id: Option<&MissionId>,
    ) -> DbResult<()> {
        let query = r#"
            INSERT INTO drone_telemetry (
                drone_id, timestamp, latitude, longitude, altitude,
                heading, speed, battery_level, fuel_level, system_health,
                status, armed, temperature, signal_strength, mission_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let timestamp_ms = telemetry.timestamp.timestamp_millis();
        let mission_uuid = mission_id.map(|m| m.0);

        self.session
            .query_unpaged(
                query,
                (
                    drone_id.as_str(),
                    timestamp_ms,
                    position.latitude,
                    position.longitude,
                    position.altitude,
                    telemetry.heading,
                    telemetry.speed,
                    telemetry.battery_level as i32,
                    telemetry.fuel_level as i32,
                    telemetry.system_health as i32,
                    "MOVING",
                    false,
                    telemetry.temperature,
                    telemetry.signal_strength as i32,
                    mission_uuid,
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

    pub async fn get_latest(
        &self,
        drone_id: &DroneId,
    ) -> DbResult<Option<(GeoPosition, Telemetry)>> {
        let query = r#"
            SELECT latitude, longitude, altitude, heading, speed,
                   battery_level, fuel_level, system_health, temperature,
                   signal_strength, timestamp
            FROM drone_telemetry
            WHERE drone_id = ?
            LIMIT 1
        "#;

        let result = self
            .session
            .query_unpaged(query, (drone_id.as_str(),))
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        // scylla 0.15 API - rows_result is not Option
        let rows_result = result.into_rows_result().map_err(|e| DbError::Query(e.to_string()))?;
        
        if rows_result.rows_num() > 0 {
            // TODO: implement proper row parsing
            return Ok(None);
        }

        Ok(None)
    }


    pub async fn get_history(
        &self,
        drone_id: &DroneId,
        _limit: i32,
    ) -> DbResult<Vec<(GeoPosition, Telemetry)>> {
        let _query = r#"
            SELECT latitude, longitude, altitude, heading, speed,
                   battery_level, fuel_level, system_health, temperature,
                   signal_strength, timestamp
            FROM drone_telemetry
            WHERE drone_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
        "#;

        // TODO: implement with proper row parsing
        let _ = drone_id; // suppress warning
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

    pub async fn record_reached(
        &self,
        drone_id: &DroneId,
        waypoint_id: &WaypointId,
        mission_id: &MissionId,
        position: &GeoPosition,
    ) -> DbResult<()> {
        let query = r#"
            INSERT INTO waypoint_events (
                drone_id, waypoint_id, mission_id, event_type,
                timestamp, latitude, longitude, altitude
            ) VALUES (?, ?, ?, 'REACHED', toTimestamp(now()), ?, ?, ?)
        "#;

        self.session
            .query_unpaged(
                query,
                (
                    drone_id.as_str(),
                    waypoint_id.0.as_str(),
                    mission_id.0,
                    position.latitude,
                    position.longitude,
                    position.altitude,
                ),
            )
            .await
            .map_err(|e: scylla::transport::errors::QueryError| DbError::Query(e.to_string()))?;

        Ok(())
    }
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
    /// NOTE: Commented out for macOS build (requires OpenCV/Xcode 15)
    /// Uncomment for Linux builds with OpenCV support
    pub async fn insert(&self, _result: &TrackingResult) -> DbResult<()> {
        // TODO: Re-enable for Linux builds with OpenCV
        /*
        // Split into two queries to avoid 16-tuple limit
        let query1 = r#"
            INSERT INTO cv_tracking (
                drone_id, frame_timestamp, bbox_x, bbox_y, bbox_width, bbox_height,
                tracking_id, confidence, halo_detected
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let timestamp_ms = _result.frame_timestamp.timestamp_millis();
        let halo_detected = _result.halo.is_some();

        self.session
            .query_unpaged(
                query1,
                (
                    _result.drone_id.as_str(),
                    timestamp_ms,
                    _result.bbox.x,
                    _result.bbox.y,
                    _result.bbox.width,
                    _result.bbox.height,
                    _result.tracking_id as i32,
                    _result.confidence,
                    halo_detected,
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        // Update halo data if present
        if let Some(halo) = &_result.halo {
            let query2 = r#"
                UPDATE cv_tracking SET
                    halo_center_x = ?, halo_center_y = ?, halo_radius = ?,
                    halo_color_r = ?, halo_color_g = ?, halo_color_b = ?
                WHERE drone_id = ? AND frame_timestamp = ?
            "#;

            self.session
                .query_unpaged(
                    query2,
                    (
                        halo.center_x,
                        halo.center_y,
                        halo.radius,
                        halo.color.r as i32,
                        halo.color.g as i32,
                        halo.color.b as i32,
                        _result.drone_id.as_str(),
                        timestamp_ms,
                    ),
                )
                .await
                .map_err(|e| DbError::Query(e.to_string()))?;
        }

        // Update estimated position if present
        if let Some(pos) = &_result.estimated_position {
            let query3 = r#"
                UPDATE cv_tracking SET est_latitude = ?, est_longitude = ?
                WHERE drone_id = ? AND frame_timestamp = ?
            "#;

            self.session
                .query_unpaged(
                    query3,
                    (pos.latitude, pos.longitude, _result.drone_id.as_str(), timestamp_ms),
                )
                .await
                .map_err(|e| DbError::Query(e.to_string()))?;
        }
        */

        Ok(()) // Stubbed for macOS
    }

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

    pub async fn create(&self, mission: &Mission) -> DbResult<()> {
        let query = r#"
            INSERT INTO missions (
                mission_id, created_at, name, description, status,
                start_time, end_time, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let created_at_ms = mission.created_at.timestamp_millis();
        let start_time_ms = mission.start_time.map(|t| t.timestamp_millis());
        let end_time_ms = mission.end_time.map(|t| t.timestamp_millis());
        let updated_at_ms = mission.updated_at.timestamp_millis();

        self.session
            .query_unpaged(
                query,
                (
                    mission.id.0,
                    created_at_ms,
                    mission.name.as_str(),
                    mission.description.as_deref(),
                    format!("{:?}", mission.status),
                    start_time_ms,
                    end_time_ms,
                    updated_at_ms,
                ),
            )
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        Ok(())
    }

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

    pub async fn get(&self, mission_id: &MissionId) -> DbResult<Option<Mission>> {
        let query = "SELECT * FROM missions WHERE mission_id = ?";

        let _result = self
            .session
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

    pub async fn get_all(&self) -> DbResult<Vec<DroneId>> {
        let query =
            "SELECT drone_id FROM drone_registry WHERE operational = true ALLOW FILTERING";

        let _result = self
            .session
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

    pub async fn acknowledge(
        &self,
        drone_id: &DroneId,
        alert_id: uuid::Uuid,
        by: &str,
    ) -> DbResult<()> {
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