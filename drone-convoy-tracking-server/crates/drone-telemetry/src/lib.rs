//! # Drone Telemetry - Metrics & Observability
//!
//! Prometheus metrics exporter for the drone convoy tracking system.
//! Provides real-time metrics for:
//! - Drone status and health
//! - System performance
//! - CV tracking statistics
//! - WebSocket connections

use drone_core::{Drone, DroneId, DroneStatus};
use parking_lot::RwLock;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec,
    IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::sync::Arc;
use tracing::{debug, info};

/// Metrics collector for the drone convoy system
pub struct MetricsCollector {
    registry: Registry,
    
    // Drone metrics
    drone_count: IntGauge,
    drone_status: IntGaugeVec,
    drone_battery: GaugeVec,
    drone_fuel: GaugeVec,
    drone_speed: GaugeVec,
    drone_altitude: GaugeVec,
    
    // Mission metrics
    mission_active: IntGauge,
    waypoints_reached: IntCounterVec,
    
    // CV tracking metrics
    cv_tracks_active: IntGauge,
    cv_frames_processed: IntCounter,
    cv_detections_total: IntCounter,
    cv_processing_time: Histogram,
    
    // WebSocket metrics
    ws_connections: IntGauge,
    ws_messages_sent: IntCounter,
    ws_messages_received: IntCounter,
    
    // Database metrics
    db_queries_total: IntCounterVec,
    db_query_duration: HistogramVec,
    db_connection_status: IntGauge,
    
    // System metrics
    api_requests_total: IntCounterVec,
    api_request_duration: HistogramVec,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> prometheus::Result<Self> {
        let registry = Registry::new();
        
        // Drone metrics
        let drone_count = IntGauge::new(
            "drone_convoy_drones_total",
            "Total number of drones in the system"
        )?;
        registry.register(Box::new(drone_count.clone()))?;

        let drone_status = IntGaugeVec::new(
            Opts::new("drone_convoy_drone_status", "Drone status by ID"),
            &["drone_id", "status"]
        )?;
        registry.register(Box::new(drone_status.clone()))?;

        let drone_battery = GaugeVec::new(
            Opts::new("drone_convoy_drone_battery_percent", "Drone battery level"),
            &["drone_id"]
        )?;
        registry.register(Box::new(drone_battery.clone()))?;

        let drone_fuel = GaugeVec::new(
            Opts::new("drone_convoy_drone_fuel_percent", "Drone fuel level"),
            &["drone_id"]
        )?;
        registry.register(Box::new(drone_fuel.clone()))?;

        let drone_speed = GaugeVec::new(
            Opts::new("drone_convoy_drone_speed_kmh", "Drone speed in km/h"),
            &["drone_id"]
        )?;
        registry.register(Box::new(drone_speed.clone()))?;

        let drone_altitude = GaugeVec::new(
            Opts::new("drone_convoy_drone_altitude_meters", "Drone altitude in meters"),
            &["drone_id"]
        )?;
        registry.register(Box::new(drone_altitude.clone()))?;

        // Mission metrics
        let mission_active = IntGauge::new(
            "drone_convoy_mission_active",
            "Whether a mission is currently active"
        )?;
        registry.register(Box::new(mission_active.clone()))?;

        let waypoints_reached = IntCounterVec::new(
            Opts::new("drone_convoy_waypoints_reached_total", "Waypoints reached by drones"),
            &["drone_id", "waypoint"]
        )?;
        registry.register(Box::new(waypoints_reached.clone()))?;

        // CV tracking metrics
        let cv_tracks_active = IntGauge::new(
            "drone_convoy_cv_tracks_active",
            "Number of active CV tracks"
        )?;
        registry.register(Box::new(cv_tracks_active.clone()))?;

        let cv_frames_processed = IntCounter::new(
            "drone_convoy_cv_frames_processed_total",
            "Total CV frames processed"
        )?;
        registry.register(Box::new(cv_frames_processed.clone()))?;

        let cv_detections_total = IntCounter::new(
            "drone_convoy_cv_detections_total",
            "Total halo detections"
        )?;
        registry.register(Box::new(cv_detections_total.clone()))?;

        let cv_processing_time = Histogram::with_opts(
            HistogramOpts::new(
                "drone_convoy_cv_processing_seconds",
                "CV frame processing time"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0])
        )?;
        registry.register(Box::new(cv_processing_time.clone()))?;

        // WebSocket metrics
        let ws_connections = IntGauge::new(
            "drone_convoy_ws_connections",
            "Active WebSocket connections"
        )?;
        registry.register(Box::new(ws_connections.clone()))?;

        let ws_messages_sent = IntCounter::new(
            "drone_convoy_ws_messages_sent_total",
            "Total WebSocket messages sent"
        )?;
        registry.register(Box::new(ws_messages_sent.clone()))?;

        let ws_messages_received = IntCounter::new(
            "drone_convoy_ws_messages_received_total",
            "Total WebSocket messages received"
        )?;
        registry.register(Box::new(ws_messages_received.clone()))?;

        // Database metrics
        let db_queries_total = IntCounterVec::new(
            Opts::new("drone_convoy_db_queries_total", "Database queries"),
            &["table", "operation"]
        )?;
        registry.register(Box::new(db_queries_total.clone()))?;

