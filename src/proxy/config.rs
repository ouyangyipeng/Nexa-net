//! Proxy Configuration

use serde::{Deserialize, Serialize};

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// API bind address
    pub api_bind: String,
    /// API port
    pub api_port: u16,
    /// gRPC port
    pub grpc_port: u16,
    /// Supernode addresses
    pub supernodes: Vec<String>,
    /// Default timeout ms
    pub default_timeout_ms: u64,
    /// Default budget
    pub default_budget: u64,
    /// Log level
    pub log_level: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            api_bind: "127.0.0.1".to_string(),
            api_port: 7070,
            grpc_port: 7071,
            supernodes: vec![],
            default_timeout_ms: 30000,
            default_budget: 100,
            log_level: "info".to_string(),
        }
    }
}

impl ProxyConfig {
    /// Load configuration from file
    pub fn from_file(_path: &str) -> crate::error::Result<Self> {
        Ok(Self::default())
    }
}