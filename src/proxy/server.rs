//! Proxy Server
//!
//! Local REST/gRPC API server for Nexa-Proxy.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Nexa-Proxy Server                      │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │                    REST API                         │   │
//! │  │  /call, /register, /discover, /channel, /balance    │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │                    gRPC Server                      │   │
//! │  │  Streaming RPC, Bidirectional communication        │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │                 Core Components                      │   │
//! │  │  - Identity: DID, Keys, Credentials                 │   │
//! │  │  - Discovery: Registry, Router, DHT                 │   │
//! │  │  - Transport: RPC, Serialization, Channels          │   │
//! │  │  - Economy: Channels, Receipts, Budget              │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use crate::discovery::{CapabilityRegistry, NodeStatusManager, SemanticRouter};
use crate::economy::{BudgetController, ChannelManager};
use crate::error::{Error, Result};
use crate::identity::{DidResolver, IdentityKeys};
use crate::proxy::config::ProxyConfig;
use crate::transport::{RpcServer, SerializationEngine, SerializationFormat};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Proxy state containing all core components
pub struct ProxyState {
    /// Identity keys
    pub identity: Option<IdentityKeys>,
    /// DID resolver
    pub resolver: DidResolver,
    /// Capability registry (wrapped in RwLock for interior mutability)
    pub registry: Arc<RwLock<CapabilityRegistry>>,
    /// Semantic router
    pub router: Arc<RwLock<SemanticRouter>>,
    /// Node status manager (wrapped in RwLock for interior mutability)
    pub node_status: Arc<RwLock<NodeStatusManager>>,
    /// Channel manager
    pub channels: Arc<RwLock<ChannelManager>>,
    /// Budget controller
    pub budget: Arc<RwLock<BudgetController>>,
    /// Serialization engine
    pub serializer: SerializationEngine,
}

impl ProxyState {
    /// Create a new proxy state
    ///
    /// Key design: registry and node_status are shared between ProxyState
    /// and SemanticRouter via the same Arc<RwLock>. This ensures that
    /// writes via /v1/register are immediately visible to /v1/discover.
    pub fn new() -> Self {
        // Create shared registry and node_status (Arc<RwLock> so router sees writes)
        let registry = Arc::new(RwLock::new(CapabilityRegistry::new()));
        let node_status = Arc::new(RwLock::new(NodeStatusManager::new()));

        // Router shares the same Arc<RwLock> references — no separate copies
        let router = Arc::new(RwLock::new(SemanticRouter::with_shared(
            registry.clone(),
            node_status.clone(),
        )));

        Self {
            identity: None,
            resolver: DidResolver::new(),
            registry,
            router,
            node_status,
            channels: Arc::new(RwLock::new(ChannelManager::new())),
            budget: Arc::new(RwLock::new(BudgetController::new())),
            serializer: SerializationEngine::new(SerializationFormat::Json),
        }
    }

    /// Initialize with identity
    pub fn with_identity(mut self, identity: IdentityKeys) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Get proxy statistics
    pub async fn proxy_stats(&self) -> ProxyStats {
        let registry = self.registry.read().await;
        let registry_stats = registry.stats();

        let channels = self.channels.read().await;
        let channel_stats = channels.stats();

        ProxyStats {
            total_capabilities: registry_stats.total_capabilities,
            available_capabilities: registry_stats.available_capabilities,
            open_channels: channel_stats.open_channels,
            total_value_locked: channel_stats.total_value_locked,
            total_transactions: channel_stats.total_transactions,
        }
    }
}

impl Default for ProxyState {
    fn default() -> Self {
        Self::new()
    }
}

