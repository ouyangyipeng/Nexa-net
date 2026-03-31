//! End-to-End Integration Tests
//!
//! Full workflow tests simulating multi-agent communication.

use nexa_net::identity::{IdentityKeys, DidResolver};
use nexa_net::discovery::{CapabilityRegistry, SemanticRouter};
use nexa_net::economy::{ChannelManager, BudgetController};
use nexa_net::types::{Did, CapabilitySchema, ServiceMetadata, RouteContext};
use nexa_net::api::sdk::{NexaClient, NexaClientBuilder, CallOptions};

/// Simulated Agent for testing
struct TestAgent {
    did: Did,
    identity: IdentityKeys,
    capabilities: Vec<CapabilitySchema>,
}

impl TestAgent {
    fn new(name: &str) -> Self {
        Self {
            did: Did::new(&format!("did:nexa:{}", name)),
            identity: IdentityKeys::generate().unwrap(),
            capabilities: vec![],
        }
    }
    
    fn add_capability(&mut self, name: &str, tags: Vec<&str>) {
        let schema = CapabilitySchema {
            version: "1.0.0".to_string(),
            metadata: ServiceMetadata {
                did: self.did.clone(),
                name: name.to_string(),
                description: format!("{} service", name),
                tags: tags.iter().map(|s| s.to_string()).collect(),
            },
            endpoints: vec![],
        };
        self.capabilities.push(schema);
    }
}

#[test]
fn test_agent_registration_flow() {
    // Create agents
    let mut provider = TestAgent::new("provider");
    provider.add_capability("Translation Service", vec!["translation", "nlp"]);
    
    let consumer = TestAgent::new("consumer");
    
    // Register provider capability
    let mut registry = CapabilityRegistry::new();
    for cap in &provider.capabilities {
        registry.register(cap.clone()).unwrap();
    }
    
    // Consumer discovers service
    let router = SemanticRouter::new(registry);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let routes = rt.block_on(async {
        router.discover("translate text", RouteContext::default()).await
    }).unwrap();
    
    // Verify discovery
    assert!(!routes.is_empty());
    assert_eq!(routes[0].provider_did, provider.did);
}

#[test]
fn test_multi_agent_scenario() {
    // Create multiple providers
    let mut registry = CapabilityRegistry::new();
    
    // Provider 1: Translation
    let mut translator = TestAgent::new("translator");
    translator.add_capability("Translation", vec!["translation", "nlp"]);
    for cap in &translator.capabilities {
        registry.register(cap.clone()).unwrap();
    }
    
    // Provider 2: Image Processing
    let mut image_processor = TestAgent::new("image-processor");
    image_processor.add_capability("Image Processing", vec!["image", "vision"]);
    for cap in &image_processor.capabilities {
        registry.register(cap.clone()).unwrap();
    }
    
    // Provider 3: Document Analysis
    let mut doc_analyzer = TestAgent::new("doc-analyzer");
    doc_analyzer.add_capability("Document Analysis", vec!["document", "nlp", "analysis"]);
    for cap in &doc_analyzer.capabilities {
        registry.register(cap.clone()).unwrap();
    }
    
    // Verify all registered
    assert_eq!(registry.stats().total_capabilities, 3);
    
    // Test discovery for different intents
    let router = SemanticRouter::new(registry);
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Discover translation
    let translation_routes = rt.block_on(async {
        router.discover("translate document", RouteContext::default()).await
    }).unwrap();
    assert!(!translation_routes.is_empty());
    
    // Discover image processing
    let image_routes = rt.block_on(async {
        router.discover("process image", RouteContext::default()).await
    }).unwrap();
    assert!(!image_routes.is_empty());
}

#[test]
fn test_payment_channel_flow() {
    let mut channel_manager = ChannelManager::new();
    let mut budget_controller = BudgetController::new();
    
    // Create parties
    let payer = Did::new("did:nexa:payer");
    let payee = Did::new("did:nexa:payee");
    
    // Open channel
    let channel = channel_manager.open(payer.clone(), payee.clone(), 10000, 0).unwrap();
    assert_eq!(channel.balance_a, 10000);
    
    // Check budget
    budget_controller.check_budget(&payer.to_string(), 100).unwrap();
    
    // Record spending
    budget_controller.record_spending(&payer.to_string(), 50);
    
    // Update channel after payment
    channel_manager.update_balances("channel-1", 9950, 50).unwrap();
    
    // Verify final state
    let channels = channel_manager.list_open();
    let updated = channels.first().unwrap();
    assert_eq!(updated.balance_a, 9950);
    assert_eq!(updated.balance_b, 50);
}

#[test]
fn test_sdk_client_creation() {
    let client = NexaClientBuilder::new()
        .endpoint("http://127.0.0.1:7070")
        .timeout_ms(30000)
        .budget(100)
        .build();
    
    assert_eq!(client.endpoint(), "http://127.0.0.1:7070");
    
    // Test DID retrieval
    let did = client.get_local_did().unwrap();
    assert!(did.to_string().starts_with("did:nexa:"));
}

#[test]
fn test_full_workflow() {
    // This test simulates a complete workflow:
    // 1. Provider registers capability
    // 2. Consumer discovers service
    // 3. Payment channel is opened
    // 4. Service is called (simulated)
    // 5. Receipt is generated (simulated)
    
    // Setup
    let mut registry = CapabilityRegistry::new();
    let mut channel_manager = ChannelManager::new();
    
    // 1. Provider registration
    let provider = TestAgent::new("service-provider");
    let mut provider_with_cap = provider;
    provider_with_cap.add_capability("Data Processing", vec!["data", "processing"]);
    
    for cap in &provider_with_cap.capabilities {
        registry.register(cap.clone()).unwrap();
    }
    
    // 2. Consumer discovery
    let consumer = TestAgent::new("consumer");
    let router = SemanticRouter::new(registry);
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let routes = rt.block_on(async {
        router.discover("process data", RouteContext::default()).await
    }).unwrap();
    
    assert!(!routes.is_empty(), "Should find at least one service");
    
    // 3. Open payment channel
    let channel = channel_manager.open(
        consumer.did.clone(),
        provider_with_cap.did.clone(),
        1000,
        0
    ).unwrap();
    
    assert_eq!(channel.balance_a, 1000);
    
    // 4. Simulate service call (deduct payment)
    let cost = 50u64;
    channel_manager.update_balances("channel-1", 1000 - cost, cost).unwrap();
    
    // 5. Verify final state
    let final_channels = channel_manager.list_open();
    let final_channel = final_channels.first().unwrap();
    assert_eq!(final_channel.balance_a, 950);
    assert_eq!(final_channel.balance_b, 50);
}