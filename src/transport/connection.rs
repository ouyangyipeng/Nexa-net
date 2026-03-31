//! Connection Management
//!
//! Connection pool and session management.

use crate::error::Result;
use std::collections::HashMap;
use std::time::Duration;

/// Connection pool
pub struct ConnectionPool {
    /// Active connections
    connections: HashMap<String, Connection>,
    /// Maximum connections
    max_connections: usize,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: HashMap::new(),
            max_connections,
        }
    }

    /// Get or create a connection
    pub async fn get(&mut self, endpoint: &str) -> Result<&Connection> {
        if !self.connections.contains_key(endpoint) {
            let conn = Connection::new(endpoint).await?;
            self.connections.insert(endpoint.to_string(), conn);
        }
        Ok(self.connections.get(endpoint).unwrap())
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Connection to a remote endpoint
pub struct Connection {
    /// Endpoint URL
    pub endpoint: String,
    /// Is connection active
    pub active: bool,
}

impl Connection {
    /// Create a new connection
    pub async fn new(endpoint: &str) -> Result<Self> {
        Ok(Self {
            endpoint: endpoint.to_string(),
            active: true,
        })
    }

    /// Close the connection
    pub fn close(&mut self) {
        self.active = false;
    }
}

/// Session for tracking a conversation
pub struct Session {
    /// Session ID
    pub id: String,
    /// Created at
    pub created_at: std::time::Instant,
    /// Last activity
    pub last_activity: std::time::Instant,
    /// Session timeout
    pub timeout: Duration,
}

impl Session {
    /// Create a new session
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: now,
            last_activity: now,
            timeout: Duration::from_secs(300),
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        self.last_activity.elapsed() > self.timeout
    }

    /// Update last activity
    pub fn touch(&mut self) {
        self.last_activity = std::time::Instant::now();
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session() {
        let session = Session::new();
        assert!(!session.is_expired());
    }
}
