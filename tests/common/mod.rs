//! Common test utilities for Nexa-net E2E and integration tests.
//!
//! Provides TestAgent, TestEnvironment, TestProxy, MockNetwork, and helper functions
//! for setting up multi-agent test scenarios without hard-coded wait times.
//! All operations are synchronous or event-driven.
//!
//! # TestProxy
//!
//! A lightweight HTTP test server that starts axum REST API on a random port
//! with graceful shutdown support. Uses `RestServer::build_router()` directly
//! instead of `RestServer::start()` to enable oneshot-based shutdown.
//!
//! # MockNetwork
//!
//! Simulates network topology with controlled availability states for
//! fault-recovery testing scenarios.

use nexa_net::api::rest::{
    ApiCallRequest, ApiCallResponse, ApiDiscoverRequest, ApiDiscoverResponse, ApiHealthResponse,
    RestServer,
};
use nexa_net::discovery::router::{RoutingConfig, RoutingWeights};
use nexa_net::discovery::{CapabilityRegistry, SemanticRouter};
use nexa_net::economy::channel::ChannelConfig;
use nexa_net::economy::{BudgetController, BudgetLimit, BudgetStatus, Channel, ChannelManager};
use nexa_net::identity::{IdentityKeys, KeyPair};
use nexa_net::proxy::server::ProxyState;
use nexa_net::security::audit::MemoryAuditSink;
use nexa_net::security::{RateLimitConfig, SecurityConfig, SecurityManager};
use nexa_net::types::{CapabilitySchema, Did, EndpointDefinition, RouteContext, ServiceMetadata};
use reqwest::Client;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

// ============================================================================
// TestAgent
// ============================================================================

/// Test agent with identity keys and registered capabilities.
///
/// Each TestAgent has:
/// - A deterministic DID (for test readability)
/// - Cryptographic identity (Ed25519 signing + X25519 key agreement)
/// - A list of capability schemas that can be registered into a CapabilityRegistry
pub struct TestAgent {
    /// Agent DID identifier
    pub did: Did,
    /// Cryptographic identity keys
    pub identity: IdentityKeys,
    /// Registered capability schemas
    pub capabilities: Vec<CapabilitySchema>,
}

impl TestAgent {
    /// Create a new test agent with a deterministic DID name.
    ///
    /// The DID format is `did:nexa:{name}` for test readability.
    /// Identity keys are randomly generated (Ed25519 + X25519).
    pub fn new(name: &str) -> Self {
        let identity = IdentityKeys::generate().unwrap();
        Self {
            did: Did::new(&format!("did:nexa:{}", name)),
            identity,
            capabilities: vec![],
        }
    }

    /// Add a capability with default cost (10 tokens per call).
    pub fn add_capability(&mut self, name: &str, tags: Vec<&str>) {
        let schema = CapabilitySchema {
            version: "1.0.0".to_string(),
            metadata: ServiceMetadata {
                did: self.did.clone(),
                name: name.to_string(),
                description: format!("{} service", name),
                tags: tags.iter().map(|s| s.to_string()).collect(),
            },
            endpoints: vec![EndpointDefinition {
                id: "default".to_string(),
                name: "default endpoint".to_string(),
                description: format!("Default endpoint for {}", name),
                input_schema: serde_json::json!({"type": "object"}),
                output_schema: serde_json::json!({"type": "object"}),
                base_cost: 10,
                rate_limit: 100,
            }],
        };
        self.capabilities.push(schema);
    }

    /// Add a capability with custom cost per call.
    pub fn add_capability_with_cost(&mut self, name: &str, tags: Vec<&str>, cost: u64) {
        let schema = CapabilitySchema {
            version: "1.0.0".to_string(),
            metadata: ServiceMetadata {
                did: self.did.clone(),
                name: name.to_string(),
                description: format!("{} service", name),
                tags: tags.iter().map(|s| s.to_string()).collect(),
            },
            endpoints: vec![EndpointDefinition {
                id: "default".to_string(),
                name: "default endpoint".to_string(),
                description: format!("Default endpoint for {}", name),
                input_schema: serde_json::json!({"type": "object"}),
                output_schema: serde_json::json!({"type": "object"}),
                base_cost: cost,
                rate_limit: 100,
            }],
        };
        self.capabilities.push(schema);
    }

    /// Get the signing KeyPair for receipt/VC operations.
    pub fn signing_keypair(&self) -> &KeyPair {
        &self.identity.signing
    }

    /// Register all agent capabilities into a CapabilityRegistry.
    pub fn register_into(&self, registry: &mut CapabilityRegistry) {
        for cap in &self.capabilities {
            registry.register(cap.clone()).unwrap();
        }
    }
}

