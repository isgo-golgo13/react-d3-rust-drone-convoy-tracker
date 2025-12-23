//! P2P error types

use thiserror::Error;

/// P2P network errors
#[derive(Error, Debug)]
pub enum P2pError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Message send error: {0}")]
    Send(String),

    #[error("Message receive error: {0}")]
    Receive(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

impl P2pError {
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    pub fn peer_not_found(msg: impl Into<String>) -> Self {
        Self::PeerNotFound(msg.into())
    }

    pub fn send(msg: impl Into<String>) -> Self {
        Self::Send(msg.into())
    }
}

pub type P2pResult<T> = Result<T, P2pError>;
