//! Identity protocol messages

use serde::{Deserialize, Serialize};

/// DID resolve request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidResolveRequest {
    /// DID to resolve
    pub did: String,
}

/// DID resolve response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidResolveResponse {
    /// DID document
    pub document: Option<serde_json::Value>,
    /// Metadata
    pub metadata: serde_json::Value,
}
