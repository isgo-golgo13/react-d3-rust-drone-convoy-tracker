//! Multi-object tracking for drones
//!
//! Tracks multiple drones across frames using Hungarian algorithm
//! for detection-to-track association and Kalman filtering for
//! position prediction.

use crate::{ActiveTrack, CvConfig, CvError, CvResult, KalmanTracker};
use drone_core::{DetectedHalo, DroneId, HaloColor};
use std::collections::HashMap;
use tracing::{debug, trace, warn};

/// Multi-object drone tracker
pub struct DroneTracker {
    config: CvConfig,
    /// Active tracks indexed by tracking ID
    tracks: HashMap<u32, TrackState>,
    /// Next tracking ID to assign
    next_id: u32,
    /// Drone ID associations (tracking_id -> drone_id)
    drone_associations: HashMap<u32, DroneId>,
    /// Frame counter
    frame_count: u64,
}

/// Internal track state
#[derive(Debug, Clone)]
struct TrackState {
    tracking_id: u32,
    kalman: KalmanTracker,
    last_detection: DetectedHalo,
    frames_since_detection: u32,
    consecutive_detections: u32,
    confidence: f64,
    confirmed: bool,
}

impl DroneTracker {
    /// Create a new drone tracker
    pub fn new(config: &CvConfig) -> CvResult<Self> {
        Ok(Self {
            config: config.clone(),
            tracks: HashMap::new(),
            next_id: 1,
            drone_associations: HashMap::new(),
            frame_count: 0,
        })
    }

    /// Update tracker with new detections
    /// 
    /// This method:
    /// 1. Predicts positions for existing tracks
    /// 2. Associates detections with tracks using IoU
    /// 3. Updates matched tracks
    /// 4. Creates new tracks for unmatched detections
    /// 5. Removes stale tracks
    pub fn update(&mut self, detections: &[DetectedHalo]) -> CvResult<Vec<ActiveTrack>> {
        self.frame_count += 1;
        trace!("Frame {}: Processing {} detections", self.frame_count, detections.len());

        // Step 1: Predict positions for all existing tracks
        for track in self.tracks.values_mut() {
            track.kalman.predict();
        }

        // Step 2: Associate detections with tracks
        let associations = self.associate_detections(detections);

        // Step 3: Update matched tracks
        for (track_id, detection_idx) in &associations {
            if let Some(track) = self.tracks.get_mut(track_id) {
                let detection = &detections[*detection_idx];
                track.kalman.update(detection.center_x as f64, detection.center_y as f64);
                track.last_detection = detection.clone();
                track.frames_since_detection = 0;
                track.consecutive_detections += 1;
                track.confidence = detection.confidence;

                // Confirm track after minimum detections
                if !track.confirmed && track.consecutive_detections >= self.config.tracking.min_frames_to_confirm {
                    track.confirmed = true;
                    debug!("Track {} confirmed", track_id);
                }
            }
        }

        // Step 4: Create new tracks for unmatched detections
        let matched_detections: Vec<usize> = associations.values().copied().collect();
        for (idx, detection) in detections.iter().enumerate() {
            if !matched_detections.contains(&idx) && self.tracks.len() < self.config.tracking.max_tracks {
                self.create_track(detection);
            }
        }

        // Step 5: Update unmatched tracks and remove stale ones
        let max_skip = self.config.tracking.max_frames_to_skip;
        let stale_ids: Vec<u32> = self.tracks.iter()
            .filter(|(id, track)| {
                !associations.contains_key(id) && track.frames_since_detection >= max_skip
            })
            .map(|(id, _)| *id)
            .collect();

        for id in stale_ids {
            debug!("Removing stale track {}", id);
            self.tracks.remove(&id);
            self.drone_associations.remove(&id);
        }

        // Increment frames_since_detection for unmatched tracks
        for (id, track) in self.tracks.iter_mut() {
            if !associations.contains_key(id) {
                track.frames_since_detection += 1;
                track.consecutive_detections = 0;
            }
        }

        // Convert to ActiveTrack output
        let active_tracks: Vec<ActiveTrack> = self.tracks.values()
            .filter(|t| t.confirmed)
            .map(|t| ActiveTrack {
                tracking_id: t.tracking_id,
                drone_id: self.drone_associations.get(&t.tracking_id).cloned(),
                kalman: t.kalman.clone(),
                last_detection: t.last_detection.clone(),
                frames_since_seen: t.frames_since_detection,
                confidence: t.confidence,
                estimated_position: None, // Set by CvEngine
            })
            .collect();

        debug!("Active tracks: {}, Total tracks: {}", active_tracks.len(), self.tracks.len());
        Ok(active_tracks)
    }