// ============================================================================
// TestEnvironment
// ============================================================================

/// Lightweight test environment with economy components.
///
/// Holds ChannelManager and BudgetController for economic operations.
/// Note: CapabilityRegistry and SemanticRouter are NOT stored here
/// because `SemanticRouter::new()` consumes the registry (wraps in Arc).
/// Use `create_test_router()` to create a router from a built-up registry.
pub struct TestEnvironment {
    /// Channel manager with test-friendly config (min_deposit=0)
    pub channel_manager: ChannelManager,
    /// Budget controller with generous limits for testing
    pub budget_controller: BudgetController,
}

impl TestEnvironment {
    /// Create a new test environment with default test configuration.
    ///
    /// ChannelConfig: min_deposit=0, max_deposit=1M, challenge_period=3600s.
    /// BudgetLimit: generous limits suitable for most E2E scenarios.
    pub fn new() -> Self {
        Self {
            channel_manager: ChannelManager::with_config(ChannelConfig {
                min_deposit: 0,
                max_deposit: 1_000_000,
                challenge_period: Duration::from_secs(3600),
                max_channels_per_peer: 100,
            }),
            budget_controller: BudgetController::with_limits(BudgetLimit {
                max_per_call: 1000,
                max_per_minute: 5000,
                max_per_hour: 50000,
                max_per_day: 100000,
                max_total: 1_000_000,
            }),
        }
    }

