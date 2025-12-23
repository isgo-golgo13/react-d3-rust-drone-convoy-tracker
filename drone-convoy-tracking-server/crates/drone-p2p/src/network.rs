//! Network management and swarm handling

use crate::{P2pConfig, P2pError, P2pResult, PeerInfo};
use drone_core::DroneId;

use libp2p::PeerId;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// Drone network abstraction
pub struct DroneNetwork {
    /// Configuration
    config: P2pConfig,
    /// Connected peers
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    /// Network statistics
    stats: Arc<RwLock<NetworkStats>>,
}

/// Network statistics
#[derive(Debug, Default, Clone)]
pub struct NetworkStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub peers_connected: usize,
    pub peers_discovered: usize,
}

impl DroneNetwork {
    /// Create a new drone network
    pub fn new(config: P2pConfig) -> Self {
        Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(NetworkStats::default())),
        }
    }

    /// Add a peer
    pub fn add_peer(&self, peer_id: PeerId, info: PeerInfo) {
        self.peers.write().insert(peer_id, info);
        self.stats.write().peers_connected += 1;
        debug!("Added peer: {}", peer_id);
    }

    /// Remove a peer
    pub fn remove_peer(&self, peer_id: &PeerId) {
        if self.peers.write().remove(peer_id).is_some() {
            let mut stats = self.stats.write();
            if stats.peers_connected > 0 {
                stats.peers_connected -= 1;
            }
            debug!("Removed peer: {}", peer_id);
        }
    }

    /// Get peer info
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.read().get(peer_id).cloned()
    }

    /// Get all peers
    pub fn get_all_peers(&self) -> Vec<PeerInfo> {
        self.peers.read().values().cloned().collect()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    /// Record message sent
    pub fn record_message_sent(&self, bytes: u64) {
        let mut stats = self.stats.write();
        stats.messages_sent += 1;
        stats.bytes_sent += bytes;
    }

    /// Record message received
    pub fn record_message_received(&self, bytes: u64) {
        let mut stats = self.stats.write();
        stats.messages_received += 1;
        stats.bytes_received += bytes;
    }

    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        self.stats.read().clone()
    }

    /// Check if peer is connected
    pub fn is_peer_connected(&self, peer_id: &PeerId) -> bool {
        self.peers.read().contains_key(peer_id)
    }
}

impl Default for DroneNetwork {
    fn default() -> Self {
        Self::new(P2pConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_peer_management() {
        let network = DroneNetwork::default();
        
        let peer_id = PeerId::random();
        let info = PeerInfo {
            peer_id,
            drone_id: Some(DroneId::new("REAPER-01")),
            addresses: Vec::new(),
            last_seen: Utc::now(),
        };

        network.add_peer(peer_id, info);
        assert_eq!(network.peer_count(), 1);
        assert!(network.is_peer_connected(&peer_id));

        network.remove_peer(&peer_id);
        assert_eq!(network.peer_count(), 0);
    }

    #[test]
    fn test_statistics() {
        let network = DroneNetwork::default();
        
        network.record_message_sent(100);
        network.record_message_sent(200);
        network.record_message_received(150);

        let stats = network.get_stats();
        assert_eq!(stats.messages_sent, 2);
        assert_eq!(stats.bytes_sent, 300);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.bytes_received, 150);
    }
}
