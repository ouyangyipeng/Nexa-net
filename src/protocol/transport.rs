//! Transport protocol messages

use serde::{Deserialize, Serialize};

/// SYN-NEXA handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynNexaMessage {
    /// Intent hash
    pub intent_hash: String,
    /// Maximum budget
    pub max_budget: u64,
    /// Supported protocols
    pub supported_protocols: Vec<String>,
    /// Supported encodings
    pub supported_encodings: Vec<String>,
}

/// ACK-SCHEMA handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckSchemaMessage {
    /// Selected protocol
    pub selected_protocol: String,
    /// Selected encoding
    pub selected_encoding: String,
    /// Schema hash
    pub schema_hash: String,
    /// Estimated cost
    pub estimated_cost: u64,
    /// Estimated latency
    pub estimated_latency_ms: u64,
}

/// RPC call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCallRequest {
    /// Method name
    pub method: String,
    /// Request data
    pub data: Vec<u8>,
    /// Timeout ms
    pub timeout_ms: u64,
}

/// RPC call response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCallResponse {
    /// Response data
    pub data: Vec<u8>,
    /// Status
    pub status: String,
    /// Error message
    pub error: Option<String>,
}