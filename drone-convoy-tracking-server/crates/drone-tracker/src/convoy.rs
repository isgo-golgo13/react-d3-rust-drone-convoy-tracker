//! Convoy formation management

use drone_core::{Drone, DroneId, GeoPosition, Mission};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Convoy formation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formation {
    /// Single file line
    Line,
    /// V-shape formation
    Vee,
    /// Diamond formation
    Diamond,
    /// Echelon (diagonal) formation
    Echelon,
    /// Column formation
    Column,
    /// Spread formation
    Spread,
}

impl Default for Formation {
    fn default() -> Self {
        Self::Line
    }
}

/// Convoy manager
pub struct ConvoyManager {
    /// Current formation
    formation: Arc<RwLock<Formation>>,
    /// Leader drone
    leader: Arc<RwLock<Option<DroneId>>>,
    /// Drone order in convoy
    order: Arc<RwLock<Vec<DroneId>>>,
    /// Formation offsets (relative to leader)
    offsets: Arc<RwLock<HashMap<DroneId, FormationOffset>>>,
    /// Spacing between drones (meters)
    spacing: f64,
}

/// Offset from leader position
#[derive(Debug, Clone, Copy)]
pub struct FormationOffset {
    /// Lateral offset (positive = right)
    pub lateral: f64,
    /// Longitudinal offset (positive = behind)
    pub longitudinal: f64,
    /// Vertical offset (positive = above)
    pub vertical: f64,
}

impl ConvoyManager {
    /// Create a new convoy manager
    pub fn new() -> Self {
        Self {
            formation: Arc::new(RwLock::new(Formation::default())),
            leader: Arc::new(RwLock::new(None)),
            order: Arc::new(RwLock::new(Vec::new())),
            offsets: Arc::new(RwLock::new(HashMap::new())),
            spacing: 50.0, // 50 meters default spacing
        }
    }

    /// Set convoy formation
    pub fn set_formation(&self, formation: Formation) {
        *self.formation.write() = formation;
        self.recalculate_offsets();
        info!("Convoy formation changed to {:?}", formation);
    }

    /// Get current formation
    pub fn get_formation(&self) -> Formation {
        *self.formation.read()
    }

    /// Set convoy leader
    pub fn set_leader(&self, drone_id: DroneId) {
        *self.leader.write() = Some(drone_id.clone());
        info!("Convoy leader set to {}", drone_id);
    }

    /// Get convoy leader
    pub fn get_leader(&self) -> Option<DroneId> {
        self.leader.read().clone()
    }

    /// Set drone order in convoy
    pub fn set_order(&self, order: Vec<DroneId>) {
        *self.order.write() = order;
        self.recalculate_offsets();
    }

    /// Get drone order
    pub fn get_order(&self) -> Vec<DroneId> {
        self.order.read().clone()
    }

    /// Set spacing between drones
    pub fn set_spacing(&self, meters: f64) {
        // self.spacing = meters;
        self.recalculate_offsets();
    }

