//! Tracking engine core logic

use crate::{TrackedDrone, TrackerConfig};
use drone_core::{DroneId, Event, GeoPosition, Telemetry};

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Tracking engine that processes updates
pub struct TrackingEngine {
    config: TrackerConfig,
    /// Last update timestamp per drone
    last_updates: Arc<RwLock<std::collections::HashMap<DroneId, Instant>>>,
    /// Event sender
    event_tx: broadcast::Sender<Event>,
    /// Statistics
    stats: Arc<RwLock<EngineStats>>,
}

/// Engine statistics
#[derive(Debug, Default, Clone)]
pub struct EngineStats {
    pub updates_processed: u64,
    pub events_emitted: u64,
    pub waypoints_detected: u64,
    pub alerts_generated: u64,
    pub uptime_seconds: u64,
}

impl TrackingEngine {
    /// Create a new tracking engine
    pub fn new(config: TrackerConfig, event_tx: broadcast::Sender<Event>) -> Self {
        Self {
            config,
            last_updates: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_tx,
            stats: Arc::new(RwLock::new(EngineStats::default())),
        }
    }

    /// Process a position update
    pub fn process_update(
        &self,
        drone_id: &DroneId,
        position: GeoPosition,
        telemetry: Telemetry,
    ) {
        // Record update time
        self.last_updates.write().insert(drone_id.clone(), Instant::now());
        
        // Update statistics
        self.stats.write().updates_processed += 1;

        debug!("Processed update for drone {}", drone_id);
    }

    /// Emit an event
    pub fn emit_event(&self, event: Event) {
        let _ = self.event_tx.send(event);
        self.stats.write().events_emitted += 1;
    }

    /// Check for stale drones
    pub fn check_stale_drones(&self, timeout: Duration) -> Vec<DroneId> {
        let now = Instant::now();
        let updates = self.last_updates.read();
        
        updates
            .iter()
            .filter(|(_, last)| now.duration_since(**last) > timeout)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get engine statistics
    pub fn get_stats(&self) -> EngineStats {
        self.stats.read().clone()
    }

    /// Record waypoint detection
    pub fn record_waypoint(&self) {
        self.stats.write().waypoints_detected += 1;
    }

    /// Record alert generation
    pub fn record_alert(&self) {
        self.stats.write().alerts_generated += 1;
    }

    /// Get update rate for a drone (updates per second)
    pub fn get_update_rate(&self, drone_id: &DroneId) -> Option<f64> {
        // Simplified - in real implementation would track update frequency
        Some(10.0) // Assume 10 Hz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TrackingEngine::new(TrackerConfig::default(), tx);
        
        let stats = engine.get_stats();
        assert_eq!(stats.updates_processed, 0);
    }

    #[test]
    fn test_update_processing() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TrackingEngine::new(TrackerConfig::default(), tx);
        
        let drone_id = DroneId::new("REAPER-01");
        let position = GeoPosition::new(34.5553, 69.2075, 3000.0);
        let telemetry = Telemetry::default();
        
        engine.process_update(&drone_id, position, telemetry);
        
        let stats = engine.get_stats();
        assert_eq!(stats.updates_processed, 1);
    }

    #[test]
    fn test_stale_detection() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TrackingEngine::new(TrackerConfig::default(), tx);
        
        let drone_id = DroneId::new("REAPER-01");
        engine.last_updates.write().insert(
            drone_id.clone(),
            Instant::now() - Duration::from_secs(60),
        );
        
        let stale = engine.check_stale_drones(Duration::from_secs(30));
        assert!(stale.contains(&drone_id));
    }
}
