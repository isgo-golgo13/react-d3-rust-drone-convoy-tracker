//! # Drone CV - Computer Vision Module
//!
//! OpenCV-based computer vision for drone halo detection and tracking.
//! Features:
//! - Red halo detection using Hough Circle Transform
//! - Multi-object tracking with unique IDs
//! - Kalman filtering for smooth position prediction
//! - Geo-coordinate projection from camera view
//!
//! ## Red Halo Tracking
//!
//! Each drone is identified by a colored halo (default: red) that is detected
//! using computer vision. The system:
//! 1. Detects circular halos using Hough transforms
//! 2. Tracks halos across frames with unique IDs
//! 3. Draws tracking overlays with ID and geo coordinates
//! 4. Uses Kalman filtering for smooth position prediction

pub mod detector;
pub mod kalman;
pub mod tracker;
pub mod renderer;
pub mod error;
pub mod config;

pub use detector::HaloDetector;
pub use kalman::KalmanTracker;
pub use tracker::DroneTracker;
pub use renderer::OverlayRenderer;
pub use error::CvError;
pub use config::CvConfig;

use drone_core::{BoundingBox, DetectedHalo, DroneId, GeoPosition, HaloColor, TrackingResult};
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Main computer vision engine that coordinates all CV operations
pub struct CvEngine {
    config: CvConfig,
    detector: Arc<RwLock<HaloDetector>>,
    tracker: Arc<RwLock<DroneTracker>>,
    renderer: Arc<RwLock<OverlayRenderer>>,
    /// Camera calibration parameters for geo-projection
    camera_matrix: Option<CameraCalibration>,
    /// Active tracking sessions
    active_tracks: Arc<RwLock<HashMap<u32, ActiveTrack>>>,
}

/// Camera calibration for geo-projection
#[derive(Debug, Clone)]
pub struct CameraCalibration {
    pub focal_length_x: f64,
    pub focal_length_y: f64,
    pub principal_point_x: f64,
    pub principal_point_y: f64,
    pub camera_altitude: f64,
    pub camera_position: GeoPosition,
    pub camera_heading: f64,
}

impl Default for CameraCalibration {
    fn default() -> Self {
        Self {
            focal_length_x: 1000.0,
            focal_length_y: 1000.0,
            principal_point_x: 640.0,
            principal_point_y: 360.0,
            camera_altitude: 5000.0,
            camera_position: GeoPosition::new(34.5553, 69.2075, 5000.0),
            camera_heading: 0.0,
        }
    }
}

/// Active tracking session for a detected drone
#[derive(Debug, Clone)]
pub struct ActiveTrack {
    pub tracking_id: u32,
    pub drone_id: Option<DroneId>,
    pub kalman: KalmanTracker,
    pub last_detection: DetectedHalo,
    pub frames_since_seen: u32,
    pub confidence: f64,
    pub estimated_position: Option<GeoPosition>,
}

impl CvEngine {
    /// Create a new CV engine with default configuration
    pub fn new() -> Result<Self, CvError> {
        Self::with_config(CvConfig::default())
    }

