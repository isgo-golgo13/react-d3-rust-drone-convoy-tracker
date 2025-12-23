//! WebSocket connection hub
//!
//! Manages all connected WebSocket clients and handles message broadcasting.

use drone_core::{DroneCommand, DroneId, Event};

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Broadcast channel capacity
const BROADCAST_CAPACITY: usize = 1024;

/// WebSocket connection hub
pub struct WebSocketHub {
    /// Broadcast sender for events
    broadcast_tx: broadcast::Sender<Event>,
    /// Connected clients
    clients: DashMap<Uuid, ClientState>,
    /// Total message count
    message_count: AtomicUsize,
    /// Command handler callback
    command_handler: RwLock<Option<Box<dyn Fn(DroneCommand) + Send + Sync>>>,
}

/// State for a connected client
#[derive(Debug)]
struct ClientState {
    /// Subscribed drone IDs (None = all)
    subscriptions: Option<HashSet<DroneId>>,
    /// Connection timestamp
    connected_at: chrono::DateTime<chrono::Utc>,
}

impl WebSocketHub {
    /// Create a new WebSocket hub
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        
        Self {
            broadcast_tx,
            clients: DashMap::new(),
            message_count: AtomicUsize::new(0),
            command_handler: RwLock::new(None),
        }
    }

    /// Register a new client and return a broadcast receiver
    pub fn register_client(&self, client_id: Uuid) -> broadcast::Receiver<Event> {
        let state = ClientState {
            subscriptions: None, // Subscribe to all by default
            connected_at: chrono::Utc::now(),
        };
        
        self.clients.insert(client_id, state);
        info!("Client {} registered ({} total)", client_id, self.clients.len());
        
        self.broadcast_tx.subscribe()
    }

    /// Unregister a client
    pub fn unregister_client(&self, client_id: Uuid) {
        self.clients.remove(&client_id);
        info!("Client {} unregistered ({} remaining)", client_id, self.clients.len());
    }

    /// Get number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Broadcast an event to all clients
    pub async fn broadcast(&self, event: Event) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
        
        // Send to broadcast channel (drops if no receivers)
        let _ = self.broadcast_tx.send(event);
    }

    /// Broadcast multiple events
    pub async fn broadcast_batch(&self, events: Vec<Event>) {
        for event in events {
            self.broadcast(event).await;
        }
    }

    /// Subscribe client to specific drones
    pub fn subscribe(&self, client_id: Uuid, drone_ids: Option<Vec<DroneId>>) {
        if let Some(mut client) = self.clients.get_mut(&client_id) {
            client.subscriptions = drone_ids.map(|ids| ids.into_iter().collect());
            debug!("Client {} subscriptions updated", client_id);
        }
    }

    /// Unsubscribe client from specific drones
    pub fn unsubscribe(&self, client_id: Uuid, drone_ids: Option<Vec<DroneId>>) {
        if let Some(mut client) = self.clients.get_mut(&client_id) {
            if let Some(ref ids) = drone_ids {
                if let Some(ref mut subs) = client.subscriptions {
                    for id in ids {
                        subs.remove(id);
                    }
                }
            } else {
                // Unsubscribe from all
                client.subscriptions = Some(HashSet::new());
            }
            debug!("Client {} unsubscribed", client_id);
        }
    }

    /// Set command handler callback
    pub fn set_command_handler<F>(&self, handler: F)
    where
        F: Fn(DroneCommand) + Send + Sync + 'static,
    {
        *self.command_handler.write() = Some(Box::new(handler));
    }

    /// Handle a command from a client
    pub async fn handle_command(&self, command: DroneCommand) {
        if let Some(ref handler) = *self.command_handler.read() {
            handler(command);
        } else {
            warn!("No command handler registered");
        }
    }

    /// Get total messages broadcast
    pub fn message_count(&self) -> usize {
        self.message_count.load(Ordering::Relaxed)
    }

    /// Get all connected client IDs
    pub fn client_ids(&self) -> Vec<Uuid> {
        self.clients.iter().map(|r| *r.key()).collect()
    }

    /// Check if a specific client is connected
    pub fn is_client_connected(&self, client_id: Uuid) -> bool {
        self.clients.contains_key(&client_id)
    }
}

impl Default for WebSocketHub {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use drone_core::DroneStatus;

    #[test]
    fn test_client_registration() {
        let hub = WebSocketHub::new();
        let id = Uuid::new_v4();
        
        let _rx = hub.register_client(id);
        assert_eq!(hub.client_count(), 1);
        assert!(hub.is_client_connected(id));
        
        hub.unregister_client(id);
        assert_eq!(hub.client_count(), 0);
        assert!(!hub.is_client_connected(id));
    }

    #[test]
    fn test_subscriptions() {
        let hub = WebSocketHub::new();
        let id = Uuid::new_v4();
        
        let _rx = hub.register_client(id);
        
        // Subscribe to specific drones
        hub.subscribe(id, Some(vec![
            DroneId::new("REAPER-01"),
            DroneId::new("REAPER-02"),
        ]));
        
        // Unsubscribe from one
        hub.unsubscribe(id, Some(vec![DroneId::new("REAPER-01")]));
        
        hub.unregister_client(id);
    }

    #[tokio::test]
    async fn test_broadcast_message_count() {
        let hub = WebSocketHub::new();
        
        assert_eq!(hub.message_count(), 0);
        
        let event = Event::drone_status_changed(
            DroneId::new("REAPER-01"),
            DroneStatus::Standby,
            DroneStatus::Moving,
        );
        
        hub.broadcast(event).await;
        
        assert_eq!(hub.message_count(), 1);
    }
}
