//! Overlay rendering for drone tracking visualization
//!
//! Draws tracking information on video frames:
//! - Red halos around detected drones
//! - Unique tracking IDs
//! - Geographic coordinates
//! - Confidence scores
//! - Bounding boxes

use crate::{CvConfig, CvError, CvResult};
use drone_core::{GeoPosition, HaloColor, TrackingResult};
use tracing::trace;

/// Renders tracking overlays on video frames
pub struct OverlayRenderer {
    config: CvConfig,
}

impl OverlayRenderer {
    /// Create a new overlay renderer
    pub fn new(config: &CvConfig) -> CvResult<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Draw all tracking overlays on a frame
    #[cfg(feature = "opencv")]
    pub fn draw_tracking_overlays(
        &self,
        frame: &mut opencv::core::Mat,
        results: &[TrackingResult],
    ) -> CvResult<()> {
        use opencv::{
            core::{Point, Scalar, Size},
            imgproc,
            prelude::*,
        };

        let render_config = &self.config.rendering;

        for result in results {
            let (b, g, r) = if let Some(halo) = &result.halo {
                halo.color.to_bgr()
            } else {
                HaloColor::RED.to_bgr()
            };
            let color = Scalar::new(b as f64, g as f64, r as f64, 255.0);

            // Draw halo circle
            if render_config.draw_halos {
                if let Some(halo) = &result.halo {
                    imgproc::circle(
                        frame,
                        Point::new(halo.center_x, halo.center_y),
                        halo.radius,
                        color,
                        render_config.halo_thickness,
                        imgproc::LINE_AA,
                        0,
                    )?;

                    // Draw inner glow effect
                    imgproc::circle(
                        frame,
                        Point::new(halo.center_x, halo.center_y),
                        halo.radius - 2,
                        Scalar::new(b as f64 * 0.7, g as f64 * 0.7, r as f64 * 0.7, 200.0),
                        1,
                        imgproc::LINE_AA,
                        0,
                    )?;
                }
            }

            // Draw bounding box
            if render_config.draw_bboxes {
                let bbox = &result.bbox;
                imgproc::rectangle(
                    frame,
                    opencv::core::Rect::new(bbox.x, bbox.y, bbox.width, bbox.height),
                    color,
                    2,
                    imgproc::LINE_AA,
                    0,
                )?;
            }

            // Calculate text position
            let text_x = result.bbox.x;
            let mut text_y = result.bbox.y - 10;

            // Draw tracking ID
            if render_config.draw_ids {
                let id_text = format!("ID:{:04} [{}]", result.tracking_id, result.drone_id);
                
                // Draw background for text
                self.draw_text_background(frame, &id_text, text_x, text_y)?;
                
                imgproc::put_text(
                    frame,
                    &id_text,
                    Point::new(text_x, text_y),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    render_config.font_scale,
                    color,
                    render_config.text_thickness,
                    imgproc::LINE_AA,
                    false,
                )?;
                text_y -= 20;
            }

            // Draw geo coordinates
            if render_config.draw_coordinates {
                if let Some(pos) = &result.estimated_position {
                    let coord_text = format!(
                        "{:.4}°N {:.4}°E",
                        pos.latitude, pos.longitude
                    );
                    
                    self.draw_text_background(frame, &coord_text, text_x, text_y)?;
                    
                    imgproc::put_text(
                        frame,
                        &coord_text,
                        Point::new(text_x, text_y),
                        imgproc::FONT_HERSHEY_SIMPLEX,
                        render_config.font_scale * 0.8,
                        Scalar::new(0.0, 255.0, 255.0, 255.0), // Cyan for coordinates
                        render_config.text_thickness,
                        imgproc::LINE_AA,
                        false,
                    )?;
                    text_y -= 18;
                }
            }

            // Draw confidence
            if render_config.draw_confidence {
                let conf_text = format!("Conf: {:.0}%", result.confidence * 100.0);
                
                self.draw_text_background(frame, &conf_text, text_x, text_y)?;
                
                // Color based on confidence level
                let conf_color = if result.confidence > 0.8 {
                    Scalar::new(0.0, 255.0, 0.0, 255.0) // Green
                } else if result.confidence > 0.5 {
                    Scalar::new(0.0, 255.0, 255.0, 255.0) // Yellow
                } else {
                    Scalar::new(0.0, 0.0, 255.0, 255.0) // Red
                };

                imgproc::put_text(
                    frame,
                    &conf_text,
                    Point::new(text_x, text_y),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    render_config.font_scale * 0.7,
                    conf_color,
                    render_config.text_thickness,
                    imgproc::LINE_AA,
                    false,
                )?;
            }
        }

        // Draw frame info
        self.draw_frame_info(frame, results.len())?;

        Ok(())
    }

