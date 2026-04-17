//! Discovery Integration Tests
//!
//! Tests for multi-agent service discovery and semantic routing.
//! Updated to use common test utilities (TestAgent, create_test_router, test_route_context).

use super::common::{create_test_router, test_route_context, TestAgent};
use nexa_net::discovery::router::{RoutingConfig, RoutingWeights};
use nexa_net::discovery::CapabilityRegistry;

#[test]
fn test_multi_agent_registration() {
    // Create registry
    let mut registry = CapabilityRegistry::new();

    // Register multiple agents with different capabilities
    let mut agent_a = TestAgent::new("agent-a");
    agent_a.add_capability("Translation Service", vec!["translation", "nlp"]);

    let mut agent_b = TestAgent::new("agent-b");
    agent_b.add_capability("Image Processing", vec!["image", "vision"]);

    let mut agent_c = TestAgent::new("agent-c");
    agent_c.add_capability("Document Analysis", vec!["document", "nlp"]);

    agent_a.register_into(&mut registry);
    agent_b.register_into(&mut registry);
    agent_c.register_into(&mut registry);

    // Verify all registered
    assert!(registry.get("did:nexa:agent-a").is_some());
    assert!(registry.get("did:nexa:agent-b").is_some());
    assert!(registry.get("did:nexa:agent-c").is_some());
}

#[test]
fn test_semantic_discovery() {
    // Create router with registry
    let mut registry = CapabilityRegistry::new();

    // Register translation services
    let mut translator_1 = TestAgent::new("translator-1");
    translator_1.add_capability(
        "English-Chinese Translation",
        vec!["translation", "english", "chinese"],
    );

    let mut translator_2 = TestAgent::new("translator-2");
    translator_2.add_capability(
        "English-Japanese Translation",
        vec!["translation", "english", "japanese"],
    );

    translator_1.register_into(&mut registry);
    translator_2.register_into(&mut registry);

    // Create router with relaxed config for testing
    let router = create_test_router(registry, false);

    // Discover services
    let rt = tokio::runtime::Runtime::new().unwrap();
    let routes = rt
        .block_on(async {
            router
                .discover("translate english to chinese", test_route_context(10))
                .await
        })
        .unwrap();

    // Should find at least one service with relaxed config
    assert!(!routes.is_empty(), "Should discover at least one service");
}

#[test]
fn test_capability_tags_indexing() {
    let mut registry = CapabilityRegistry::new();

    let mut svc1 = TestAgent::new("svc1");
    svc1.add_capability("NLP Service", vec!["nlp", "translation"]);

    let mut svc2 = TestAgent::new("svc2");
    svc2.add_capability("Translation API", vec!["translation", "api"]);

    let mut svc3 = TestAgent::new("svc3");
    svc3.add_capability("Vision API", vec!["vision", "api"]);

    svc1.register_into(&mut registry);
    svc2.register_into(&mut registry);
    svc3.register_into(&mut registry);

    // Find by tag
    let translation_services = registry.find_by_tags(&["translation".to_string()]);
    assert_eq!(translation_services.len(), 2);

    let api_services = registry.find_by_tags(&["api".to_string()]);
    assert_eq!(api_services.len(), 2);
}

#[test]
fn test_routing_weights() {
    let weights = RoutingWeights {
        similarity: 0.4,
        quality: 0.3,
        cost: 0.15,
        load: 0.1,
        latency: 0.05,
    };

    // Verify weights sum to ~1.0
    let total =
        weights.similarity + weights.quality + weights.cost + weights.load + weights.latency;
    assert!((total - 1.0).abs() < 0.01);

    // Verify validation
    assert!(weights.validate());
}

#[test]
fn test_routing_config_defaults() {
    let config = RoutingConfig::default();
    assert_eq!(config.min_similarity, 0.5);
    assert_eq!(config.min_quality, 0.8);
    assert!(config.available_only);
    assert!(config.weights.validate());
}
