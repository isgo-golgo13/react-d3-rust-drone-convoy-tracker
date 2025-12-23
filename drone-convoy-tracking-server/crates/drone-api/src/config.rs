//! API server configuration

use drone_db::DbConfig;
use serde::Deserialize;

/// API server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    /// REST API port
    pub api_port: u16,
    /// WebSocket port
    pub ws_port: u16,
    /// Database configuration
    pub db: DbConfig,
    /// Enable CORS for all origins (development)
    pub cors_permissive: bool,
    /// Enable CV tracking
    pub cv_enabled: bool,
    /// Simulation mode (generate fake data)
    pub simulation_mode: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            api_port: 3000,
            ws_port: 9090,
            db: DbConfig::default(),
            cors_permissive: true,
            cv_enabled: true,
            simulation_mode: true,
        }
    }
}

impl ApiConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let api_port = std::env::var("API_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);

        let ws_port = std::env::var("WS_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(9090);

        let cors_permissive = std::env::var("CORS_PERMISSIVE")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(true);

        let cv_enabled = std::env::var("OPENCV_TRACKING_ENABLED")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(true);

        let simulation_mode = std::env::var("SIMULATION_MODE")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(true);

        Self {
            api_port,
            ws_port,
            db: DbConfig::from_env(),
            cors_permissive,
            cv_enabled,
            simulation_mode,
        }
    }

    /// Configuration for Docker environment
    pub fn docker() -> Self {
        Self {
            api_port: 3000,
            ws_port: 9090,
            db: DbConfig::docker(),
            cors_permissive: true,
            cv_enabled: true,
            simulation_mode: true,
        }
    }
}
