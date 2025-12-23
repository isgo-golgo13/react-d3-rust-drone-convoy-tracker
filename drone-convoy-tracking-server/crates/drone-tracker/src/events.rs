//! Event bus for system-wide event distribution

use drone_core::Event;

use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::debug;

/// Event bus for distributing events across the system
pub struct EventBus {
    /// Broadcast sender for events
    sender: broadcast::Sender<Event>,
    /// Event history (last N events)
    history: Arc<RwLock<Vec<Event>>>,
    /// Maximum history size
    max_history: usize,
    /// Event counter
    event_count: Arc<RwLock<u64>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        
        Self {
            sender,
            history: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            max_history: 1000,
            event_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Publish an event
    pub fn publish(&self, event: Event) {
        // Add to history
        {
            let mut history = self.history.write();
            history.push(event.clone());
            if history.len() > self.max_history {
                history.remove(0);
            }
        }

        // Increment counter
        *self.event_count.write() += 1;

        // Broadcast
        let _ = self.sender.send(event);
        
        debug!("Event published, total: {}", self.get_event_count());
    }

    /// Publish multiple events
    pub fn publish_batch(&self, events: Vec<Event>) {
        for event in events {
            self.publish(event);
        }
    }

    /// Get recent events
    pub fn get_recent(&self, count: usize) -> Vec<Event> {
        let history = self.history.read();
        let start = history.len().saturating_sub(count);
        history[start..].to_vec()
    }

    /// Get event count
    pub fn get_event_count(&self) -> u64 {
        *self.event_count.read()
    }

    /// Clear history
    pub fn clear_history(&self) {
        self.history.write().clear();
    }

    /// Get subscriber count (approximate)
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            history: self.history.clone(),
            max_history: self.max_history,
            event_count: self.event_count.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drone_core::{DroneId, DroneStatus};

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new(100);
        assert_eq!(bus.get_event_count(), 0);
    }

    #[test]
    fn test_event_publishing() {
        let bus = EventBus::new(100);
        
        let event = Event::drone_status_changed(
            DroneId::new("REAPER-01"),
            DroneStatus::Standby,
            DroneStatus::Moving,
        );
        
        bus.publish(event);
        
        assert_eq!(bus.get_event_count(), 1);
    }

    #[test]
    fn test_event_history() {
        let bus = EventBus::new(100);
        
        for i in 0..5 {
            let event = Event::drone_status_changed(
                DroneId::new(format!("REAPER-{:02}", i)),
                DroneStatus::Standby,
                DroneStatus::Moving,
            );
            bus.publish(event);
        }
        
        let recent = bus.get_recent(3);
        assert_eq!(recent.len(), 3);
    }

    #[tokio::test]
    async fn test_subscription() {
        let bus = EventBus::new(100);
        let mut rx = bus.subscribe();
        
        let event = Event::drone_status_changed(
            DroneId::new("REAPER-01"),
            DroneStatus::Standby,
            DroneStatus::Moving,
        );
        
        bus.publish(event.clone());
        
        let received = rx.try_recv();
        assert!(received.is_ok());
    }
}
