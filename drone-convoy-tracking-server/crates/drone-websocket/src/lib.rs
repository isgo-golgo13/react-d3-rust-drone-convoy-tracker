//! # Drone WebSocket Server
//!
//! Real-time WebSocket server for streaming drone telemetry
//! to the React frontend. Supports:
//! - Broadcast to all connected clients
//! - Per-drone subscriptions
//! - Bidirectional communication for commands
//!
//! ## Protocol
//!
//! Messages are JSON-encoded using the types from `drone_core::events`:
//! - Server → Client: `ServerMessage`
//! - Client → Server: `ClientMessage`

pub mod error;
pub mod hub;

pub use error::{WsError, WsResult};
pub use hub::WebSocketHub;

use drone_core::{
    Event, ServerMessage, ClientMessage, FullStateEvent,
    Drone, DroneId, Mission, TrackingResult,
};

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Start the WebSocket server
pub async fn start_server(hub: Arc<WebSocketHub>, port: u16) -> WsResult<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("🔌 WebSocket server listening on ws://{}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let hub = hub.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(hub, stream, addr).await {
                        error!("WebSocket connection error from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept WebSocket connection: {}", e);
            }
        }
    }
}

/// Handle a single WebSocket connection
async fn handle_connection(
    hub: Arc<WebSocketHub>,
    stream: TcpStream,
    addr: SocketAddr,
) -> WsResult<()> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Generate client ID
    let client_id = Uuid::new_v4();
    info!("🔗 WebSocket client {} connected from {}", client_id, addr);

    // Register client and get broadcast receiver
    let mut broadcast_rx = hub.register_client(client_id);

    // Send initial state
    let initial_state = ServerMessage::InitialState(FullStateEvent {
        drones: Vec::new(), // Will be populated by API
        mission: None,
        tracking_results: Vec::new(),
    });
    
    let msg = serde_json::to_string(&initial_state)?;
    ws_sender.send(Message::Text(msg.into())).await?;

    // Spawn task to handle incoming messages from client
    let hub_clone = hub.clone();
    let client_id_clone = client_id;
    let incoming_handle = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = handle_client_message(&hub_clone, client_id_clone, &text).await {
                        warn!("Error handling client message: {}", e);
                    }
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping from {}", client_id_clone);
                    // Pong is handled automatically by tungstenite
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong from {}", client_id_clone);
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} sent close frame", client_id_clone);
                    break;
                }
                Ok(Message::Binary(_)) => {
                    warn!("Received unexpected binary message from {}", client_id_clone);
                }
                Err(e) => {
                    error!("Error receiving message from {}: {}", client_id_clone, e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Forward broadcast messages to this client
    loop {
        tokio::select! {
            // Receive from broadcast channel
            result = broadcast_rx.recv() => {
                match result {
                    Ok(event) => {
                        let msg = ServerMessage::Event(event);
                        match serde_json::to_string(&msg) {
                            Ok(json) => {
                                if let Err(e) = ws_sender.send(Message::Text(json.into())).await {
                                    error!("Failed to send to client {}: {}", client_id, e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to serialize event: {}", e);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Client {} lagged by {} messages", client_id, n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Broadcast channel closed");
                        break;
                    }
                }
            }
            // Check if incoming handler finished (client disconnected)
            _ = &mut Box::pin(async { incoming_handle.is_finished() }) => {
                if incoming_handle.is_finished() {
                    break;
                }
            }
        }
    }

    // Cleanup
    hub.unregister_client(client_id);
    info!("🔌 WebSocket client {} disconnected", client_id);

    Ok(())
}

/// Handle a message from a client
async fn handle_client_message(
    hub: &WebSocketHub,
    client_id: Uuid,
    text: &str,
) -> WsResult<()> {
    let msg: ClientMessage = serde_json::from_str(text)?;

    match msg {
        ClientMessage::Subscribe { drone_ids } => {
            debug!("Client {} subscribing to {:?}", client_id, drone_ids);
            hub.subscribe(client_id, drone_ids);
        }
        ClientMessage::Unsubscribe { drone_ids } => {
            debug!("Client {} unsubscribing from {:?}", client_id, drone_ids);
            hub.unsubscribe(client_id, drone_ids);
        }
        ClientMessage::RequestState => {
            debug!("Client {} requesting state", client_id);
            // State is sent via HTTP API, not WebSocket
        }
        ClientMessage::DroneCommand(cmd) => {
            info!("Client {} sending command to {}: {:?}", 
                  client_id, cmd.drone_id, cmd.command);
            // Forward to command handler
            hub.handle_command(cmd).await;
        }
        ClientMessage::Pong { timestamp } => {
            debug!("Client {} pong: {}", client_id, timestamp);
        }
    }

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_creation() {
        let hub = WebSocketHub::new();
        assert_eq!(hub.client_count(), 0);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let hub = WebSocketHub::new();
        
        // Register a client
        let client_id = Uuid::new_v4();
        let mut rx = hub.register_client(client_id);
        
        assert_eq!(hub.client_count(), 1);
        
        // Broadcast an event
        let event = Event::drone_status_changed(
            DroneId::new("REAPER-01"),
            drone_core::DroneStatus::Standby,
            drone_core::DroneStatus::Moving,
        );
        
        hub.broadcast(event.clone()).await;
        
        // Should receive the event
        let received = rx.try_recv();
        assert!(received.is_ok());
        
        // Cleanup
        hub.unregister_client(client_id);
        assert_eq!(hub.client_count(), 0);
    }
}