    /// Recalculate formation offsets based on current formation
    fn recalculate_offsets(&self) {
        let formation = *self.formation.read();
        let order = self.order.read().clone();
        let mut offsets = self.offsets.write();
        offsets.clear();

        for (i, drone_id) in order.iter().enumerate() {
            if i == 0 {
                // Leader has no offset
                offsets.insert(drone_id.clone(), FormationOffset {
                    lateral: 0.0,
                    longitudinal: 0.0,
                    vertical: 0.0,
                });
                continue;
            }

            let offset = match formation {
                Formation::Line => FormationOffset {
                    lateral: 0.0,
                    longitudinal: self.spacing * i as f64,
                    vertical: 0.0,
                },
                Formation::Vee => {
                    let side = if i % 2 == 1 { 1.0 } else { -1.0 };
                    let row = ((i + 1) / 2) as f64;
                    FormationOffset {
                        lateral: side * self.spacing * row * 0.7,
                        longitudinal: self.spacing * row,
                        vertical: 0.0,
                    }
                },
                Formation::Diamond => {
                    let angle = (i as f64 - 1.0) * (std::f64::consts::PI * 2.0 / 4.0);
                    FormationOffset {
                        lateral: self.spacing * angle.sin(),
                        longitudinal: self.spacing * angle.cos(),
                        vertical: 0.0,
                    }
                },
                Formation::Echelon => FormationOffset {
                    lateral: self.spacing * i as f64 * 0.5,
                    longitudinal: self.spacing * i as f64,
                    vertical: 0.0,
                },
                Formation::Column => FormationOffset {
                    lateral: 0.0,
                    longitudinal: self.spacing * i as f64,
                    vertical: 0.0,
                },
                Formation::Spread => FormationOffset {
                    lateral: self.spacing * (i as f64 - (order.len() as f64 / 2.0)),
                    longitudinal: 0.0,
                    vertical: 0.0,
                },
            };

            offsets.insert(drone_id.clone(), offset);
        }
    }

    /// Get target position for a drone based on leader position
    pub fn get_target_position(
        &self,
        drone_id: &DroneId,
        leader_position: &GeoPosition,
        leader_heading: f64,
    ) -> Option<GeoPosition> {
        let offsets = self.offsets.read();
        let offset = offsets.get(drone_id)?;

        // Convert heading to radians
        let heading_rad = leader_heading.to_radians();

        // Rotate offset by heading
        let rotated_lat = offset.longitudinal * heading_rad.cos() 
                        - offset.lateral * heading_rad.sin();
        let rotated_lng = offset.longitudinal * heading_rad.sin() 
                        + offset.lateral * heading_rad.cos();

        // Convert meters to degrees (approximate)
        let lat_offset = rotated_lat / 111000.0;
        let lng_offset = rotated_lng / (111000.0 * leader_position.latitude.to_radians().cos());

        Some(GeoPosition::new(
            leader_position.latitude - lat_offset,
            leader_position.longitude + lng_offset,
            leader_position.altitude + offset.vertical,
        ))
    }

    /// Get formation offset for a drone
    pub fn get_offset(&self, drone_id: &DroneId) -> Option<FormationOffset> {
        self.offsets.read().get(drone_id).copied()
    }

    /// Check if drone is in formation position
    pub fn is_in_position(
        &self,
        drone_id: &DroneId,
        current_position: &GeoPosition,
        leader_position: &GeoPosition,
        leader_heading: f64,
        tolerance_meters: f64,
    ) -> bool {
        if let Some(target) = self.get_target_position(drone_id, leader_position, leader_heading) {
            let distance = current_position.distance_to(&target) * 1000.0; // to meters
            distance <= tolerance_meters
        } else {
            false
        }
    }
}

impl Default for ConvoyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convoy_creation() {
        let convoy = ConvoyManager::new();
        assert_eq!(convoy.get_formation(), Formation::Line);
        assert!(convoy.get_leader().is_none());
    }

    #[test]
    fn test_formation_change() {
        let convoy = ConvoyManager::new();
        convoy.set_formation(Formation::Vee);
        assert_eq!(convoy.get_formation(), Formation::Vee);
    }

    #[test]
    fn test_offset_calculation() {
        let convoy = ConvoyManager::new();
        convoy.set_order(vec![
            DroneId::new("REAPER-01"),
            DroneId::new("REAPER-02"),
            DroneId::new("REAPER-03"),
        ]);
        
        let offset1 = convoy.get_offset(&DroneId::new("REAPER-01"));
        assert!(offset1.is_some());
        assert_eq!(offset1.unwrap().longitudinal, 0.0); // Leader
        
        let offset2 = convoy.get_offset(&DroneId::new("REAPER-02"));
        assert!(offset2.is_some());
        assert!(offset2.unwrap().longitudinal > 0.0); // Behind leader
    }
}
