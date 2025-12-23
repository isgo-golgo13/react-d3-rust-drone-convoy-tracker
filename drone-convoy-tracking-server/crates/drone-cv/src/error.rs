//! Error types for the CV module

use thiserror::Error;

/// Errors that can occur in CV operations
#[derive(Error, Debug)]
pub enum CvError {
    #[error("OpenCV error: {0}")]
    OpenCV(String),

    #[error("Frame processing error: {0}")]
    FrameProcessing(String),

    #[error("Halo detection failed: {0}")]
    HaloDetection(String),

    #[error("Tracking error: {0}")]
    Tracking(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Camera calibration error: {0}")]
    Calibration(String),

    #[error("Rendering error: {0}")]
    Rendering(String),

    #[error("Resource not available: {0}")]
    ResourceUnavailable(String),
}

impl CvError {
    pub fn opencv(msg: impl Into<String>) -> Self {
        Self::OpenCV(msg.into())
    }

    pub fn frame_processing(msg: impl Into<String>) -> Self {
        Self::FrameProcessing(msg.into())
    }

    pub fn halo_detection(msg: impl Into<String>) -> Self {
        Self::HaloDetection(msg.into())
    }

    pub fn tracking(msg: impl Into<String>) -> Self {
        Self::Tracking(msg.into())
    }

    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }
}

#[cfg(feature = "opencv")]
impl From<opencv::Error> for CvError {
    fn from(err: opencv::Error) -> Self {
        Self::OpenCV(err.to_string())
    }
}

pub type CvResult<T> = Result<T, CvError>;
