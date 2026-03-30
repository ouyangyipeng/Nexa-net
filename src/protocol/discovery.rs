//! Discovery protocol messages

use serde::{Deserialize, Serialize};

/// Register capability request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterCapabilityRequest {
    /// Capability schema
    pub schema: serde_json::Value,
}

/// Register capability response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterCapabilityResponse {
    /// Success
    pub success: bool,
    /// Registration ID
    pub registration_id: String,
}

/// Discover service request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverServiceRequest {
    /// Intent
    pub intent: String,
    /// Maximum results
    pub max_results: u32,
    /// Similarity threshold
    pub similarity_threshold: f32,
}

/// Discover service response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverServiceResponse {
    /// Found services
    pub services: Vec<ServiceMatch>,
}

/// Service match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMatch {
    /// Provider DID
    pub provider_did: String,
    /// Endpoint ID
    pub endpoint_id: String,
    /// Similarity score
    pub similarity: f32,
}