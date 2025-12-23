//! Halo detection using Hough Circle Transform
//!
//! Detects circular halos around drones using color filtering and
//! the Hough Circle Transform algorithm.

use crate::{CvConfig, CvError, CvResult};
use drone_core::{DetectedHalo, HaloColor};
use tracing::{debug, trace};

/// Halo detector using OpenCV
pub struct HaloDetector {
    config: CvConfig,
    /// Detection statistics
    stats: DetectionStats,
}

/// Statistics for halo detection
#[derive(Debug, Default, Clone)]
pub struct DetectionStats {
    pub frames_processed: u64,
    pub halos_detected: u64,
    pub false_positives: u64,
    pub detection_time_ms: f64,
}

impl HaloDetector {
    /// Create a new halo detector with the given configuration
    pub fn new(config: &CvConfig) -> CvResult<Self> {
        Ok(Self {
            config: config.clone(),
            stats: DetectionStats::default(),
        })
    }

    /// Detect halos in a frame
    /// 
    /// Process:
    /// 1. Convert to HSV color space
    /// 2. Filter for target halo color (red)
    /// 3. Apply morphological operations
    /// 4. Detect circles using Hough Transform
    /// 5. Validate and return detections
    #[cfg(feature = "opencv")]
    pub fn detect(&self, frame: &opencv::core::Mat) -> CvResult<Vec<DetectedHalo>> {
        use opencv::{
            core::{self, Mat, Scalar, Vector},
            imgproc,
            prelude::*,
        };

        let start = std::time::Instant::now();

        // Convert to HSV
        let mut hsv = Mat::default();
        imgproc::cvt_color(frame, &mut hsv, imgproc::COLOR_BGR2HSV, 0)?;

        // Create mask for red color (wraps around in HSV)
        let halo_config = &self.config.halo;
        
        // Red has hue around 0 and 180 in OpenCV (0-180 range)
        let lower_red1 = Scalar::new(0.0, halo_config.saturation_min, halo_config.value_min, 0.0);
        let upper_red1 = Scalar::new(halo_config.hue_tolerance, 255.0, 255.0, 0.0);
        
        let lower_red2 = Scalar::new(180.0 - halo_config.hue_tolerance, halo_config.saturation_min, halo_config.value_min, 0.0);
        let upper_red2 = Scalar::new(180.0, 255.0, 255.0, 0.0);

        let mut mask1 = Mat::default();
        let mut mask2 = Mat::default();
        core::in_range(&hsv, &lower_red1, &upper_red1, &mut mask1)?;
        core::in_range(&hsv, &lower_red2, &upper_red2, &mut mask2)?;

        let mut mask = Mat::default();
        core::bitwise_or(&mask1, &mask2, &mut mask, &core::no_array())?;

        // Morphological operations to clean up mask
        let kernel = imgproc::get_structuring_element(
            imgproc::MORPH_ELLIPSE,
            core::Size::new(5, 5),
            core::Point::new(-1, -1),
        )?;
        
        let mut cleaned = Mat::default();
        imgproc::morphology_ex(&mask, &mut cleaned, imgproc::MORPH_OPEN, &kernel, 
                               core::Point::new(-1, -1), 2, core::BORDER_CONSTANT, 
                               imgproc::morphology_default_border_value()?)?;
        imgproc::morphology_ex(&cleaned, &mut cleaned, imgproc::MORPH_CLOSE, &kernel,
                               core::Point::new(-1, -1), 2, core::BORDER_CONSTANT,
                               imgproc::morphology_default_border_value()?)?;

        // Apply Gaussian blur
        let mut blurred = Mat::default();
        imgproc::gaussian_blur(&cleaned, &mut blurred, core::Size::new(9, 9), 2.0, 2.0, core::BORDER_DEFAULT)?;

        // Detect circles using Hough Circle Transform
        let mut circles = Vector::<core::Vec3f>::new();
        imgproc::hough_circles(
            &blurred,
            &mut circles,
            imgproc::HOUGH_GRADIENT,
            halo_config.dp,
            halo_config.min_dist,
            halo_config.param1,
            halo_config.param2,
            halo_config.min_radius,
            halo_config.max_radius,
        )?;

        // Convert to DetectedHalo
        let mut detections = Vec::with_capacity(circles.len());
        for i in 0..circles.len() {
            let circle = circles.get(i)?;
            let center_x = circle[0] as i32;
            let center_y = circle[1] as i32;
            let radius = circle[2] as i32;

            // Calculate confidence based on circle quality
            let confidence = self.calculate_confidence(frame, center_x, center_y, radius)?;

            if confidence >= halo_config.min_confidence {
                detections.push(DetectedHalo {
                    center_x,
                    center_y,
                    radius,
                    color: HaloColor::RED,
                    confidence,
                });
            }
        }

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        debug!("Detected {} halos in {:.2}ms", detections.len(), elapsed);

        Ok(detections)
    }

    /// Detect halos without OpenCV (for testing/simulation)
    #[cfg(not(feature = "opencv"))]
    pub fn detect(&self, _frame: &()) -> CvResult<Vec<DetectedHalo>> {
        Ok(Vec::new())
    }

    /// Calculate detection confidence
    #[cfg(feature = "opencv")]
    fn calculate_confidence(
        &self,
        frame: &opencv::core::Mat,
        center_x: i32,
        center_y: i32,
        radius: i32,
    ) -> CvResult<f64> {
        use opencv::{core::Mat, prelude::*};

        // Verify the circle has strong red color
        // Sample pixels along the circle circumference
        let mut red_pixels = 0;
        let sample_count = 16;
        
        for i in 0..sample_count {
            let angle = (i as f64 / sample_count as f64) * 2.0 * std::f64::consts::PI;
            let px = (center_x as f64 + radius as f64 * angle.cos()) as i32;
            let py = (center_y as f64 + radius as f64 * angle.sin()) as i32;
            
            if px >= 0 && px < frame.cols() && py >= 0 && py < frame.rows() {
                let pixel = frame.at_2d::<opencv::core::Vec3b>(py, px)?;
                // Check if pixel is reddish (BGR format)
                if pixel[2] > 150 && pixel[2] > pixel[1] && pixel[2] > pixel[0] {
                    red_pixels += 1;
                }
            }
        }

        Ok(red_pixels as f64 / sample_count as f64)
    }

    /// Get detection statistics
    pub fn stats(&self) -> &DetectionStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = DetectionStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let config = CvConfig::default();
        let detector = HaloDetector::new(&config);
        assert!(detector.is_ok());
    }
}