    /// Create a new CV engine with custom configuration
    pub fn with_config(config: CvConfig) -> Result<Self, CvError> {
        info!("🎯 Initializing CV Engine with config: {:?}", config);
        
        let detector = HaloDetector::new(&config)?;
        let tracker = DroneTracker::new(&config)?;
        let renderer = OverlayRenderer::new(&config)?;

        Ok(Self {
            config,
            detector: Arc::new(RwLock::new(detector)),
            tracker: Arc::new(RwLock::new(tracker)),
            renderer: Arc::new(RwLock::new(renderer)),
            camera_matrix: Some(CameraCalibration::default()),
            active_tracks: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Process a video frame and return tracking results
    /// 
    /// This is the main entry point for CV processing:
    /// 1. Detect halos in the frame
    /// 2. Update tracker with detections
    /// 3. Project pixel coordinates to geo coordinates
    /// 4. Return tracking results
    #[cfg(feature = "opencv")]
    pub fn process_frame(&self, frame: &opencv::core::Mat) -> Result<Vec<TrackingResult>, CvError> {
        use opencv::prelude::*;

        // Step 1: Detect halos
        let detections = {
            let detector = self.detector.read();
            detector.detect(frame)?
        };

        debug!("Detected {} halos in frame", detections.len());

        // Step 2: Update tracker
        let tracks = {
            let mut tracker = self.tracker.write();
            tracker.update(&detections)?
        };

        // Step 3: Project to geo coordinates and build results
        let mut results = Vec::with_capacity(tracks.len());
        let calibration = self.camera_matrix.as_ref();

        for track in tracks {
            let estimated_position = calibration.map(|cal| {
                self.project_to_geo(track.last_detection.center_x, track.last_detection.center_y, cal)
            });

            let bbox = BoundingBox::new(
                track.last_detection.center_x - track.last_detection.radius,
                track.last_detection.center_y - track.last_detection.radius,
                track.last_detection.radius * 2,
                track.last_detection.radius * 2,
            );

            let drone_id = track.drone_id.clone()
                .unwrap_or_else(|| DroneId::new(format!("TRACK-{:04}", track.tracking_id)));

            let mut result = TrackingResult::new(drone_id, track.tracking_id, bbox);
            result.halo = Some(track.last_detection.clone());
            result.estimated_position = estimated_position;
            result.confidence = track.confidence;
            result.frame_timestamp = Utc::now();

            results.push(result);
        }

        Ok(results)
    }

    /// Process a frame and render tracking overlays
    /// 
    /// Returns the frame with red halos, tracking IDs, and geo coordinates drawn
    #[cfg(feature = "opencv")]
    pub fn process_and_render(&self, frame: &mut opencv::core::Mat) -> Result<Vec<TrackingResult>, CvError> {
        let results = self.process_frame(frame)?;

        // Render overlays
        {
            let renderer = self.renderer.read();
            renderer.draw_tracking_overlays(frame, &results)?;
        }

        Ok(results)
    }

    /// Project pixel coordinates to geographic coordinates
    fn project_to_geo(&self, pixel_x: i32, pixel_y: i32, cal: &CameraCalibration) -> GeoPosition {
        // Simplified pinhole camera model projection
        // In production, this would use proper camera calibration and terrain models
        
        let dx = (pixel_x as f64 - cal.principal_point_x) / cal.focal_length_x;
        let dy = (pixel_y as f64 - cal.principal_point_y) / cal.focal_length_y;

        // Convert to ground coordinates (assuming flat terrain)
        let ground_x = dx * cal.camera_altitude;
        let ground_y = dy * cal.camera_altitude;

        // Convert to lat/lng offset (simplified)
        // ~111km per degree latitude, varies for longitude
        let lat_offset = ground_y / 111000.0;
        let lng_offset = ground_x / (111000.0 * cal.camera_position.latitude.to_radians().cos());

        // Rotate by camera heading
        let heading_rad = cal.camera_heading.to_radians();
        let rotated_lat = lat_offset * heading_rad.cos() - lng_offset * heading_rad.sin();
        let rotated_lng = lat_offset * heading_rad.sin() + lng_offset * heading_rad.cos();

        GeoPosition::new(
            cal.camera_position.latitude + rotated_lat,
            cal.camera_position.longitude + rotated_lng,
            0.0, // Ground level
        )
    }

    /// Set camera calibration parameters
    pub fn set_camera_calibration(&mut self, calibration: CameraCalibration) {
        self.camera_matrix = Some(calibration);
    }

    /// Associate a tracking ID with a specific drone ID
    pub fn associate_drone(&self, tracking_id: u32, drone_id: DroneId) {
        let mut tracker = self.tracker.write();
        tracker.associate_drone(tracking_id, drone_id);
    }

    /// Get current active track count
    pub fn active_track_count(&self) -> usize {
        let tracker = self.tracker.read();
        tracker.active_count()
    }

    /// Get configuration
    pub fn config(&self) -> &CvConfig {
        &self.config
    }
}

impl Default for CvEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default CvEngine")
    }
}

// ============================================================================
// FRAME SIMULATION (for testing without actual OpenCV)
// ============================================================================

/// Simulated frame data for testing
#[derive(Debug, Clone)]
pub struct SimulatedFrame {
    pub width: u32,
    pub height: u32,
    pub drones: Vec<SimulatedDrone>,
}

/// Simulated drone position for testing
#[derive(Debug, Clone)]
pub struct SimulatedDrone {
    pub id: DroneId,
    pub pixel_x: i32,
    pub pixel_y: i32,
    pub halo_radius: i32,
}

impl CvEngine {
    /// Process a simulated frame (for testing without OpenCV)
    pub fn process_simulated_frame(&self, frame: &SimulatedFrame) -> Vec<TrackingResult> {
        let calibration = self.camera_matrix.as_ref();

        frame.drones.iter().enumerate().map(|(idx, drone)| {
            let tracking_id = idx as u32 + 1;
            
            let estimated_position = calibration.map(|cal| {
                self.project_to_geo(drone.pixel_x, drone.pixel_y, cal)
            });

            let bbox = BoundingBox::new(
                drone.pixel_x - drone.halo_radius,
                drone.pixel_y - drone.halo_radius,
                drone.halo_radius * 2,
                drone.halo_radius * 2,
            );

            let halo = DetectedHalo {
                center_x: drone.pixel_x,
                center_y: drone.pixel_y,
                radius: drone.halo_radius,
                color: HaloColor::RED,
                confidence: 0.95,
            };

            let mut result = TrackingResult::new(drone.id.clone(), tracking_id, bbox);
            result.halo = Some(halo);
            result.estimated_position = estimated_position;
            result.confidence = 0.95;
            result.frame_timestamp = Utc::now();

            result
        }).collect()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cv_engine_creation() {
        let engine = CvEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_geo_projection() {
        let engine = CvEngine::new().unwrap();
        let cal = CameraCalibration::default();
        
        // Center pixel should project to camera position
        let pos = engine.project_to_geo(640, 360, &cal);
        assert!((pos.latitude - 34.5553).abs() < 0.01);
        assert!((pos.longitude - 69.2075).abs() < 0.01);
    }

    #[test]
    fn test_simulated_frame_processing() {
        let engine = CvEngine::new().unwrap();
        
        let frame = SimulatedFrame {
            width: 1280,
            height: 720,
            drones: vec![
                SimulatedDrone {
                    id: DroneId::new("REAPER-01"),
                    pixel_x: 400,
                    pixel_y: 300,
                    halo_radius: 30,
                },
                SimulatedDrone {
                    id: DroneId::new("REAPER-02"),
                    pixel_x: 800,
                    pixel_y: 400,
                    halo_radius: 25,
                },
            ],
        };

        let results = engine.process_simulated_frame(&frame);
        assert_eq!(results.len(), 2);
        assert!(results[0].halo.is_some());
        assert!(results[0].estimated_position.is_some());
    }
}