    /// Associate detections with existing tracks
    fn associate_detections(&self, detections: &[DetectedHalo]) -> HashMap<u32, usize> {
        if self.tracks.is_empty() || detections.is_empty() {
            return HashMap::new();
        }

        let track_ids: Vec<u32> = self.tracks.keys().copied().collect();
        let track_count = track_ids.len();
        let detection_count = detections.len();

        // Build cost matrix based on IoU
        let mut cost_matrix = vec![vec![f64::MAX; detection_count]; track_count];

        for (t_idx, track_id) in track_ids.iter().enumerate() {
            if let Some(track) = self.tracks.get(track_id) {
                let (pred_x, pred_y) = track.kalman.position();
                let pred_radius = track.last_detection.radius;

                for (d_idx, detection) in detections.iter().enumerate() {
                    let iou = Self::calculate_circle_iou(
                        pred_x as i32, pred_y as i32, pred_radius,
                        detection.center_x, detection.center_y, detection.radius,
                    );

                    if iou > self.config.tracking.iou_threshold {
                        // Cost is inverse of IoU (lower is better)
                        cost_matrix[t_idx][d_idx] = 1.0 - iou;
                    }
                }
            }
        }

        // Greedy assignment (could be replaced with Hungarian algorithm)
        let mut associations = HashMap::new();
        let mut assigned_detections = vec![false; detection_count];
        let mut assigned_tracks = vec![false; track_count];

        // Find minimum cost assignments
        loop {
            let mut min_cost = f64::MAX;
            let mut min_t = 0;
            let mut min_d = 0;

            for t_idx in 0..track_count {
                if assigned_tracks[t_idx] {
                    continue;
                }
                for d_idx in 0..detection_count {
                    if assigned_detections[d_idx] {
                        continue;
                    }
                    if cost_matrix[t_idx][d_idx] < min_cost {
                        min_cost = cost_matrix[t_idx][d_idx];
                        min_t = t_idx;
                        min_d = d_idx;
                    }
                }
            }

            if min_cost == f64::MAX {
                break;
            }

            associations.insert(track_ids[min_t], min_d);
            assigned_tracks[min_t] = true;
            assigned_detections[min_d] = true;
        }

        trace!("Associated {} tracks with detections", associations.len());
        associations
    }

    /// Calculate IoU (Intersection over Union) for two circles
    fn calculate_circle_iou(
        x1: i32, y1: i32, r1: i32,
        x2: i32, y2: i32, r2: i32,
    ) -> f64 {
        let dx = (x2 - x1) as f64;
        let dy = (y2 - y1) as f64;
        let d = (dx * dx + dy * dy).sqrt();
        let r1 = r1 as f64;
        let r2 = r2 as f64;

        // No overlap
        if d >= r1 + r2 {
            return 0.0;
        }

        // One circle contains the other
        if d <= (r1 - r2).abs() {
            let min_area = std::f64::consts::PI * r1.min(r2).powi(2);
            let max_area = std::f64::consts::PI * r1.max(r2).powi(2);
            return min_area / max_area;
        }

        // Partial overlap - use lens formula
        let part1 = r1.powi(2) * (((d.powi(2) + r1.powi(2) - r2.powi(2)) / (2.0 * d * r1)).acos());
        let part2 = r2.powi(2) * (((d.powi(2) + r2.powi(2) - r1.powi(2)) / (2.0 * d * r2)).acos());
        let part3 = 0.5 * ((-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2)).sqrt();
        
        let intersection = part1 + part2 - part3;
        let union = std::f64::consts::PI * (r1.powi(2) + r2.powi(2)) - intersection;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }

    /// Create a new track from a detection
    fn create_track(&mut self, detection: &DetectedHalo) -> u32 {
        let tracking_id = self.next_id;
        self.next_id += 1;

        let mut kalman = KalmanTracker::new(
            self.config.tracking.kalman_process_noise,
            self.config.tracking.kalman_measurement_noise,
        );
        kalman.initialize(detection.center_x as f64, detection.center_y as f64);

        let track = TrackState {
            tracking_id,
            kalman,
            last_detection: detection.clone(),
            frames_since_detection: 0,
            consecutive_detections: 1,
            confidence: detection.confidence,
            confirmed: false,
        };

        self.tracks.insert(tracking_id, track);
        debug!("Created new track {}", tracking_id);
        tracking_id
    }

    /// Associate a tracking ID with a specific drone
    pub fn associate_drone(&mut self, tracking_id: u32, drone_id: DroneId) {
        self.drone_associations.insert(tracking_id, drone_id);
        debug!("Associated track {} with drone {}", tracking_id, drone_id);
    }

    /// Get the number of active tracks
    pub fn active_count(&self) -> usize {
        self.tracks.values().filter(|t| t.confirmed).count()
    }

    /// Get total track count (including unconfirmed)
    pub fn total_count(&self) -> usize {
        self.tracks.len()
    }

    /// Get all active track IDs
    pub fn active_track_ids(&self) -> Vec<u32> {
        self.tracks.values()
            .filter(|t| t.confirmed)
            .map(|t| t.tracking_id)
            .collect()
    }

    /// Clear all tracks
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.drone_associations.clear();
        debug!("Cleared all tracks");
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let config = CvConfig::default();
        let tracker = DroneTracker::new(&config);
        assert!(tracker.is_ok());
    }

    #[test]
    fn test_circle_iou() {
        // Same circle = IoU of 1.0
        let iou = DroneTracker::calculate_circle_iou(100, 100, 50, 100, 100, 50);
        assert!((iou - 1.0).abs() < 0.01);

        // No overlap
        let iou = DroneTracker::calculate_circle_iou(0, 0, 10, 100, 100, 10);
        assert!(iou < 0.01);

        // Partial overlap
        let iou = DroneTracker::calculate_circle_iou(100, 100, 50, 120, 100, 50);
        assert!(iou > 0.5 && iou < 1.0);
    }

    #[test]
    fn test_track_creation() {
        let config = CvConfig::default();
        let mut tracker = DroneTracker::new(&config).unwrap();

        let detections = vec![
            DetectedHalo {
                center_x: 100,
                center_y: 100,
                radius: 30,
                color: HaloColor::RED,
                confidence: 0.9,
            },
        ];

        // First update creates track
        let _ = tracker.update(&detections).unwrap();
        assert_eq!(tracker.total_count(), 1);

        // Not confirmed yet
        assert_eq!(tracker.active_count(), 0);

        // Multiple updates to confirm
        for _ in 0..5 {
            let _ = tracker.update(&detections).unwrap();
        }

        // Now should be confirmed
        assert_eq!(tracker.active_count(), 1);
    }
}
