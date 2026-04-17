//! REST API Server using Axum
//!
//! Implements the REST API endpoints for Nexa-Proxy as defined in
//! ARCHITECTURE.md and API_REFERENCE.md.
//!
//! # Endpoints
//!
//! - `POST /v1/call` — Invoke a remote capability
//! - `POST /v1/register` — Register a local capability
//! - `POST /v1/discover` — Discover capabilities by intent
//! - `GET /v1/channels` — List open state channels
//! - `GET /v1/balance/:did` — Get token balance for a DID
//! - `GET /v1/status` — Get proxy status
//! - `GET /v1/health` — Health check

use crate::error::{Error, Result};
use crate::proxy::server::{ProxyState, ProxyStats};
use crate::types::{Did, EndpointDefinition};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

// ============================================================================
// API Request/Response Types (JSON-friendly, separate from internal types)
// ============================================================================

/// Request body for /v1/call endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallRequest {
    /// Intent description (e.g., "translate English PDF to Chinese")
    pub intent: String,
    /// Target service DID (optional, for direct calls)
    pub target_did: Option<String>,
    /// Input data payload (base64-encoded)
    pub input_data: Option<String>,
    /// Maximum budget for this call
    pub max_budget: Option<u64>,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

/// Response body for /v1/call endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallResponse {
    /// Call ID
    pub call_id: String,
    /// Result data (base64-encoded, if successful)
    pub result_data: Option<String>,
    /// Result MIME type
    pub result_type: Option<String>,
    /// Cost of the call
    pub cost: u64,
    /// Status of the call ("success" or "error")
    pub status: String,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Request body for /v1/register endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRegisterRequest {
    /// Service name
    pub name: String,
    /// Service description
    pub description: String,
    /// Service tags
    pub tags: Vec<String>,
    /// Endpoint address (for service routing)
    pub endpoint: String,
    /// Cost per call in NEXA tokens
    pub cost_per_call: u64,
}

/// Response body for /v1/register endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRegisterResponse {
    /// Assigned DID for the service
    pub did: String,
    /// Registration status
    pub status: String,
}

/// Request body for /v1/discover endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDiscoverRequest {
    /// Intent description
    pub intent: String,
    /// Maximum number of results
    pub max_results: Option<usize>,
    /// Minimum similarity threshold (0.0 - 1.0)
    pub threshold: Option<f32>,
}

/// Response body for /v1/discover endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDiscoverResponse {
    /// Discovered routes
    pub routes: Vec<ApiRouteInfo>,
}

/// Route information in discover response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRouteInfo {
    /// Provider DID
    pub provider_did: String,
    /// Service name
    pub service_name: String,
    /// Endpoint name
    pub endpoint_name: String,
    /// Similarity score
    pub similarity: f32,
    /// Estimated cost
    pub estimated_cost: u64,
    /// Estimated latency ms
    pub estimated_latency_ms: u64,
}

/// Channel info in list channels response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChannelInfo {
    /// Channel ID
    pub channel_id: String,
    /// Party A DID
    pub party_a: String,
    /// Party B DID
    pub party_b: String,
    /// Party A balance
    pub balance_a: u64,
    /// Party B balance
    pub balance_b: u64,
    /// Channel state
    pub state: String,
}

/// Balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiBalanceResponse {
    /// DID
    pub did: String,
    /// Total balance across channels
    pub total_balance: u64,
    /// Number of channels
    pub channel_count: usize,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiHealthResponse {
    /// Service status
    pub status: String,
    /// Version
    pub version: String,
    /// Uptime seconds
    pub uptime_seconds: u64,
}

/// Status response (wraps ProxyStats with additional info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStatusResponse {
    /// Proxy stats
    pub stats: ProxyStats,
}

// ============================================================================
// REST Server
// ============================================================================

/// REST API server using Axum
pub struct RestServer {
    /// Bind address
    bind: String,
    /// Port
    port: u16,
}

impl RestServer {
    /// Create a new REST server
    pub fn new(bind: &str, port: u16) -> Self {
        Self {
            bind: bind.to_string(),
            port,
        }
    }

    /// Build the Axum router with all API routes
    pub fn build_router(state: Arc<ProxyState>) -> Router {
        Router::new()
            .route("/v1/call", post(handle_call))
            .route("/v1/register", post(handle_register))
            .route("/v1/discover", post(handle_discover))
            .route("/v1/channels", get(handle_list_channels))
            .route("/v1/balance/{did}", get(handle_get_balance))
            .route("/v1/status", get(handle_status))
            .route("/v1/health", get(handle_health))
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }

    /// Start the REST server
    pub async fn start(&self, state: Arc<ProxyState>) -> Result<()> {
        let router = Self::build_router(state);
        let addr = format!("{}:{}", self.bind, self.port);

        tracing::info!("Starting REST API server on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| Error::Internal(format!("Failed to bind REST API: {}", e)))?;

        axum::serve(listener, router)
            .await
            .map_err(|e| Error::Internal(format!("REST API server error: {}", e)))?;

        Ok(())
    }
}

// ============================================================================
// Route Handlers
// ============================================================================

/// Handle /v1/call — invoke a remote capability
async fn handle_call(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<ApiCallRequest>,
) -> impl IntoResponse {
    tracing::info!("Call request: intent={}", request.intent);

    // Map API request to internal CallRequest
    let call_request = crate::proxy::server::CallRequest {
        intent: request.intent.clone(),
        data: request
            .input_data
            .and_then(|b| base64_decode(&b))
            .unwrap_or_default(),
        max_budget: request.max_budget.unwrap_or(100),
        timeout_ms: request.timeout_ms.unwrap_or(30000),
    };

    let result = crate::proxy::server::handlers::handle_call(state, call_request).await;

    match result {
        Ok(response) => {
            // Map internal CallResponse to API response
            let (result_data, result_type) = match &response.result {
                Some(call_result) => (
                    Some(base64_encode(&call_result.data)),
                    Some(call_result.data_type.clone()),
                ),
                None => (None, None),
            };
            let error_msg = response.error.map(|e| e.message.clone());

            let api_response = ApiCallResponse {
                call_id: response.call_id,
                result_data,
                result_type,
                cost: 0, // Internal CallResponse doesn't carry cost yet
                status: match response.status {
                    crate::types::CallStatus::Success => "success".to_string(),
                    crate::types::CallStatus::Error => "error".to_string(),
                    crate::types::CallStatus::Timeout => "timeout".to_string(),
                    crate::types::CallStatus::Cancelled => "cancelled".to_string(),
                },
                error: error_msg,
            };
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            let api_response = ApiCallResponse {
                call_id: uuid::Uuid::new_v4().to_string(),
                result_data: None,
                result_type: None,
                cost: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

/// Handle /v1/register — register a local capability
async fn handle_register(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<ApiRegisterRequest>,
) -> impl IntoResponse {
    tracing::info!("Register request: name={}", request.name);

    // Map API request to CapabilitySchema
    let did = Did::new(format!("did:nexa:{}", uuid::Uuid::new_v4()));
    let schema = crate::types::CapabilitySchema {
        version: "1.0.0".to_string(),
        metadata: crate::types::ServiceMetadata {
            did: did.clone(),
            name: request.name.clone(),
            description: request.description,
            tags: request.tags,
        },
        endpoints: vec![EndpointDefinition {
            id: "main".to_string(),
            name: request.name.clone(),
            description: String::new(),
            input_schema: serde_json::Value::Object(serde_json::Map::new()),
            output_schema: serde_json::Value::Object(serde_json::Map::new()),
            base_cost: request.cost_per_call,
            rate_limit: 100,
        }],
    };

    let result = crate::proxy::server::handlers::handle_register(state, schema).await;

    match result {
        Ok(_) => {
            let api_response = ApiRegisterResponse {
                did: did.as_str().to_string(),
                status: "registered".to_string(),
            };
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            let api_response = ApiRegisterResponse {
                did: String::new(),
                status: format!("error: {}", e),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

/// Handle /v1/discover — discover capabilities by intent
async fn handle_discover(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<ApiDiscoverRequest>,
) -> impl IntoResponse {
    tracing::info!("Discover request: intent={}", request.intent);

    let max_results = request.max_results.unwrap_or(10);

    let result =
        crate::proxy::server::handlers::handle_discover(state, &request.intent, max_results).await;

    match result {
        Ok(routes) => {
            let api_routes: Vec<ApiRouteInfo> = routes
                .iter()
                .take(max_results)
                .map(|r| ApiRouteInfo {
                    provider_did: r.provider_did.as_str().to_string(),
                    service_name: r.endpoint.name.clone(),
                    endpoint_name: r.endpoint.id.clone(),
                    similarity: r.similarity_score,
                    estimated_cost: r.estimated_cost,
                    estimated_latency_ms: r.estimated_latency_ms,
                })
                .collect();

            let api_response = ApiDiscoverResponse { routes: api_routes };
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            let api_response = ApiDiscoverResponse { routes: vec![] };
            tracing::error!("Discover error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

/// Handle /v1/channels — list open state channels
async fn handle_list_channels(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    tracing::info!("List channels request");

    let channels = crate::proxy::server::handlers::handle_list_channels(state).await;

    match channels {
        Ok(channels) => {
            let channel_list: Vec<ApiChannelInfo> = channels
                .iter()
                .map(|c| ApiChannelInfo {
                    channel_id: c.id.clone(),
                    party_a: c.party_a.as_str().to_string(),
                    party_b: c.party_b.as_str().to_string(),
                    balance_a: c.balance_a,
                    balance_b: c.balance_b,
                    state: match c.state {
                        crate::types::ChannelState::Open => "open".to_string(),
                        crate::types::ChannelState::Closing => "closing".to_string(),
                        crate::types::ChannelState::Closed => "closed".to_string(),
                        crate::types::ChannelState::Disputed => "disputed".to_string(),
                    },
                })
                .collect();
            (StatusCode::OK, Json(channel_list)).into_response()
        }
        Err(e) => {
            tracing::error!("List channels error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Vec::<ApiChannelInfo>::new()),
            )
                .into_response()
        }
    }
}

/// Handle /v1/balance/:did — get token balance
async fn handle_get_balance(
    State(state): State<Arc<ProxyState>>,
    Path(did_str): Path<String>,
) -> impl IntoResponse {
    tracing::info!("Balance request: did={}", did_str);

    let result = crate::proxy::server::handlers::handle_get_balance(state, &did_str).await;

    match result {
        Ok(balance) => {
            let api_response = ApiBalanceResponse {
                did: balance.did,
                total_balance: balance.total_balance,
                channel_count: balance.channel_count,
            };
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            let api_response = ApiBalanceResponse {
                did: did_str,
                total_balance: 0,
                channel_count: 0,
            };
            tracing::error!("Balance error: {}", e);
            (StatusCode::NOT_FOUND, Json(api_response)).into_response()
        }
    }
}

/// Handle /v1/status — get proxy status
async fn handle_status(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let stats = state.proxy_stats().await;
    let api_response = ApiStatusResponse { stats };
    (StatusCode::OK, Json(api_response)).into_response()
}

/// Handle /v1/health — health check
async fn handle_health() -> impl IntoResponse {
    let response = ApiHealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0,
    };
    (StatusCode::OK, Json(response)).into_response()
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Base64 encode bytes to string
fn base64_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Base64 decode string to bytes
fn base64_decode(data: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD.decode(data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rest_server_creation() {
        let server = RestServer::new("127.0.0.1", 7070);
        assert_eq!(server.port, 7070);
        assert_eq!(server.bind, "127.0.0.1");
    }

    #[test]
    fn test_api_call_request_serialization() {
        let request = ApiCallRequest {
            intent: "translate English to Chinese".to_string(),
            target_did: None,
            input_data: Some(base64_encode(b"hello world")),
            max_budget: Some(100),
            timeout_ms: Some(30000),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ApiCallRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.intent, request.intent);
        assert_eq!(deserialized.max_budget, request.max_budget);
    }

    #[test]
    fn test_api_health_response() {
        let response = ApiHealthResponse {
            status: "healthy".to_string(),
            version: "0.2.0".to_string(),
            uptime_seconds: 0,
        };
        assert_eq!(response.status, "healthy");
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"test data payload";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data.to_vec());
    }

    #[test]
    fn test_api_discover_request_serialization() {
        let request = ApiDiscoverRequest {
            intent: "find translation service".to_string(),
            max_results: Some(5),
            threshold: Some(0.7),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("find translation service"));
    }

    #[test]
    fn test_api_register_request_serialization() {
        let request = ApiRegisterRequest {
            name: "translation-service".to_string(),
            description: "English to Chinese translation".to_string(),
            tags: vec!["translation".to_string(), "nlp".to_string()],
            endpoint: "https://api.example.com/translate".to_string(),
            cost_per_call: 10,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("translation-service"));
    }

    #[test]
    fn test_api_channel_info_serialization() {
        let info = ApiChannelInfo {
            channel_id: "ch-123".to_string(),
            party_a: "did:nexa:alice".to_string(),
            party_b: "did:nexa:bob".to_string(),
            balance_a: 500,
            balance_b: 300,
            state: "open".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("ch-123"));
    }

    #[test]
    fn test_api_balance_response_serialization() {
        let response = ApiBalanceResponse {
            did: "did:nexa:alice".to_string(),
            total_balance: 1000,
            channel_count: 3,
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: ApiBalanceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_balance, 1000);
        assert_eq!(parsed.channel_count, 3);
    }
}
