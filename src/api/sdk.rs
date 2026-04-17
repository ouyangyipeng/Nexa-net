//! SDK Interface
//!
//! High-level SDK for developers to interact with Nexa-Proxy.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Agent Application                       │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │                    NexaClient SDK                    │   │
//! │  │  - call(): Make network calls                        │   │
//! │  │  - register(): Register capabilities                  │   │
//! │  │  - discover(): Find services                         │   │
//! │  │  - stream(): Streaming calls                         │   │
//! │  │  - balance(): Query balance                           │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                             │                               │
//! │                             ▼                               │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │              Nexa-Proxy (Local Sidecar)              │   │
//! │  │  REST API: http://127.0.0.1:7070/api/v1              │   │
//! │  │  Unix Socket: /var/run/nexa-proxy.sock               │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use nexa_net::api::sdk::{NexaClientBuilder, CallOptions};
//!
//! // Create client
//! let client = NexaClientBuilder::new()
//!     .endpoint("http://127.0.0.1:7070")
//!     .timeout_ms(30000)
//!     .budget(100)
//!     .build();
//!
//! // Make a call (requires running proxy server)
//! // let response = client.call(
//! //     "translate English to Chinese",
//! //     b"Hello World".to_vec(),
//! //     CallOptions::new()
//! // ).await?;
//! ```

use crate::error::Result;
use crate::types::{
    CallRequest, CallResponse, CallResult, CallStatus, CapabilitySchema, Did, Route,
};
use std::collections::HashMap;

/// Nexa client for SDK users
pub struct NexaClient {
    /// Proxy endpoint (REST API base URL)
    endpoint: String,
    /// Default timeout in milliseconds
    default_timeout_ms: u64,
    /// Default budget per call
    default_budget: u64,
}

impl NexaClient {
    /// Create a new client with default settings
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            default_timeout_ms: 30000,
            default_budget: 100,
        }
    }

    /// Get the endpoint
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get the local DID (placeholder - would query proxy for actual DID)
    pub fn get_local_did(&self) -> Result<Did> {
        // In real implementation, this would query the proxy
        Ok(Did::new("did:nexa:local"))
    }

    /// Make a network call (simple interface)
    pub async fn call(
        &self,
        intent: &str,
        data: Vec<u8>,
        options: CallOptions,
    ) -> Result<CallResponse> {
        let request = CallRequest {
            intent: intent.to_string(),
            data,
            data_type: Some(options.data_type.clone()),
            max_budget: options.max_budget.unwrap_or(self.default_budget),
            timeout_ms: options.timeout_ms.unwrap_or(self.default_timeout_ms),
            options: options.metadata,
        };

        self.call_full(request).await
    }

    /// Make a full network call with complete request
    pub async fn call_full(&self, _request: CallRequest) -> Result<CallResponse> {
        // Placeholder implementation - in real code this would make HTTP request
        // to the proxy's REST API
        Ok(CallResponse {
            call_id: uuid::Uuid::new_v4().to_string(),
            status: CallStatus::Success,
            result: Some(CallResult {
                data: b"placeholder response".to_vec(),
                data_type: "text/plain".to_string(),
                metadata: HashMap::new(),
            }),
            error: None,
            cost: 10,
            latency_ms: 100,
            provider: None,
        })
    }

    /// Register a capability
    pub async fn register(&self, schema: CapabilitySchema) -> Result<()> {
        // Placeholder - would POST to /api/v1/capabilities
        tracing::debug!("Registering capability: {:?}", schema);
        Ok(())
    }

    /// Discover services matching an intent
    pub async fn discover(&self, intent: &str, _max_results: usize) -> Result<Vec<Route>> {
        // Placeholder - would POST to /api/v1/discover
        tracing::debug!("Discovering for intent: {}", intent);
        Ok(vec![])
    }

    /// Get list of open channels
    pub async fn list_channels(&self) -> Result<Vec<ChannelInfo>> {
        // Placeholder - would GET /api/v1/channels
        Ok(vec![])
    }

    /// Get balance for a DID
    pub async fn get_balance(&self, did: &str) -> Result<BalanceInfo> {
        // Placeholder - would GET /api/v1/balance/{did}
        Ok(BalanceInfo {
            did: did.to_string(),
            total_balance: 0,
            channel_count: 0,
        })
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        // Placeholder - would GET /api/v1/health
        Ok(true)
    }
}

/// Builder for NexaClient
pub struct NexaClientBuilder {
    endpoint: String,
    timeout_ms: u64,
    default_budget: u64,
}

impl NexaClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            endpoint: "http://127.0.0.1:7070".to_string(),
            timeout_ms: 30000,
            default_budget: 100,
        }
    }

    /// Set endpoint
    pub fn endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = endpoint.to_string();
        self
    }

    /// Set timeout
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set default budget
    pub fn budget(mut self, budget: u64) -> Self {
        self.default_budget = budget;
        self
    }

    /// Build the client
    pub fn build(self) -> NexaClient {
        NexaClient::new(&self.endpoint)
    }
}

