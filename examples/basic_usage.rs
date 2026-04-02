//! Nexa-net Basic Usage Example
//!
//! This example demonstrates the core functionality of Nexa-net:
//! - Identity creation and management
//! - Capability registration
//! - Semantic discovery
//! - Payment channels

use nexa_net::{
    api::sdk::{CapabilityBuilder, NexaClientBuilder},
    discovery::capability::{CapabilityRegistry, QualityMetrics},
    economy::channel::ChannelManager,
    identity::IdentityKeys,
    types::{Did, EndpointDefinition},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Nexa-net Basic Usage Example ===\n");

    // =========================================================================
    // 1. Identity Management
    // =========================================================================
    println!("1. Creating Identity...");

    // Generate a new identity key pair
    let identity = IdentityKeys::generate()?;
    let did = Did::new("did:nexa:agent:example-agent");

    println!("   Created DID: {}", did.as_str());
    println!(
        "   Public key: {} bytes",
        identity.signing.public_key().to_bytes().len()
    );

    // =========================================================================
    // 2. Capability Registration
    // =========================================================================
    println!("\n2. Registering Capabilities...");

    // Create a capability registry
    let mut registry = CapabilityRegistry::new();

    // Build a capability using the builder pattern
    let capability = CapabilityBuilder::new(&did, "Translation Service")
        .description("High-quality neural machine translation service")
        .tag("translation")
        .tag("nlp")
        .tag("multilingual")
        .endpoint(EndpointDefinition {
            id: "translate-v1".to_string(),
            name: "Translate API".to_string(),
            description: "Translate text between languages".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "source_lang": {"type": "string"},
                    "target_lang": {"type": "string"}
                },
                "required": ["text", "target_lang"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "translated_text": {"type": "string"}
                }
            }),
            base_cost: 100,   // 100 micro-tokens per call
            rate_limit: 1000, // 1000 calls per minute
        })
        .build();

    // Register the capability
    registry.register(capability.clone())?;
    println!("   Registered capability: {}", capability.metadata.name);
    println!("   Tags: {:?}", capability.metadata.tags);

    // =========================================================================
    // 3. Semantic Discovery
    // =========================================================================
    println!("\n3. Semantic Discovery...");

    // Search by tags
    let results = registry.find_by_tags(&["translation".to_string()]);
    println!(
        "   Found {} capabilities matching 'translation'",
        results.len()
    );

    // Update quality metrics
    registry.update_quality(
        did.as_str(),
        QualityMetrics {
            success_rate: 0.98,
            avg_response_time_ms: 45.0,
            uptime: 0.99,
            total_calls: 1000,
            rating: 4.8,
        },
    )?;

    // Find high-quality capabilities
    let high_quality = registry.find_by_quality(0.95);
    println!(
        "   Found {} high-quality capabilities (>95%% success)",
        high_quality.len()
    );

    // =========================================================================
    // 4. Payment Channels
    // =========================================================================
    println!("\n4. Payment Channel Management...");

    // Create a channel manager
    let mut channel_manager = ChannelManager::default();

    // Create two parties
    let party_a = Did::new("did:nexa:agent:alice");
    let party_b = Did::new("did:nexa:agent:bob");

    // Open a payment channel
    let channel = channel_manager.open(
        party_a.clone(),
        party_b.clone(),
        10000, // Alice deposits 10,000 micro-tokens
        5000,  // Bob deposits 5,000 micro-tokens
    )?;

    println!("   Opened channel: {}", channel.id);
    println!("   Party A balance: {}", channel.balance_a);
    println!("   Party B balance: {}", channel.balance_b);
    println!(
        "   Total capacity: {}",
        channel.balance_a + channel.balance_b
    );

    // Simulate a payment (A pays B)
    channel_manager.update_balances(
        &channel.id,
        channel.balance_a - 500, // A loses 500
        channel.balance_b + 500, // B gains 500
    )?;
    println!(
        "   After payment: A={}, B={}",
        channel.balance_a - 500,
        channel.balance_b + 500
    );

    // =========================================================================
    // 5. SDK Client Usage
    // =========================================================================
    println!("\n5. SDK Client Usage...");

    // Build a client
    let client = NexaClientBuilder::new()
        .endpoint("http://localhost:7070")
        .timeout_ms(5000)
        .budget(100000) // Max 100,000 micro-tokens per session
        .build();

    println!("   Created SDK client for endpoint: {}", client.endpoint());
    println!("   Local DID: {:?}", client.get_local_did()?);

    // Note: These would make actual network calls if the proxy is running
    // For this example, we just show the API usage

    // =========================================================================
    // 6. Statistics
    // =========================================================================
    println!("\n6. Statistics...");

    let registry_stats = registry.stats();
    println!(
        "   Registry: {} capabilities, {} unique tags",
        registry_stats.total_capabilities, registry_stats.unique_tags
    );

    let channel_stats = channel_manager.stats();
    println!(
        "   Channels: {} open, {} total value locked",
        channel_stats.open_channels, channel_stats.total_value_locked
    );

    println!("\n=== Example Complete ===");
    Ok(())
}