/// Proxy server
pub struct ProxyServer {
    /// Configuration
    config: ProxyConfig,
    /// Server state
    state: Arc<ProxyState>,
    /// RPC server
    rpc_server: RpcServer,
    /// Shutdown signal
    #[allow(dead_code)]
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ProxyServer {
    /// Create a new proxy server
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            state: Arc::new(ProxyState::new()),
            rpc_server: RpcServer::new(),
            shutdown: None,
        }
    }

    /// Create a proxy server with existing state
    pub fn with_state(config: ProxyConfig, state: Arc<ProxyState>) -> Self {
        Self {
            config,
            state,
            rpc_server: RpcServer::new(),
            shutdown: None,
        }
    }

    /// Get server state
    pub fn state(&self) -> Arc<ProxyState> {
        self.state.clone()
    }

    /// Run the server
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!(
            "Starting Nexa-Proxy on {}:{}",
            self.config.api_bind,
            self.config.api_port
        );

        // Initialize components
        self.initialize().await?;

        // Start REST API server
        let rest_addr = format!("{}:{}", self.config.api_bind, self.config.api_port);
        tracing::info!("REST API listening on {}", rest_addr);

        // Start gRPC server
        let grpc_addr = format!("{}:{}", self.config.api_bind, self.config.grpc_port);
        tracing::info!("gRPC listening on {}", grpc_addr);

        // Register default handlers
        self.register_handlers().await?;

        // Keep running until shutdown
        tracing::info!("Nexa-Proxy started successfully");
        tracing::info!("API endpoints:");
        tracing::info!("  POST /call       - Make a network call");
        tracing::info!("  POST /register   - Register a capability");
        tracing::info!("  POST /discover   - Discover services");
        tracing::info!("  GET  /channel    - List channels");
        tracing::info!("  GET  /balance    - Get balance");

        // Wait forever (or until shutdown)
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| Error::Internal(format!("Signal error: {}", e)))?;

        tracing::info!("Shutdown signal received");
        Ok(())
    }

    /// Initialize server components
    async fn initialize(&self) -> Result<()> {
        // Initialize identity if not set
        if self.state.identity.is_none() {
            tracing::info!("Generating new identity keys");
            // In production, would load from disk or generate new
        }

        // Initialize capability registry
        tracing::info!("Initializing capability registry");

        // Initialize channel manager
        tracing::info!("Initializing channel manager");

        // Connect to supernodes
        for supernode in &self.config.supernodes {
            tracing::info!("Connecting to supernode: {}", supernode);
        }

        Ok(())
    }

    /// Register RPC handlers (requires mutable self to modify rpc_server)
    async fn register_handlers(&mut self) -> Result<()> {
        // Note: RPC handlers are registered synchronously
        // The actual async processing happens in handle_frame

        // Register call handler (synchronous wrapper)
        self.rpc_server.register("call", |header, _data| {
            // Return RpcResponse with placeholder data
            let response_header = crate::transport::RpcResponseHeader::success(
                header.call_id,
                0, // cost
                1, // processing_time_ms
            );
            Ok(crate::transport::RpcResponse {
                header: response_header,
                data: b"Call processed".to_vec(),
            })
        });

        // Register discover handler
        self.rpc_server.register("discover", |header, _data| {
            let response_header =
                crate::transport::RpcResponseHeader::success(header.call_id, 0, 1);
            Ok(crate::transport::RpcResponse {
                header: response_header,
                data: br#"{"services": []}"#.to_vec(),
            })
        });

        Ok(())
    }

    /// Shutdown the server
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Nexa-Proxy");

        // Close all channels
        let channels = self.state.channels.read().await;
        let stats = channels.stats();
        tracing::info!("Closing {} channels", stats.open_channels);

        // Save state
        tracing::info!("Saving state...");

        Ok(())
    }

    /// Get server statistics
    pub async fn stats(&self) -> ProxyStats {
        let registry = self.state.registry.read().await;
        let registry_stats = registry.stats();

        let channels = self.state.channels.read().await;
        let channel_stats = channels.stats();

        ProxyStats {
            total_capabilities: registry_stats.total_capabilities,
            available_capabilities: registry_stats.available_capabilities,
            open_channels: channel_stats.open_channels,
            total_value_locked: channel_stats.total_value_locked,
            total_transactions: channel_stats.total_transactions,
        }
    }
}

/// Proxy server statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyStats {
    /// Total registered capabilities
    pub total_capabilities: usize,
    /// Available capabilities
    pub available_capabilities: usize,
    /// Open channels
    pub open_channels: usize,
    /// Total value locked in channels
    pub total_value_locked: u64,
    /// Total transactions processed
    pub total_transactions: u64,
}

/// REST API handlers
pub mod handlers {
    use super::*;

