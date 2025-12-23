//! Configuration for the CV module

use drone_core::HaloColor;
use serde::{Deserialize, Serialize};

/// Configuration for the CV engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvConfig {
    /// Halo detection settings
    pub halo: HaloConfig,
    /// Tracking settings
    pub tracking: TrackingConfig,
    /// Rendering settings
    pub rendering: RenderingConfig,
}

impl Default for CvConfig {
    fn default() -> Self {
        Self {
            halo: HaloConfig::default(),
            tracking: TrackingConfig::default(),
            rendering: RenderingConfig::default(),
        }
    }
}

/// Halo detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaloConfig {
    /// Target halo color (default: red)
    pub color: HaloColor,
    /// Color tolerance for detection (HSV)
    pub hue_tolerance: f64,
    pub saturation_min: f64,
    pub value_min: f64,
    /// Hough circle detection parameters
    pub min_radius: i32,
    pub max_radius: i32,
    pub dp: f64,           // Inverse ratio of accumulator resolution
    pub min_dist: f64,     // Minimum distance between circle centers
    pub param1: f64,       // Canny edge detector threshold
    pub param2: f64,       // Accumulator threshold for circle centers
    /// Minimum confidence for detection
    pub min_confidence: f64,
}

impl Default for HaloConfig {
    fn default() -> Self {
        Self {
            color: HaloColor::RED,
            hue_tolerance: 15.0,
            saturation_min: 100.0,
            value_min: 100.0,
            min_radius: 15,
            max_radius: 100,
            dp: 1.0,
            min_dist: 50.0,
            param1: 100.0,
            param2: 30.0,
            min_confidence: 0.7,
        }
    }
}

/// Tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    /// Maximum frames to keep tracking without detection
    pub max_frames_to_skip: u32,
    /// IoU threshold for track association
    pub iou_threshold: f64,
    /// Kalman filter process noise
    pub kalman_process_noise: f64,
    /// Kalman filter measurement noise
    pub kalman_measurement_noise: f64,
    /// Maximum number of active tracks
    pub max_tracks: usize,
    /// Minimum frames to confirm a new track
    pub min_frames_to_confirm: u32,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            max_frames_to_skip: 10,
            iou_threshold: 0.3,
            kalman_process_noise: 0.01,
            kalman_measurement_noise: 0.1,
            max_tracks: 50,
            min_frames_to_confirm: 3,
        }
    }
}

/// Rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingConfig {
    /// Draw halo circles
    pub draw_halos: bool,
    /// Draw bounding boxes
    pub draw_bboxes: bool,
    /// Draw tracking IDs
    pub draw_ids: bool,
    /// Draw geo coordinates
    pub draw_coordinates: bool,
    /// Draw confidence scores
    pub draw_confidence: bool,
    /// Halo circle thickness
    pub halo_thickness: i32,
    /// Font scale for text
    pub font_scale: f64,
    /// Text thickness
    pub text_thickness: i32,
    /// Overlay background opacity
    pub overlay_opacity: f64,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            draw_halos: true,
            draw_bboxes: true,
            draw_ids: true,
            draw_coordinates: true,
            draw_confidence: true,
            halo_thickness: 3,
            font_scale: 0.6,
            text_thickness: 2,
            overlay_opacity: 0.7,
        }
    }
}

impl CvConfig {
    /// Create config optimized for red halo detection
    pub fn red_halo_tracking() -> Self {
        Self {
            halo: HaloConfig {
                color: HaloColor::RED,
                hue_tolerance: 10.0,
                saturation_min: 150.0,
                value_min: 150.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create config for multi-color halo tracking
    pub fn multi_color_tracking() -> Self {
        Self {
            halo: HaloConfig {
                hue_tolerance: 20.0,
                saturation_min: 80.0,
                value_min: 80.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create config optimized for high performance (lower quality)
    pub fn high_performance() -> Self {
        Self {
            halo: HaloConfig {
                dp: 2.0,
                param2: 50.0,
                ..Default::default()
            },
            tracking: TrackingConfig {
                max_tracks: 20,
                max_frames_to_skip: 5,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
