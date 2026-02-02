use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::mpsc::UnboundedSender;

use crate::error::AppError;

pub type ConnectionId = String;
pub type ConnectionSender = UnboundedSender<String>;
pub type ConnectionsMap = Arc<RwLock<HashMap<ConnectionId, ConnectionSender>>>;

fn lock_error() -> AppError {
    AppError::internal_error(
        "Lock poisoned".to_string(),
        Some("A thread panicked while holding the lock".to_string()),
    )
}

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

    pub fn add(&self, id: &ConnectionId, sender: ConnectionSender) -> Result<(), AppError> {
        let mut connections = self.connections.write().map_err(|_| lock_error())?;
        connections.insert(id.to_string(), sender);
        Ok(())
    }

    pub fn remove(&self, id: ConnectionId) -> Result<(), AppError> {
        let mut connections = self.connections.write().map_err(|_| lock_error())?;
        connections.remove(&id);
        Ok(())
    }

    pub fn broadcast(&self, message: String) -> Result<(), AppError> {
        let connections = self.connections.read().map_err(|_| lock_error())?;
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
        Ok(())
    }

    pub fn get_connections(&self) -> Result<Vec<String>, AppError> {
        let connections = self.connections.read().map_err(|_| lock_error())?;
        Ok(connections.keys().cloned().collect())
    }
}

impl Default for SocketConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