    /// Draw semi-transparent background for text
    #[cfg(feature = "opencv")]
    fn draw_text_background(
        &self,
        frame: &mut opencv::core::Mat,
        text: &str,
        x: i32,
        y: i32,
    ) -> CvResult<()> {
        use opencv::{
            core::{Point, Scalar, Size},
            imgproc,
            prelude::*,
        };

        let render_config = &self.config.rendering;
        
        // Get text size
        let mut baseline = 0;
        let text_size = imgproc::get_text_size(
            text,
            imgproc::FONT_HERSHEY_SIMPLEX,
            render_config.font_scale,
            render_config.text_thickness,
            &mut baseline,
        )?;

        // Draw background rectangle
        let padding = 3;
        imgproc::rectangle(
            frame,
            opencv::core::Rect::new(
                x - padding,
                y - text_size.height - padding,
                text_size.width + padding * 2,
                text_size.height + padding * 2,
            ),
            Scalar::new(0.0, 0.0, 0.0, 180.0), // Semi-transparent black
            -1, // Filled
            imgproc::LINE_8,
            0,
        )?;

        Ok(())
    }

    /// Draw frame information overlay
    #[cfg(feature = "opencv")]
    fn draw_frame_info(&self, frame: &mut opencv::core::Mat, track_count: usize) -> CvResult<()> {
        use opencv::{
            core::{Point, Scalar},
            imgproc,
            prelude::*,
        };

        let render_config = &self.config.rendering;
        let info_text = format!("Tracking: {} drones", track_count);

        // Position at top-left
        imgproc::put_text(
            frame,
            &info_text,
            Point::new(10, 30),
            imgproc::FONT_HERSHEY_SIMPLEX,
            render_config.font_scale,
            Scalar::new(255.0, 255.0, 255.0, 255.0), // White
            render_config.text_thickness,
            imgproc::LINE_AA,
            false,
        )?;

        Ok(())
    }

    /// Draw tracking overlays without OpenCV (returns formatted text)
    #[cfg(not(feature = "opencv"))]
    pub fn draw_tracking_overlays(
        &self,
        _frame: &mut (),
        results: &[TrackingResult],
    ) -> CvResult<()> {
        for result in results {
            trace!(
                "Track {}: drone={}, pos=({}, {}), conf={:.2}",
                result.tracking_id,
                result.drone_id,
                result.bbox.x + result.bbox.width / 2,
                result.bbox.y + result.bbox.height / 2,
                result.confidence
            );
        }
        Ok(())
    }

    /// Generate text-based overlay info (for logging/debugging)
    pub fn format_overlay_text(&self, results: &[TrackingResult]) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("=== Tracking {} Drones ===\n", results.len()));
        
        for result in results {
            output.push_str(&format!(
                "ID:{:04} [{}]\n",
                result.tracking_id, result.drone_id
            ));
            
            if let Some(halo) = &result.halo {
                output.push_str(&format!(
                    "  Halo: center=({}, {}), radius={}\n",
                    halo.center_x, halo.center_y, halo.radius
                ));
            }
            
            if let Some(pos) = &result.estimated_position {
                output.push_str(&format!(
                    "  Geo: {:.6}°N, {:.6}°E\n",
                    pos.latitude, pos.longitude
                ));
            }
            
            output.push_str(&format!(
                "  Confidence: {:.1}%\n",
                result.confidence * 100.0
            ));
        }
        
        output
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use drone_core::{BoundingBox, DetectedHalo, DroneId};

    #[test]
    fn test_renderer_creation() {
        let config = CvConfig::default();
        let renderer = OverlayRenderer::new(&config);
        assert!(renderer.is_ok());
    }

    #[test]
    fn test_format_overlay_text() {
        let config = CvConfig::default();
        let renderer = OverlayRenderer::new(&config).unwrap();

        let results = vec![
            TrackingResult {
                drone_id: DroneId::new("REAPER-01"),
                tracking_id: 1,
                bbox: BoundingBox::new(100, 100, 60, 60),
                halo: Some(DetectedHalo::new(130, 130, 30)),
                estimated_position: Some(GeoPosition::new(34.5553, 69.2075, 0.0)),
                confidence: 0.95,
                frame_timestamp: chrono::Utc::now(),
            },
        ];

        let text = renderer.format_overlay_text(&results);
        assert!(text.contains("REAPER-01"));
        assert!(text.contains("34.5553"));
        assert!(text.contains("95.0%"));
    }
}