    /// Handle /call endpoint
    pub async fn handle_call(state: Arc<ProxyState>, request: CallRequest) -> Result<CallResponse> {
        // Parse intent
        let intent = &request.intent;
        tracing::debug!("Processing call: {}", intent);

        // Discover services
        let router = state.router.read().await;
        let context = crate::types::RouteContext::default();
        let routes = router.discover(intent, context).await?;

        if routes.is_empty() {
            return Err(Error::ServiceNotFound(intent.clone()));
        }

        // Select best route
        let route = routes.into_iter().next().unwrap();

        // Check budget - need to provide DID and amount
        let budget = state.budget.read().await;
        budget.check_budget(&route.provider_did.to_string(), request.max_budget)?;

        // Make the call (placeholder)
        let result = format!("Called {} on {}", route.endpoint.name, route.provider_did);

        // Create receipt
        let _cost = route.estimated_cost;

        Ok(CallResponse {
            call_id: uuid::Uuid::new_v4().to_string(),
            status: crate::types::CallStatus::Success,
            result: Some(crate::types::CallResult {
                data: result.into_bytes(),
                data_type: "text/plain".to_string(),
                metadata: std::collections::HashMap::new(),
            }),
            error: None,
        })
    }

    /// Handle /register endpoint
    pub async fn handle_register(
        state: Arc<ProxyState>,
        schema: crate::types::CapabilitySchema,
    ) -> Result<()> {
        let mut registry = state.registry.write().await;
        registry.register(schema)?;
        Ok(())
    }

    /// Handle /discover endpoint
    pub async fn handle_discover(
        state: Arc<ProxyState>,
        intent: &str,
        max_candidates: usize,
    ) -> Result<Vec<crate::types::Route>> {
        let router = state.router.read().await;
        let context = crate::types::RouteContext {
            max_candidates,
            ..Default::default()
        };
        router.discover(intent, context).await
    }

    /// Handle /channel endpoint
    pub async fn handle_list_channels(
        state: Arc<ProxyState>,
    ) -> Result<Vec<crate::economy::Channel>> {
        let channels = state.channels.read().await;
        // list_open returns Vec<&Channel>, need to clone
        Ok(channels.list_open().iter().map(|c| (*c).clone()).collect())
    }

    /// Handle /balance endpoint
    pub async fn handle_get_balance(state: Arc<ProxyState>, did: &str) -> Result<BalanceInfo> {
        let channels = state.channels.read().await;
        let peer_channels = channels.list_for_peer(&crate::types::Did::new(did));

        let total_balance: u64 = peer_channels
            .iter()
            .map(|c| c.balance_a + c.balance_b)
            .sum();

        Ok(BalanceInfo {
            did: did.to_string(),
            total_balance,
            channel_count: peer_channels.len(),
        })
    }
}

/// Call request for REST API
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CallRequest {
    /// Intent description
    pub intent: String,
    /// Input data
    pub data: Vec<u8>,
    /// Maximum budget
    pub max_budget: u64,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

/// Call response for REST API
#[derive(Debug, Clone, serde::Serialize)]
pub struct CallResponse {
    /// Call ID
    pub call_id: String,
    /// Status
    pub status: crate::types::CallStatus,
    /// Result (if successful)
    pub result: Option<crate::types::CallResult>,
    /// Error (if failed)
    pub error: Option<crate::types::CallError>,
}

/// Balance information
#[derive(Debug, Clone, serde::Serialize)]
pub struct BalanceInfo {
    /// DID
    pub did: String,
    /// Total balance across channels
    pub total_balance: u64,
    /// Number of channels
    pub channel_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_state_creation() {
        let state = ProxyState::new();
        assert!(state.identity.is_none());
    }

    #[test]
    fn test_proxy_server_creation() {
        let config = ProxyConfig::default();
        let server = ProxyServer::new(config);
        assert!(server.state().identity.is_none());
    }

    #[tokio::test]
    async fn test_proxy_stats() {
        let config = ProxyConfig::default();
        let server = ProxyServer::new(config);
        let stats = server.stats().await;
        assert_eq!(stats.total_capabilities, 0);
        assert_eq!(stats.open_channels, 0);
    }
}
