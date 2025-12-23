//! WebSocket error types

use thiserror::Error;

/// WebSocket errors
#[derive(Error, Debug)]
pub enum WsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Client not found: {0}")]
    ClientNotFound(String),

    #[error("Broadcast error: {0}")]
    Broadcast(String),
}

pub type WsResult<T> = Result<T, WsError>;
