//! Node Status Management
//!
//! Tracks health and load of network nodes.

use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Node status information
#[derive(Debug, Clone)]
pub struct NodeStatus {
    /// Node DID
    pub did: String,
    /// Is node online
    pub online: bool,
    /// Current load (0.0 - 1.0)
    pub load: f32,
    /// Average latency in ms
    pub avg_latency_ms: u64,
    /// Last heartbeat
    pub last_heartbeat: DateTime<Utc>,
    /// Total calls served
    pub total_calls: u64,
    /// Failed calls
    pub failed_calls: u64,
}

impl NodeStatus {
    /// Create a new node status
    pub fn new(did: &str) -> Self {
        Self {
            did: did.to_string(),
            online: true,
            load: 0.0,
            avg_latency_ms: 0,
            last_heartbeat: Utc::now(),
            total_calls: 0,
            failed_calls: 0,
        }
    }
    
    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.total_calls == 0 {
            return 1.0;
        }
        (self.total_calls - self.failed_calls) as f32 / self.total_calls as f32
    }
}

/// Node status manager
#[derive(Debug, Clone, Default)]
pub struct NodeStatusManager {
    /// Node statuses by DID
    statuses: HashMap<String, NodeStatus>,
}

impl NodeStatusManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Update node status
    pub fn update(&mut self, status: NodeStatus) {
        self.statuses.insert(status.did.clone(), status);
    }
    
    /// Get node status
    pub fn get(&self, did: &str) -> Option<&NodeStatus> {
        self.statuses.get(did)
    }
    
    /// Check if node is healthy
    pub fn is_healthy(&self, did: &str) -> bool {
        self.statuses.get(did)
            .map(|s| s.online && s.success_rate() > 0.9)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_status() {
        let status = NodeStatus::new("did:nexa:test");
        assert!(status.online);
        assert_eq!(status.success_rate(), 1.0);
    }

    #[test]
    fn test_status_manager() {
        let mut manager = NodeStatusManager::new();
        let status = NodeStatus::new("did:nexa:test");
        
        manager.update(status);
        assert!(manager.is_healthy("did:nexa:test"));
    }
}