    /// Create with restricted budget limits for budget-exceeded tests.
    ///
    /// max_per_call=50, max_total=500 — easy to exceed in tests.
    pub fn with_restricted_budget() -> Self {
        Self {
            channel_manager: ChannelManager::with_config(ChannelConfig {
                min_deposit: 0,
                max_deposit: 1_000_000,
                challenge_period: Duration::from_secs(3600),
                max_channels_per_peer: 100,
            }),
            budget_controller: BudgetController::with_limits(BudgetLimit {
                max_per_call: 50,
                max_per_minute: 100,
                max_per_hour: 500,
                max_per_day: 1000,
                max_total: 500,
            }),
        }
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TestProxy — HTTP test server with graceful shutdown
// ============================================================================

/// Lightweight HTTP test proxy that starts the axum REST API server
/// on a random port with graceful shutdown support.
///
/// # Usage
///
/// ```rust,ignore
/// let proxy = TestProxy::new();
/// let proxy = proxy.start().await;
///
/// // Register a capability
/// proxy.register_capability(schema).await;
///
/// // HTTP discover
/// let response = proxy.discover("translate text", 10).await;
///
/// // Shutdown
/// proxy.shutdown().await;
/// ```
pub struct TestProxy {
    /// ProxyState (core component collection shared with axum router)
    state: Arc<ProxyState>,
    /// REST API server address (e.g. "http://127.0.0.1:38291")
    address: String,
    /// oneshot sender for shutdown signal
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// tokio task handle for the server task
    server_handle: Option<JoinHandle<()>>,
}

impl TestProxy {
    /// Create a new TestProxy with a fresh ProxyState.
    ///
    /// Does NOT start the server — call `start()` to bind and serve.
    pub fn new() -> Self {
        Self {
            state: Arc::new(ProxyState::new()),
            address: String::new(),
            shutdown_tx: None,
            server_handle: None,
        }
    }

    /// Start the REST API server on a random port (127.0.0.1:0).
    ///
    /// Uses `RestServer::build_router()` + `axum::serve().with_graceful_shutdown()`
    /// instead of `RestServer::start()` which blocks forever without shutdown support.
    ///
    /// After starting, automatically configures the SemanticRouter with relaxed
    /// test settings (min_similarity=-1.0, min_quality=0.0) to ensure discovery
    /// works with the hash-based MockEmbedder.
    pub async fn start(mut self) -> Self {
        let router = RestServer::build_router(self.state.clone());

        // Bind to random port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind TCP listener for TestProxy");

        let port = listener
            .local_addr()
            .expect("Failed to get local addr")
            .port();
        self.address = format!("http://127.0.0.1:{}", port);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn server task with graceful shutdown
        let handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("TestProxy server error");
        });

        self.shutdown_tx = Some(shutdown_tx);
        self.server_handle = Some(handle);

        // Give the server a moment to start accepting connections
        // (event-driven: just check health endpoint)
        let client = Client::new();
        let health_url = format!("{}/v1/health", self.address);
        for _ in 0..20 {
            if client.get(&health_url).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Configure router with relaxed test settings (hash-based MockEmbedder
        // produces negative similarity scores, so min_similarity=-1.0 is needed)
        self.configure_router_for_testing(false).await;

        self
    }

    /// Configure the SemanticRouter with relaxed test settings.
    ///
    /// Uses `min_similarity=-1.0` (accept all cosine similarity) and
    /// `min_quality=0.0` to ensure discovery works in test environments.
    /// `available_only` controls whether unavailable services are filtered out.
    pub async fn configure_router_for_testing(&self, available_only: bool) {
        use nexa_net::discovery::router::RoutingConfig;

        let config = RoutingConfig {
            min_similarity: -1.0,
            min_quality: 0.0,
            available_only,
            max_cost: 0,
            max_latency_ms: 0,
            weights: nexa_net::discovery::router::RoutingWeights::default(),
        };

        // Recreate router with test-friendly config, sharing the same registry
        let new_router = nexa_net::discovery::SemanticRouter::with_shared(
            self.state.registry.clone(),
            self.state.node_status.clone(),
        )
        .with_config(config);

        let mut router_lock = self.state.router.write().await;
        *router_lock = new_router;
    }

    /// Send shutdown signal and wait for server task to complete.
    ///
    /// Drops the oneshot sender which triggers graceful shutdown in axum.
    pub async fn shutdown(self) {
        if let Some(tx) = self.shutdown_tx {
            // Send shutdown signal; if server already stopped, ignore error
            let _ = tx.send(());
        }
        if let Some(handle) = self.server_handle {
            let _ = handle.await;
        }
    }

    /// Get the server endpoint URL (e.g. "http://127.0.0.1:38291")
    pub fn endpoint(&self) -> &str {
        &self.address
    }

    /// Get the ProxyState for direct internal operations.
    pub fn state(&self) -> Arc<ProxyState> {
        self.state.clone()
    }

    // ========================================================================
    // Internal operations (direct ProxyState access, no HTTP)
    // ========================================================================

    /// Register a capability schema directly into ProxyState.registry.
    ///
    /// This bypasses the REST API and writes directly, which is useful
    /// for setting up test state before making HTTP calls.
    pub async fn register_capability(&self, schema: CapabilitySchema) {
        let mut registry = self.state.registry.write().await;
        registry
            .register(schema)
            .expect("Failed to register capability in TestProxy");
    }

    /// Set availability of a registered capability directly.
    pub async fn set_availability(&self, did: &str, available: bool) {
        let mut registry = self.state.registry.write().await;
        registry
            .set_availability(did, available)
            .expect("Failed to set availability in TestProxy");
    }

    /// Open a payment channel directly in ProxyState.channels.
    ///
    /// Returns the created Channel for verification.
    pub async fn open_channel(
        &self,
        payer: Did,
        payee: Did,
        deposit_a: u64,
        deposit_b: u64,
    ) -> Channel {
        let mut channels = self.state.channels.write().await;
        channels
            .open(payer, payee, deposit_a, deposit_b)
            .expect("Failed to open channel in TestProxy")
    }

    /// Update channel balances directly.
    pub async fn update_balances(&self, channel_id: &str, balance_a: u64, balance_b: u64) {
        let mut channels = self.state.channels.write().await;
        channels
            .update_balances(channel_id, balance_a, balance_b)
            .expect("Failed to update balances in TestProxy");
    }

    /// Close a channel (bypassing challenge period for tests).
    pub async fn close_channel_bypass(&self, channel_id: &str) -> Channel {
        let mut channels = self.state.channels.write().await;
        let channel = channels.get_mut(channel_id).expect("Channel not found");
        channel.initiate_close(Duration::from_secs(3600)).unwrap();
        // Bypass challenge period: set deadline to the past
        channel.settlement_deadline = Some(chrono::Utc::now() - chrono::Duration::seconds(1));
        channel.finalize_close().unwrap();
        channel.clone()
    }

    /// Get a channel by ID.
    pub async fn get_channel(&self, channel_id: &str) -> Channel {
        let channels = self.state.channels.read().await;
        channels.get(channel_id).expect("Channel not found").clone()
    }

    /// Check budget directly.
    pub async fn check_budget(&self, did: &str, amount: u64) -> nexa_net::error::Result<()> {
        let budget = self.state.budget.read().await;
        budget.check_budget(did, amount)
    }

    /// Record budget spending directly.
    pub async fn record_spending(&self, did: &str, amount: u64) {
        let mut budget = self.state.budget.write().await;
        budget.record_spending(did, amount);
    }

    /// Get budget status for a DID.
    pub async fn budget_status(&self, did: &str) -> BudgetStatus {
        let budget = self.state.budget.read().await;
        budget.get_status(did).clone()
    }

    /// Get registry stats.
    pub async fn registry_stats(&self) -> nexa_net::discovery::capability::RegistryStats {
        let registry = self.state.registry.read().await;
        registry.stats()
    }

    // ========================================================================
    // HTTP operations (via reqwest)
    // ========================================================================

    /// HTTP GET /v1/health — health check.
    pub async fn health(&self) -> ApiHealthResponse {
        let client = Client::new();
        let url = format!("{}/v1/health", self.address);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to call /v1/health");
        assert_eq!(response.status(), 200, "/v1/health must return 200");
        response
            .json::<ApiHealthResponse>()
            .await
            .expect("Failed to parse /v1/health response")
    }

    /// HTTP POST /v1/discover — discover capabilities by intent.
    pub async fn discover(&self, intent: &str, max_results: usize) -> ApiDiscoverResponse {
        let client = Client::new();
        let url = format!("{}/v1/discover", self.address);
        let request = ApiDiscoverRequest {
            intent: intent.to_string(),
            max_results: Some(max_results),
            threshold: None,
        };
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .expect("Failed to call /v1/discover");
        assert!(
            response.status() == 200 || response.status() == 500,
            "/v1/discover returned unexpected status: {}",
            response.status()
        );
        response
            .json::<ApiDiscoverResponse>()
            .await
            .expect("Failed to parse /v1/discover response")
    }

    /// HTTP POST /v1/call — invoke a remote capability.
    pub async fn call(&self, intent: &str, max_budget: u64) -> ApiCallResponse {
        let client = Client::new();
        let url = format!("{}/v1/call", self.address);
        let request = ApiCallRequest {
            intent: intent.to_string(),
            target_did: None,
            input_data: None,
            max_budget: Some(max_budget),
            timeout_ms: Some(5000),
        };
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .expect("Failed to call /v1/call");
        // Accept both 200 (success) and 500 (error) — both return ApiCallResponse
        response
            .json::<ApiCallResponse>()
            .await
            .expect("Failed to parse /v1/call response")
    }

    /// HTTP POST /v1/register — register a capability via REST API.
    pub async fn http_register(
        &self,
        name: &str,
        description: &str,
        tags: Vec<&str>,
        cost_per_call: u64,
    ) -> nexa_net::api::rest::ApiRegisterResponse {
        let client = Client::new();
        let url = format!("{}/v1/register", self.address);
        let request = nexa_net::api::rest::ApiRegisterRequest {
            name: name.to_string(),
            description: description.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            endpoint: format!("{}/service/{}", self.address, name),
            cost_per_call,
        };
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .expect("Failed to call /v1/register");
        response
            .json::<nexa_net::api::rest::ApiRegisterResponse>()
            .await
            .expect("Failed to parse /v1/register response")
    }

    /// HTTP GET /v1/channels — list open state channels.
    pub async fn list_channels(&self) -> Vec<nexa_net::api::rest::ApiChannelInfo> {
        let client = Client::new();
        let url = format!("{}/v1/channels", self.address);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to call /v1/channels");
        response
            .json::<Vec<nexa_net::api::rest::ApiChannelInfo>>()
            .await
            .expect("Failed to parse /v1/channels response")
    }

    /// HTTP GET /v1/balance/{did} — get balance for a DID.
    pub async fn get_balance(&self, did: &str) -> nexa_net::api::rest::ApiBalanceResponse {
        let client = Client::new();
        let url = format!("{}/v1/balance/{}", self.address, did);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to call /v1/balance");
        response
            .json::<nexa_net::api::rest::ApiBalanceResponse>()
            .await
            .expect("Failed to parse /v1/balance response")
    }

    /// HTTP GET /v1/status — get proxy status.
    pub async fn status(&self) -> nexa_net::api::rest::ApiStatusResponse {
        let client = Client::new();
        let url = format!("{}/v1/status", self.address);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to call /v1/status");
        response
            .json::<nexa_net::api::rest::ApiStatusResponse>()
            .await
            .expect("Failed to parse /v1/status response")
    }
}

// ============================================================================
// MockNetwork — simulated network topology for fault-recovery testing
// ============================================================================

/// Simulated network with controlled availability states.
///
/// Creates N TCP listener endpoints on random ports, then allows
/// marking individual endpoints as unavailable (simulating node failure)
/// for fault-recovery E2E test scenarios.
pub struct MockNetwork {
    /// Available endpoint addresses (e.g. "127.0.0.1:38291")
    endpoints: Vec<String>,
    /// Currently unavailable endpoints (simulating disconnected nodes)
    unavailable: HashSet<String>,
    /// TCP listeners held to keep ports alive
    #[allow(dead_code)]
    listeners: Vec<tokio::net::TcpListener>,
}

impl MockNetwork {
    /// Create a mock network with N endpoints on random ports.
    ///
    /// Each endpoint gets a `TcpListener::bind("127.0.0.1:0")` allocation,
    /// keeping the port reserved but not serving anything (simulating
    /// a real network node address).
    pub async fn new(n: usize) -> Self {
        let mut endpoints = Vec::with_capacity(n);
        let mut listeners = Vec::with_capacity(n);

        for _ in 0..n {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("Failed to bind mock endpoint");
            let addr = listener
                .local_addr()
                .expect("Failed to get mock endpoint addr");
            endpoints.push(addr.to_string());
            listeners.push(listener);
        }

        Self {
            endpoints,
            unavailable: HashSet::new(),
            listeners,
        }
    }

    /// Mark an endpoint as unavailable (simulating node failure/disconnection).
    pub fn mark_unavailable(&mut self, addr: &str) {
        self.unavailable.insert(addr.to_string());
    }

    /// Mark an endpoint as available again (simulating node recovery).
    pub fn mark_available(&mut self, addr: &str) {
        self.unavailable.remove(addr);
    }

    /// Check if an endpoint is currently available.
    pub fn is_available(&self, addr: &str) -> bool {
        !self.unavailable.contains(addr)
    }

    /// Get all endpoint addresses.
    pub fn endpoints(&self) -> &[String] {
        &self.endpoints
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a SemanticRouter with relaxed test configuration.
///
/// Uses `min_similarity=-1.0` (accept all cosine similarity values, since
/// the hash-based MockEmbedder can produce negative similarity scores) and
/// `min_quality=0.0` to ensure discovery works in test environments.
/// `available_only` controls whether unavailable services are filtered out.
pub fn create_test_router(registry: CapabilityRegistry, available_only: bool) -> SemanticRouter {
    let config = RoutingConfig {
        min_similarity: -1.0,
        min_quality: 0.0,
        available_only,
        max_cost: 0,
        max_latency_ms: 0,
        weights: RoutingWeights::default(),
    };
    SemanticRouter::new(registry).with_config(config)
}

/// Create a RouteContext with a reasonable max_candidates value.
///
/// Default RouteContext has `max_candidates=0` which returns no results.
/// This helper sets a usable default for testing.
pub fn test_route_context(max_candidates: usize) -> RouteContext {
    RouteContext {
        max_candidates,
        ..Default::default()
    }
}

/// Create a SecurityManager with an in-memory audit sink for testing.
///
/// Returns both the SecurityManager and the `Arc<MemoryAuditSink>`
/// so tests can inspect captured audit events.
pub fn create_security_manager_with_sink() -> (SecurityManager, Arc<MemoryAuditSink>) {
    let sink = Arc::new(MemoryAuditSink::new(1000));
    let config = SecurityConfig::default();
    let manager = SecurityManager::with_audit_sink(sink.clone(), config).unwrap();
    (manager, sink)
}

/// Create a SecurityManager with restricted rate limits for testing.
///
/// Allows only 2 requests per minute, suitable for rate-limit-exceeded tests.
pub fn create_security_manager_with_rate_limit() -> (SecurityManager, Arc<MemoryAuditSink>) {
    let sink = Arc::new(MemoryAuditSink::new(1000));
    let config = SecurityConfig {
        rate_limit: RateLimitConfig {
            requests_per_minute: 2,
            requests_per_hour: 100,
            requests_per_day: 1000,
            burst_size: 0,
            enabled: true,
        },
        ..Default::default()
    };
    let manager = SecurityManager::with_audit_sink(sink.clone(), config).unwrap();
    (manager, sink)
}

/// Close a channel bypassing the challenge period wait.
///
/// In production, closing requires waiting for the challenge period.
/// This test helper sets the settlement deadline to the past so
/// the channel can be closed immediately in tests.
pub fn close_channel_bypass_challenge(
    channel_manager: &mut ChannelManager,
    channel_id: &str,
) -> Channel {
    let channel = channel_manager.get_mut(channel_id).unwrap();
    channel.initiate_close(Duration::from_secs(3600)).unwrap();
    // Bypass challenge period: set deadline to the past
    channel.settlement_deadline = Some(chrono::Utc::now() - chrono::Duration::seconds(1));
    channel.finalize_close().unwrap();
    channel.clone()
}