        let db_query_duration = HistogramVec::new(
            HistogramOpts::new(
                "drone_convoy_db_query_duration_seconds",
                "Database query duration"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["table", "operation"]
        )?;
        registry.register(Box::new(db_query_duration.clone()))?;

        let db_connection_status = IntGauge::new(
            "drone_convoy_db_connected",
            "Database connection status"
        )?;
        registry.register(Box::new(db_connection_status.clone()))?;

        // API metrics
        let api_requests_total = IntCounterVec::new(
            Opts::new("drone_convoy_api_requests_total", "API requests"),
            &["method", "path", "status"]
        )?;
        registry.register(Box::new(api_requests_total.clone()))?;

        let api_request_duration = HistogramVec::new(
            HistogramOpts::new(
                "drone_convoy_api_request_duration_seconds",
                "API request duration"
            ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
            &["method", "path"]
        )?;
        registry.register(Box::new(api_request_duration.clone()))?;

        info!("📊 Metrics collector initialized");

        Ok(Self {
            registry,
            drone_count,
            drone_status,
            drone_battery,
            drone_fuel,
            drone_speed,
            drone_altitude,
            mission_active,
            waypoints_reached,
            cv_tracks_active,
            cv_frames_processed,
            cv_detections_total,
            cv_processing_time,
            ws_connections,
            ws_messages_sent,
            ws_messages_received,
            db_queries_total,
            db_query_duration,
            db_connection_status,
            api_requests_total,
            api_request_duration,
        })
    }

    /// Get Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> String {
        use prometheus::Encoder;
        
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    // ========================================================================
    // DRONE METRICS
    // ========================================================================

    /// Update drone count
    pub fn set_drone_count(&self, count: i64) {
        self.drone_count.set(count);
    }

    /// Update drone telemetry
    pub fn update_drone(&self, drone: &Drone) {
        let id = drone.id.as_str();
        
        self.drone_battery
            .with_label_values(&[id])
            .set(drone.telemetry.battery_level as f64);
        
        self.drone_fuel
            .with_label_values(&[id])
            .set(drone.telemetry.fuel_level as f64);
        
        self.drone_speed
            .with_label_values(&[id])
            .set(drone.telemetry.speed);
        
        self.drone_altitude
            .with_label_values(&[id])
            .set(drone.position.altitude);
    }

    /// Update drone status
    pub fn set_drone_status(&self, drone_id: &str, status: &DroneStatus) {
        let status_str = format!("{}", status);
        self.drone_status
            .with_label_values(&[drone_id, &status_str])
            .set(1);
    }

    // ========================================================================
    // MISSION METRICS
    // ========================================================================

    /// Set mission active status
    pub fn set_mission_active(&self, active: bool) {
        self.mission_active.set(if active { 1 } else { 0 });
    }

    /// Record waypoint reached
    pub fn record_waypoint_reached(&self, drone_id: &str, waypoint: &str) {
        self.waypoints_reached
            .with_label_values(&[drone_id, waypoint])
            .inc();
    }

    // ========================================================================
    // CV METRICS
    // ========================================================================

    /// Set active CV track count
    pub fn set_cv_tracks(&self, count: i64) {
        self.cv_tracks_active.set(count);
    }

    /// Record CV frame processed
    pub fn record_cv_frame(&self, processing_time_secs: f64, detections: u64) {
        self.cv_frames_processed.inc();
        self.cv_detections_total.inc_by(detections);
        self.cv_processing_time.observe(processing_time_secs);
    }

    // ========================================================================
    // WEBSOCKET METRICS
    // ========================================================================

    /// Set WebSocket connection count
    pub fn set_ws_connections(&self, count: i64) {
        self.ws_connections.set(count);
    }

    /// Record WebSocket message sent
    pub fn record_ws_sent(&self) {
        self.ws_messages_sent.inc();
    }

    /// Record WebSocket message received
    pub fn record_ws_received(&self) {
        self.ws_messages_received.inc();
    }

    // ========================================================================
    // DATABASE METRICS
    // ========================================================================

    /// Set database connection status
    pub fn set_db_connected(&self, connected: bool) {
        self.db_connection_status.set(if connected { 1 } else { 0 });
    }

    /// Record database query
    pub fn record_db_query(&self, table: &str, operation: &str, duration_secs: f64) {
        self.db_queries_total
            .with_label_values(&[table, operation])
            .inc();
        self.db_query_duration
            .with_label_values(&[table, operation])
            .observe(duration_secs);
    }

    // ========================================================================
    // API METRICS
    // ========================================================================

    /// Record API request
    pub fn record_api_request(
        &self,
        method: &str,
        path: &str,
        status: u16,
        duration_secs: f64,
    ) {
        self.api_requests_total
            .with_label_values(&[method, path, &status.to_string()])
            .inc();
        self.api_request_duration
            .with_label_values(&[method, path])
            .observe(duration_secs);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create MetricsCollector")
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = MetricsCollector::new();
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_metrics_export() {
        let metrics = MetricsCollector::new().unwrap();
        
        metrics.set_drone_count(12);
        metrics.set_ws_connections(5);
        metrics.set_mission_active(true);
        
        let export = metrics.export();
        assert!(export.contains("drone_convoy_drones_total"));
        assert!(export.contains("drone_convoy_ws_connections"));
        assert!(export.contains("drone_convoy_mission_active"));
    }

    #[test]
    fn test_drone_metrics() {
        let metrics = MetricsCollector::new().unwrap();
        
        let drone = Drone::new(DroneId::new("REAPER-01"), "Alpha Lead");
        metrics.update_drone(&drone);
        
        let export = metrics.export();
        assert!(export.contains("REAPER-01"));
    }
}
