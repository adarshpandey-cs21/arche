use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::mpsc::UnboundedSender;

pub type ConnectionId = String;
pub type ConnectionSender = UnboundedSender<String>;
pub type ConnectionsMap = Arc<RwLock<HashMap<ConnectionId, ConnectionSender>>>;

#[derive(Debug, Clone)]
pub struct SocketConnectionManager {
    connections: ConnectionsMap,
}

impl SocketConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add(&self, id: &ConnectionId, sender: ConnectionSender) {
        let mut connections = self.connections.write().unwrap();
        connections.insert(id.to_string(), sender);
    }

    pub fn remove(&self, id: ConnectionId) {
        let mut connections = self.connections.write().unwrap();
        connections.remove(&id);
    }

    pub fn broadcast(&self, message: String) {
        let connections = self.connections.read().unwrap();
        let all_connections: Vec<_> = connections.iter().collect();
        for (conn_id, sender) in all_connections {
            if let Err(e) = sender.send(message.clone()) {
                println!(
                    "Failed to send message to connection {}: {:?}",
                    conn_id,
                    e.to_string()
                );
            }
        }
    }

    pub fn get_connections(&self) -> Vec<String> {
        let connections = self.connections.read().unwrap();
        connections.keys().cloned().collect()
    }
}

impl Default for SocketConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
