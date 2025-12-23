//! Kalman filtering for smooth drone position tracking
//!
//! Uses a simple 2D Kalman filter to predict and smooth drone positions
//! across frames, reducing jitter and handling temporary occlusions.

use serde::{Deserialize, Serialize};
use tracing::trace;

/// State vector size: [x, y, vx, vy]
const STATE_SIZE: usize = 4;
/// Measurement vector size: [x, y]
const MEASUREMENT_SIZE: usize = 2;

/// Kalman filter for 2D position tracking with velocity estimation
#[derive(Debug, Clone)]
pub struct KalmanTracker {
    /// State vector [x, y, vx, vy]
    state: [f64; STATE_SIZE],
    /// Error covariance matrix (4x4 flattened)
    covariance: [[f64; STATE_SIZE]; STATE_SIZE],
    /// Process noise covariance
    process_noise: f64,
    /// Measurement noise covariance
    measurement_noise: f64,
    /// Time step (assume 30 FPS by default)
    dt: f64,
    /// Whether the filter has been initialized
    initialized: bool,
    /// Number of updates
    update_count: u64,
}

impl KalmanTracker {
    /// Create a new Kalman tracker
    pub fn new(process_noise: f64, measurement_noise: f64) -> Self {
        Self {
            state: [0.0; STATE_SIZE],
            covariance: Self::initial_covariance(),
            process_noise,
            measurement_noise,
            dt: 1.0 / 30.0, // 30 FPS
            initialized: false,
            update_count: 0,
        }
    }

    /// Initialize with a measurement
    pub fn initialize(&mut self, x: f64, y: f64) {
        self.state = [x, y, 0.0, 0.0]; // Position, zero velocity
        self.covariance = Self::initial_covariance();
        self.initialized = true;
        self.update_count = 1;
        trace!("Kalman filter initialized at ({}, {})", x, y);
    }

    /// Predict the next state
    pub fn predict(&mut self) -> (f64, f64) {
        if !self.initialized {
            return (0.0, 0.0);
        }

        // State transition: x' = x + vx*dt, y' = y + vy*dt
        let predicted_x = self.state[0] + self.state[2] * self.dt;
        let predicted_y = self.state[1] + self.state[3] * self.dt;

        // Update state
        self.state[0] = predicted_x;
        self.state[1] = predicted_y;

        // Update covariance: P = F*P*F' + Q
        let f = self.transition_matrix();
        let q = self.process_noise_matrix();
        
        let mut fp = [[0.0; STATE_SIZE]; STATE_SIZE];
        for i in 0..STATE_SIZE {
            for j in 0..STATE_SIZE {
                for k in 0..STATE_SIZE {
                    fp[i][j] += f[i][k] * self.covariance[k][j];
                }
            }
        }

        let mut fpft = [[0.0; STATE_SIZE]; STATE_SIZE];
        for i in 0..STATE_SIZE {
            for j in 0..STATE_SIZE {
                for k in 0..STATE_SIZE {
                    fpft[i][j] += fp[i][k] * f[j][k]; // F transpose
                }
            }
        }

        for i in 0..STATE_SIZE {
            for j in 0..STATE_SIZE {
                self.covariance[i][j] = fpft[i][j] + q[i][j];
            }
        }

        trace!("Predicted position: ({:.1}, {:.1})", predicted_x, predicted_y);
        (predicted_x, predicted_y)
    }

    /// Update with a new measurement
    pub fn update(&mut self, measured_x: f64, measured_y: f64) -> (f64, f64) {
        if !self.initialized {
            self.initialize(measured_x, measured_y);
            return (measured_x, measured_y);
        }

        // First predict
        self.predict();

        // Measurement residual: y = z - H*x
        let residual_x = measured_x - self.state[0];
        let residual_y = measured_y - self.state[1];

        // Residual covariance: S = H*P*H' + R
        let s00 = self.covariance[0][0] + self.measurement_noise;
        let s01 = self.covariance[0][1];
        let s10 = self.covariance[1][0];
        let s11 = self.covariance[1][1] + self.measurement_noise;

        // Kalman gain: K = P*H'*S^(-1)
        let det = s00 * s11 - s01 * s10;
        if det.abs() < 1e-10 {
            // Singular matrix, just return measurement
            return (measured_x, measured_y);
        }

        let s_inv_00 = s11 / det;
        let s_inv_01 = -s01 / det;
        let s_inv_10 = -s10 / det;
        let s_inv_11 = s00 / det;

        // K = P * H' * S_inv (H is [1 0 0 0; 0 1 0 0])
        let k = [
            [self.covariance[0][0] * s_inv_00 + self.covariance[0][1] * s_inv_10,
             self.covariance[0][0] * s_inv_01 + self.covariance[0][1] * s_inv_11],
            [self.covariance[1][0] * s_inv_00 + self.covariance[1][1] * s_inv_10,
             self.covariance[1][0] * s_inv_01 + self.covariance[1][1] * s_inv_11],
            [self.covariance[2][0] * s_inv_00 + self.covariance[2][1] * s_inv_10,
             self.covariance[2][0] * s_inv_01 + self.covariance[2][1] * s_inv_11],
            [self.covariance[3][0] * s_inv_00 + self.covariance[3][1] * s_inv_10,
             self.covariance[3][0] * s_inv_01 + self.covariance[3][1] * s_inv_11],
        ];

        // Update state: x = x + K*y
        self.state[0] += k[0][0] * residual_x + k[0][1] * residual_y;
        self.state[1] += k[1][0] * residual_x + k[1][1] * residual_y;
        self.state[2] += k[2][0] * residual_x + k[2][1] * residual_y;
        self.state[3] += k[3][0] * residual_x + k[3][1] * residual_y;

        // Update covariance: P = (I - K*H)*P
        let i_kh = [
            [1.0 - k[0][0], -k[0][1], 0.0, 0.0],
            [-k[1][0], 1.0 - k[1][1], 0.0, 0.0],
            [-k[2][0], -k[2][1], 1.0, 0.0],
            [-k[3][0], -k[3][1], 0.0, 1.0],
        ];

        let old_cov = self.covariance;
        for i in 0..STATE_SIZE {
            for j in 0..STATE_SIZE {
                self.covariance[i][j] = 0.0;
                for k in 0..STATE_SIZE {
                    self.covariance[i][j] += i_kh[i][k] * old_cov[k][j];
                }
            }
        }

        self.update_count += 1;
        trace!("Updated position: ({:.1}, {:.1}), velocity: ({:.1}, {:.1})",
               self.state[0], self.state[1], self.state[2], self.state[3]);

        (self.state[0], self.state[1])
    }

