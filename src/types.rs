//! Core types for Nexa-net

use serde::{Deserialize, Serialize};

// ============================================================================
// DID Types
// ============================================================================

/// Nexa DID identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Did(String);

impl Did {
    /// Create a new DID from string
    pub fn new(did: impl Into<String>) -> Self {
        Self(did.into())
    }

    /// Get the DID string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this is a valid Nexa DID
    pub fn is_valid(&self) -> bool {
        self.0.starts_with("did:nexa:") && self.0.len() > 10
    }
}

impl std::fmt::Display for Did {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Did {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Did {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

// ============================================================================
// Capability Types
// ============================================================================

/// Service capability schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySchema {
    /// Capability version
    pub version: String,
    /// Service metadata
    pub metadata: ServiceMetadata,
    /// Available endpoints
    pub endpoints: Vec<EndpointDefinition>,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetadata {
    /// Service DID
    pub did: Did,
    /// Service name
    pub name: String,
    /// Service description
    pub description: String,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Endpoint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDefinition {
    /// Endpoint ID
    pub id: String,
    /// Endpoint name
    pub name: String,
    /// Endpoint description
    pub description: String,
    /// Input schema (JSON Schema format)
    pub input_schema: serde_json::Value,
    /// Output schema (JSON Schema format)
    pub output_schema: serde_json::Value,
    /// Cost per call in NEXA tokens
    pub base_cost: u64,
    /// Rate limit (calls per minute)
    pub rate_limit: u32,
}

// ============================================================================
// Call Types
// ============================================================================

/// Network call request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallRequest {
    /// Intent description (semantic query)
    pub intent: String,
    /// Input data (binary)
    pub data: Vec<u8>,
    /// Data MIME type
    pub data_type: Option<String>,
    /// Maximum budget in NEXA tokens
    pub max_budget: u64,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Additional options
    pub options: std::collections::HashMap<String, serde_json::Value>,
}

/// Network call response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResponse {
    /// Call ID for tracking
    pub call_id: String,
    /// Response status
    pub status: CallStatus,
    /// Result data (if successful)
    pub result: Option<CallResult>,
    /// Error information (if failed)
    pub error: Option<CallError>,
    /// Cost in NEXA tokens
    pub cost: u64,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Provider information
    pub provider: Option<ProviderInfo>,
}

/// Call status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CallStatus {
    /// Call succeeded
    Success,
    /// Call failed
    Error,
    /// Call timed out
    Timeout,
    /// Call was cancelled
    Cancelled,
}

/// Call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResult {
    /// Result data (binary)
    pub data: Vec<u8>,
    /// Data MIME type
    pub data_type: String,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Call error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Additional details
    pub details: serde_json::Value,
}

/// Provider information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider DID
    pub did: Did,
    /// Endpoint ID used
    pub endpoint_id: String,
}

// ============================================================================
// Economy Types
// ============================================================================

/// Token amount (in micro-NEXA, 1 NEXA = 1,000,000 micro-NEXA)
pub type TokenAmount = u64;

/// Channel state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelState {
    /// Channel is open and active
    Open,
    /// Channel is closing
    Closing,
    /// Channel is closed
    Closed,
    /// Channel is in dispute
    Disputed,
}

/// Micro-receipt for a single call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroReceipt {
    /// Receipt ID
    pub receipt_id: String,
    /// Call ID
    pub call_id: String,
    /// Payer DID
    pub payer: Did,
    /// Payee DID
    pub payee: Did,
    /// Amount in micro-NEXA
    pub amount: TokenAmount,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Payer signature
    pub payer_signature: Vec<u8>,
    /// Payee signature (optional, for confirmed receipts)
    pub payee_signature: Option<Vec<u8>>,
}

// ============================================================================
// Routing Types
// ============================================================================

/// Routing context for semantic routing
#[derive(Debug, Clone, Default)]
pub struct RouteContext {
    /// Maximum number of candidates to return
    pub max_candidates: usize,
    /// Minimum similarity threshold (0.0 - 1.0)
    pub similarity_threshold: f32,
    /// Preferred providers (DIDs)
    pub preferred_providers: Vec<Did>,
    /// Excluded providers (DIDs)
    pub excluded_providers: Vec<Did>,
    /// Maximum latency in milliseconds
    pub max_latency_ms: Option<u64>,
    /// Maximum cost in NEXA
    pub max_cost: Option<u64>,
}

/// Routing result
#[derive(Debug, Clone)]
pub struct Route {
    /// Selected endpoint
    pub endpoint: EndpointDefinition,
    /// Provider DID
    pub provider_did: Did,
    /// Similarity score (0.0 - 1.0)
    pub similarity_score: f32,
    /// Estimated latency in milliseconds
    pub estimated_latency_ms: u64,
    /// Estimated cost in NEXA
    pub estimated_cost: u64,
}

// ============================================================================
// Protocol Types
// ============================================================================

/// Supported protocols
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum Protocol {
    /// Nexa RPC v1
    NexaRpcV1,
    /// gRPC
    Grpc,
    /// FlatBuffers over HTTP/2
    FlatBuffers,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::NexaRpcV1 => write!(f, "nexa-rpc-v1"),
            Protocol::Grpc => write!(f, "grpc"),
            Protocol::FlatBuffers => write!(f, "flatbuffers"),
        }
    }
}

/// Supported encodings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum Encoding {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// LZ4 compression
    Lz4,
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Encoding::None => write!(f, "none"),
            Encoding::Gzip => write!(f, "gzip"),
            Encoding::Lz4 => write!(f, "lz4"),
        }
    }
}

// ============================================================================
// Configuration Types
// ============================================================================

/// Nexa-Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Local API bind address
    pub api_bind: String,
    /// Local API port
    pub api_port: u16,
    /// gRPC port
    pub grpc_port: u16,
    /// Supernode addresses
    pub supernodes: Vec<String>,
    /// Default timeout in milliseconds
    pub default_timeout_ms: u64,
    /// Default budget in NEXA
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_creation() {
        let did = Did::new("did:nexa:abc123");
        assert_eq!(did.as_str(), "did:nexa:abc123");
        assert!(did.is_valid());
    }

    #[test]
    fn test_did_invalid() {
        let did = Did::new("invalid");
        assert!(!did.is_valid());
    }

    #[test]
    fn test_did_display() {
        let did = Did::new("did:nexa:test");
        assert_eq!(format!("{}", did), "did:nexa:test");
    }

    #[test]
    fn test_call_request_default() {
        let req = CallRequest::default();
        assert!(req.data.is_empty());
        assert_eq!(req.max_budget, 0);
    }

    #[test]
    fn test_proxy_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.api_port, 7070);
        assert_eq!(config.grpc_port, 7071);
    }
}
