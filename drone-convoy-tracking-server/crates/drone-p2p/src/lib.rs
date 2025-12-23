//! # Drone P2P - Mesh Networking
//!
//! libp2p-based mesh networking for drone-to-drone communication.
//! Enables decentralized coordination between drones in the convoy.
//!
//! ## Features
//! - Gossipsub for broadcast messaging
//! - Kademlia DHT for peer discovery
//! - mDNS for local network discovery
//! - Direct messaging between specific drones

pub mod error;
pub mod network;
pub mod protocol;

pub use error::{P2pError, P2pResult};
pub use network::DroneNetwork;
pub use protocol::{DroneMessage, MessageType};

use drone_core::{DroneId, GeoPosition, Telemetry};
use libp2p::{
    gossipsub, identify, kad, mdns, noise, 
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// P2P network configuration
#[derive(Debug, Clone)]
pub struct P2pConfig {
    /// Listen addresses
    pub listen_addrs: Vec<Multiaddr>,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    /// Enable mDNS for local discovery
    pub mdns_enabled: bool,
    /// Gossipsub topic for drone messages
    pub gossip_topic: String,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            listen_addrs: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
            bootstrap_peers: Vec::new(),
            mdns_enabled: true,
            gossip_topic: "drone-convoy".into(),
            heartbeat_interval: Duration::from_secs(1),
        }
    }
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub drone_id: Option<DroneId>,
    pub addresses: Vec<Multiaddr>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// P2P network manager
pub struct P2pManager {
    config: P2pConfig,
    /// Our peer ID
    local_peer_id: PeerId,
    /// Known peers
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    /// Drone ID to Peer ID mapping
    drone_peers: Arc<RwLock<HashMap<DroneId, PeerId>>>,
    /// Message sender
    message_tx: mpsc::Sender<DroneMessage>,
    /// Message receiver
    message_rx: Arc<RwLock<Option<mpsc::Receiver<DroneMessage>>>>,
}

impl P2pManager {
    /// Create a new P2P manager
    pub async fn new(config: P2pConfig) -> P2pResult<Self> {
        info!("🌐 Initializing P2P network...");

        // Generate keypair
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer ID: {}", local_peer_id);

        let (message_tx, message_rx) = mpsc::channel(1024);

        Ok(Self {
            config,
            local_peer_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
            drone_peers: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(Some(message_rx))),
        })
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Get number of connected peers
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    /// Get all connected peer IDs
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.peers.read().keys().cloned().collect()
    }

    /// Register a drone with its peer ID
    pub fn register_drone(&self, drone_id: DroneId, peer_id: PeerId) {
        self.drone_peers.write().insert(drone_id.clone(), peer_id);
        info!("Registered drone {} with peer {}", drone_id, peer_id);
    }

    /// Get peer ID for a drone
    pub fn get_drone_peer(&self, drone_id: &DroneId) -> Option<PeerId> {
        self.drone_peers.read().get(drone_id).cloned()
    }

    /// Broadcast a message to all peers
    pub async fn broadcast(&self, message: DroneMessage) -> P2pResult<()> {
        self.message_tx.send(message).await
            .map_err(|e| P2pError::send(e.to_string()))?;
        Ok(())
    }

    /// Send position update to all peers
    pub async fn broadcast_position(
        &self,
        drone_id: DroneId,
        position: GeoPosition,
        telemetry: Telemetry,
    ) -> P2pResult<()> {
        let message = DroneMessage::position_update(drone_id, position, telemetry);
        self.broadcast(message).await
    }

    /// Send direct message to specific drone
    pub async fn send_to_drone(
        &self,
        target: &DroneId,
        message: DroneMessage,
    ) -> P2pResult<()> {
        if let Some(_peer_id) = self.get_drone_peer(target) {
            // In real implementation, would use direct protocol
            self.broadcast(message).await
        } else {
            Err(P2pError::peer_not_found(target.as_str()))
        }
    }

    /// Take the message receiver (can only be called once)
    pub fn take_message_receiver(&self) -> Option<mpsc::Receiver<DroneMessage>> {
        self.message_rx.write().take()
    }

    /// Start the P2P network (runs in background)
    pub async fn start(&self) -> P2pResult<()> {
        info!("🚀 Starting P2P network on {:?}", self.config.listen_addrs);
        
        // In a real implementation, this would:
        // 1. Create the libp2p swarm
        // 2. Start listening on configured addresses
        // 3. Connect to bootstrap peers
        // 4. Handle incoming/outgoing messages
        
        // For now, we just log that we're "running"
        info!("✅ P2P network started (simulation mode)");
        
        Ok(())
    }

    /// Stop the P2P network
    pub async fn stop(&self) -> P2pResult<()> {
        info!("🛑 Stopping P2P network...");
        Ok(())
    }
}

impl Default for P2pManager {
    fn default() -> Self {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::new(P2pConfig::default()))
            .expect("Failed to create P2pManager")
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_manager_creation() {
        let config = P2pConfig::default();
        let manager = P2pManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_drone_registration() {
        let manager = P2pManager::new(P2pConfig::default()).await.unwrap();
        
        let drone_id = DroneId::new("REAPER-01");
        let peer_id = manager.local_peer_id();
        
        manager.register_drone(drone_id.clone(), peer_id);
        
        assert_eq!(manager.get_drone_peer(&drone_id), Some(peer_id));
    }
}
