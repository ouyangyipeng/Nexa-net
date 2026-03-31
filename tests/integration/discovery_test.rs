//! Discovery Integration Tests
//!
//! Tests for multi-agent service discovery and semantic routing.

use nexa_net::discovery::{CapabilityRegistry, SemanticRouter, RoutingWeights};
use nexa_net::types::{CapabilitySchema, ServiceMetadata, Did, RouteContext};

/// Create a test capability schema
fn create_capability(did: &str, name: &str, tags: Vec<&str>) -> CapabilitySchema {
    CapabilitySchema {
        version: "1.0.0".to_string(),
        metadata: ServiceMetadata {
            did: Did::new(did),
            name: name.to_string(),
            description: format!("{} service", name),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        },
        endpoints: vec![],
    }
}

#[test]
fn test_multi_agent_registration() {
    // Create registry
    let mut registry = CapabilityRegistry::new();
    
    // Register multiple agents with different capabilities
    let agent_a = create_capability("did:nexa:agent-a", "Translation Service", vec!["translation", "nlp"]);
    let agent_b = create_capability("did:nexa:agent-b", "Image Processing", vec!["image", "vision"]);
    let agent_c = create_capability("did:nexa:agent-c", "Document Analysis", vec!["document", "nlp"]);
    
    registry.register(agent_a).unwrap();
    registry.register(agent_b).unwrap();
    registry.register(agent_c).unwrap();
    
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
    let translator_1 = create_capability(
        "did:nexa:translator-1",
        "English-Chinese Translation",
        vec!["translation", "english", "chinese"]
    );
    let translator_2 = create_capability(
        "did:nexa:translator-2",
        "English-Japanese Translation",
        vec!["translation", "english", "japanese"]
    );
    
    registry.register(translator_1).unwrap();
    registry.register(translator_2).unwrap();
    
    // Create router
    let router = SemanticRouter::new(registry);
    
    // Discover services
    let rt = tokio::runtime::Runtime::new().unwrap();
    let routes = rt.block_on(async {
        router.discover("translate english to chinese", RouteContext::default()).await
    }).unwrap();
    
    // Should find at least one service
    assert!(!routes.is_empty());
}

#[test]
fn test_capability_tags_indexing() {
    let mut registry = CapabilityRegistry::new();
    
    // Register with overlapping tags
    registry.register(create_capability("did:nexa:svc1", "NLP Service", vec!["nlp", "translation"])).unwrap();
    registry.register(create_capability("did:nexa:svc2", "Translation API", vec!["translation", "api"])).unwrap();
    registry.register(create_capability("did:nexa:svc3", "Vision API", vec!["vision", "api"])).unwrap();
    
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
    let total = weights.similarity + weights.quality + weights.cost + weights.load + weights.latency;
    assert!((total - 1.0).abs() < 0.01);
    
    // Verify validation
    assert!(weights.validate());
}