    /// Get current position estimate
    pub fn position(&self) -> (f64, f64) {
        (self.state[0], self.state[1])
    }

    /// Get current velocity estimate
    pub fn velocity(&self) -> (f64, f64) {
        (self.state[2], self.state[3])
    }

    /// Get current state vector
    pub fn state(&self) -> &[f64; STATE_SIZE] {
        &self.state
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get update count
    pub fn update_count(&self) -> u64 {
        self.update_count
    }

    /// Set time step
    pub fn set_dt(&mut self, dt: f64) {
        self.dt = dt;
    }

    /// State transition matrix
    fn transition_matrix(&self) -> [[f64; STATE_SIZE]; STATE_SIZE] {
        [
            [1.0, 0.0, self.dt, 0.0],
            [0.0, 1.0, 0.0, self.dt],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    /// Process noise matrix
    fn process_noise_matrix(&self) -> [[f64; STATE_SIZE]; STATE_SIZE] {
        let dt2 = self.dt * self.dt;
        let dt3 = dt2 * self.dt;
        let dt4 = dt2 * dt2;
        let q = self.process_noise;

        [
            [dt4 / 4.0 * q, 0.0, dt3 / 2.0 * q, 0.0],
            [0.0, dt4 / 4.0 * q, 0.0, dt3 / 2.0 * q],
            [dt3 / 2.0 * q, 0.0, dt2 * q, 0.0],
            [0.0, dt3 / 2.0 * q, 0.0, dt2 * q],
        ]
    }

    /// Initial covariance matrix
    fn initial_covariance() -> [[f64; STATE_SIZE]; STATE_SIZE] {
        [
            [1000.0, 0.0, 0.0, 0.0],
            [0.0, 1000.0, 0.0, 0.0],
            [0.0, 0.0, 1000.0, 0.0],
            [0.0, 0.0, 0.0, 1000.0],
        ]
    }
}

impl Default for KalmanTracker {
    fn default() -> Self {
        Self::new(0.01, 0.1)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_initialization() {
        let mut tracker = KalmanTracker::new(0.01, 0.1);
        assert!(!tracker.is_initialized());
        
        tracker.initialize(100.0, 200.0);
        assert!(tracker.is_initialized());
        
        let pos = tracker.position();
        assert!((pos.0 - 100.0).abs() < 0.01);
        assert!((pos.1 - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_kalman_tracking() {
        let mut tracker = KalmanTracker::new(0.01, 0.1);
        tracker.set_dt(1.0); // 1 second time step for easier testing

        // Simulate object moving right at 10 pixels per frame
        for i in 0..10 {
            let measured_x = 100.0 + i as f64 * 10.0;
            let measured_y = 200.0;
            tracker.update(measured_x, measured_y);
        }

        let (vx, vy) = tracker.velocity();
        // Velocity should be approximately 10 pixels/frame
        assert!(vx > 8.0 && vx < 12.0);
        assert!(vy.abs() < 1.0);
    }

    #[test]
    fn test_prediction() {
        let mut tracker = KalmanTracker::new(0.01, 0.1);
        tracker.set_dt(1.0);

        // Initialize with known velocity
        tracker.initialize(100.0, 100.0);
        tracker.update(110.0, 100.0); // Moving 10 pixels right

        // Predict next position
        let predicted = tracker.predict();
        // Should be around 120 (110 + 10)
        assert!(predicted.0 > 115.0 && predicted.0 < 125.0);
    }
}