impl Default for NexaClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Call options for customizing network calls
#[derive(Debug, Clone)]
pub struct CallOptions {
    /// Data MIME type
    pub data_type: String,
    /// Maximum budget for this call
    pub max_budget: Option<u64>,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CallOptions {
    /// Create default options
    pub fn new() -> Self {
        Self {
            data_type: "application/octet-stream".to_string(),
            max_budget: None,
            timeout_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Set data type
    pub fn with_data_type(mut self, data_type: &str) -> Self {
        self.data_type = data_type.to_string();
        self
    }

    /// Set max budget
    pub fn with_budget(mut self, budget: u64) -> Self {
        self.max_budget = Some(budget);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

impl Default for CallOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Discovery filters
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryFilters {
    /// Maximum cost filter
    pub max_cost: Option<u64>,
    /// Minimum quality score
    pub min_quality: Option<f32>,
    /// Region filter
    pub region: Option<String>,
}

impl DiscoveryFilters {
    /// Create new filters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max cost
    pub fn with_max_cost(mut self, cost: u64) -> Self {
        self.max_cost = Some(cost);
        self
    }

    /// Set min quality
    pub fn with_min_quality(mut self, quality: f32) -> Self {
        self.min_quality = Some(quality);
        self
    }

    /// Set region
    pub fn with_region(mut self, region: &str) -> Self {
        self.region = Some(region.to_string());
        self
    }
}

/// Channel information (simplified for SDK)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelInfo {
    /// Channel ID
    pub channel_id: String,
    /// Peer DID
    pub peer_did: String,
    /// Local balance
    pub local_balance: u64,
    /// Remote balance
    pub remote_balance: u64,
    /// Channel state
    pub state: String,
}

/// Balance information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BalanceInfo {
    /// DID
    pub did: String,
    /// Total balance
    pub total_balance: u64,
    /// Number of channels
    pub channel_count: usize,
}

/// Streaming call interface (placeholder for future implementation)
pub struct StreamCall {
    /// Stream ID
    pub stream_id: String,
    /// Intent
    pub intent: String,
}

impl StreamCall {
    /// Create a new stream call
    pub fn new(intent: &str) -> Self {
        Self {
            stream_id: uuid::Uuid::new_v4().to_string(),
            intent: intent.to_string(),
        }
    }

    /// Send a data chunk (placeholder)
    pub async fn send(&self, _data: Vec<u8>) -> Result<()> {
        // NOTE: Future — requires WebSocket/gRPC streaming support
        Ok(())
    }

    /// Receive data chunk (placeholder)
    pub async fn recv(&mut self) -> Result<Option<Vec<u8>>> {
        // NOTE: Future — requires WebSocket/gRPC streaming support
        Ok(None)
    }

    /// Close the stream
    pub async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// Capability registration helper
pub struct CapabilityBuilder {
    /// Service name
    name: String,
    /// Service description
    description: String,
    /// Service DID
    did: Did,
    /// Tags
    tags: Vec<String>,
    /// Endpoints
    endpoints: Vec<crate::types::EndpointDefinition>,
}

impl CapabilityBuilder {
    /// Create a new capability builder
    pub fn new(did: &Did, name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            did: did.clone(),
            tags: vec![],
            endpoints: vec![],
        }
    }

    /// Set description
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Add tag
    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Add endpoint
    pub fn endpoint(mut self, endpoint: crate::types::EndpointDefinition) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    /// Build the capability schema
    pub fn build(self) -> CapabilitySchema {
        CapabilitySchema {
            version: "1.0.0".to_string(),
            metadata: crate::types::ServiceMetadata {
                did: self.did,
                name: self.name,
                description: self.description,
                tags: self.tags,
            },
            endpoints: self.endpoints,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = NexaClientBuilder::new()
            .endpoint("http://localhost:8080")
            .timeout_ms(60000)
            .budget(200)
            .build();

        assert_eq!(client.endpoint(), "http://localhost:8080");
    }

    #[test]
    fn test_call_options() {
        let options = CallOptions::new()
            .with_data_type("application/json")
            .with_budget(50)
            .with_timeout(10000)
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(options.data_type, "application/json");
        assert_eq!(options.max_budget, Some(50));
        assert_eq!(options.timeout_ms, Some(10000));
    }

    #[test]
    fn test_discovery_filters() {
        let filters = DiscoveryFilters::new()
            .with_max_cost(100)
            .with_min_quality(0.8)
            .with_region("asia-east");

        assert_eq!(filters.max_cost, Some(100));
        assert_eq!(filters.min_quality, Some(0.8));
        assert_eq!(filters.region, Some("asia-east".to_string()));
    }

    #[test]
    fn test_capability_builder() {
        let did = Did::new("did:nexa:test123");
        let schema = CapabilityBuilder::new(&did, "My Service")
            .description("A test service")
            .tag("test")
            .tag("demo")
            .build();

        assert_eq!(schema.metadata.name, "My Service");
        assert_eq!(schema.metadata.description, "A test service");
        assert_eq!(schema.metadata.tags, vec!["test", "demo"]);
    }

    #[test]
    fn test_get_local_did() {
        let client = NexaClient::new("http://localhost:7070");
        let did = client.get_local_did().unwrap();
        assert!(did.to_string().starts_with("did:nexa:"));
    }

    #[test]
    fn test_balance_info() {
        let balance = BalanceInfo {
            did: "did:nexa:test".to_string(),
            total_balance: 1000,
            channel_count: 5,
        };
        assert_eq!(balance.did, "did:nexa:test");
        assert_eq!(balance.total_balance, 1000);
        assert_eq!(balance.channel_count, 5);
    }
}
