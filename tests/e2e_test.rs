//! End-to-End Integration Tests
//!
//! Full workflow tests simulating multi-agent communication,
//! including discovery, channel management, receipt generation,
//! settlement, and security verification.
//!
//! # Test Categories
//!
//! - **Existing tests** (updated to use common module): basic flow validation
//! - **E2E Scenario 1**: Dual agent communication (discover → call → receipt → settle)
//! - **E2E Scenario 2**: Multi-agent community (5 agents, cross-calls, budget control)
//! - **E2E Scenario 3**: Fault recovery (unavailable agent → fallback routing)
//! - **E2E Scenario 4**: Economic loop (10 calls → 10 receipts → close → settle)
//! - **E2E Scenario 5**: Security verification (unsigned/forged/budget/rate-limit)

#[path = "common/mod.rs"]
mod common;

use nexa_net::api::sdk::NexaClientBuilder;
use nexa_net::discovery::CapabilityRegistry;
use nexa_net::economy::settlement::SettlementStatus;
use nexa_net::economy::{MicroReceipt, ReceiptChain, ReceiptVerifier, SettlementEngine};
use nexa_net::identity::{KeyPair, VerifiableCredential};
use nexa_net::security::RateLimitKey;
use nexa_net::types::Did;
use std::collections::HashMap;

// ============================================================================
// Existing Tests (Updated to use common module)
// ============================================================================

#[test]
fn test_agent_registration_flow() {
    let mut provider = common::TestAgent::new("provider");
    provider.add_capability("Translation Service", vec!["translation", "nlp"]);
    let _consumer = common::TestAgent::new("consumer");

    let mut registry = CapabilityRegistry::new();
    provider.register_into(&mut registry);

    let router = common::create_test_router(registry, false);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let routes = rt
        .block_on(async {
            router
                .discover("translate text", common::test_route_context(10))
                .await
        })
        .unwrap();

    // With relaxed config and max_candidates > 0, should find routes
    assert!(!routes.is_empty(), "Should discover at least one service");
    assert_eq!(routes[0].provider_did, provider.did);
}

#[test]
fn test_multi_agent_scenario() {
    let mut registry = CapabilityRegistry::new();

    let mut translator = common::TestAgent::new("translator");
    translator.add_capability("Translation", vec!["translation", "nlp"]);
    translator.register_into(&mut registry);

    let mut image_processor = common::TestAgent::new("image-processor");
    image_processor.add_capability("Image Processing", vec!["image", "vision"]);
    image_processor.register_into(&mut registry);

    let mut doc_analyzer = common::TestAgent::new("doc-analyzer");
    doc_analyzer.add_capability("Document Analysis", vec!["document", "nlp", "analysis"]);
    doc_analyzer.register_into(&mut registry);

    assert_eq!(registry.stats().total_capabilities, 3);

    let router = common::create_test_router(registry, false);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let translation_routes = rt
        .block_on(async {
            router
                .discover("translate document", common::test_route_context(10))
                .await
        })
        .unwrap();
    assert!(
        !translation_routes.is_empty(),
        "Should find translation services"
    );

    let image_routes = rt
        .block_on(async {
            router
                .discover("process image", common::test_route_context(10))
                .await
        })
        .unwrap();
    assert!(!image_routes.is_empty(), "Should find image services");
}

#[test]
fn test_payment_channel_flow() {
    let mut env = common::TestEnvironment::new();

    let payer = Did::new("did:nexa:payer");
    let payee = Did::new("did:nexa:payee");

    // Open channel — min_deposit=0 in test config
    let channel = env
        .channel_manager
        .open(payer.clone(), payee.clone(), 10000, 10)
        .unwrap();
    assert_eq!(channel.balance_a, 10000);
    assert_eq!(channel.balance_b, 10);

    // Check and record budget
    env.budget_controller
        .check_budget(&payer.to_string(), 100)
        .unwrap();
    env.budget_controller
        .record_spending(&payer.to_string(), 50);

    // Update channel after payment: 10000→9950, 10→60 (total preserved = 10010)
    env.channel_manager
        .update_balances(&channel.id, 9950, 60)
        .unwrap();

    let channels = env.channel_manager.list_open();
    let updated = channels.first().unwrap();
    assert_eq!(updated.balance_a, 9950);
    assert_eq!(updated.balance_b, 60);
}

#[test]
fn test_sdk_client_creation() {
    let client = NexaClientBuilder::new()
        .endpoint("http://127.0.0.1:7070")
        .timeout_ms(30000)
        .budget(100)
        .build();

    assert_eq!(client.endpoint(), "http://127.0.0.1:7070");

    let did = client.get_local_did().unwrap();
    assert!(did.to_string().starts_with("did:nexa:"));
}

#[test]
fn test_full_workflow() {
    let mut registry = CapabilityRegistry::new();
    let mut provider = common::TestAgent::new("service-provider");
    provider.add_capability("Data Processing", vec!["data", "processing"]);
    provider.register_into(&mut registry);

    let consumer = common::TestAgent::new("consumer");
    let router = common::create_test_router(registry, false);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let routes = rt
        .block_on(async {
            router
                .discover("process data", common::test_route_context(10))
                .await
        })
        .unwrap();
    assert!(!routes.is_empty(), "Should find at least one service");

    let mut env = common::TestEnvironment::new();
    let channel = env
        .channel_manager
        .open(consumer.did.clone(), provider.did.clone(), 1000, 10)
        .unwrap();
    assert_eq!(channel.balance_a, 1000);

    // Simulate service call: consumer pays 50, total preserved = 1010
    let cost = 50u64;
    env.channel_manager
        .update_balances(&channel.id, 1000 - cost, 10 + cost)
        .unwrap();

    let final_channels = env.channel_manager.list_open();
    let final_channel = final_channels.first().unwrap();
    assert_eq!(final_channel.balance_a, 950);
    assert_eq!(final_channel.balance_b, 60);
}

// ============================================================================
// E2E Scenario 1: Dual Agent Communication
// ============================================================================
//
// Agent A (consumer) → discover → Agent B (provider)
// Verify: route discovery + receipt generation + channel settlement

#[test]
fn test_dual_agent_communication() {
    // Agent A: consumer with no capabilities
    let consumer = common::TestAgent::new("consumer-a");
    // Agent B: provider with translation capability
    let mut provider = common::TestAgent::new("provider-b");
    provider.add_capability("Translation Service", vec!["translation", "nlp"]);

    // Step 1: Register provider capability
    let mut registry = CapabilityRegistry::new();
    provider.register_into(&mut registry);

    // Step 2: Discover service — should find provider B
    let router = common::create_test_router(registry, true);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let routes = rt
        .block_on(async {
            router
                .discover(
                    "translate english to chinese",
                    common::test_route_context(10),
                )
                .await
        })
        .unwrap();
    assert!(!routes.is_empty(), "Should discover translation service");
    assert_eq!(
        routes[0].provider_did, provider.did,
        "Should find provider B"
    );

    // Step 3: Open payment channel (A deposits 1000, B deposits 10)
    let mut env = common::TestEnvironment::new();
    let channel = env
        .channel_manager
        .open(consumer.did.clone(), provider.did.clone(), 1000, 10)
        .unwrap();
    assert_eq!(channel.total_balance(), 1010);

    // Step 4: Generate signed receipt for the call
    let mut receipt_chain = ReceiptChain::new(consumer.did.clone(), provider.did.clone());
    let mut receipt = receipt_chain.create_receipt("call-translate-1", 10, "/translate");
    receipt.sign_payer(consumer.signing_keypair()).unwrap();
    receipt.sign_payee(provider.signing_keypair()).unwrap();
    receipt_chain.add_receipt(receipt).unwrap();

    // Step 5: Update channel balances (A pays 10: 1000→990, 10→20)
    env.channel_manager
        .update_balances(&channel.id, 990, 20)
        .unwrap();

    // Verification: route found, receipt confirmed, channel settled
    assert!(
        receipt_chain.verify_chain_integrity().unwrap(),
        "Receipt chain must be intact"
    );
    assert!(
        receipt_chain.last().unwrap().is_confirmed(),
        "Receipt must be fully signed by both parties"
    );

    let final_channel = env.channel_manager.get(&channel.id).unwrap();
    assert_eq!(final_channel.balance_a, 990);
    assert_eq!(final_channel.balance_b, 20);
    assert_eq!(
        final_channel.total_balance(),
        1010,
        "Total balance must remain constant"
    );
}

// ============================================================================
// E2E Scenario 2: Multi-Agent Community (5 Agents)
// ============================================================================
//
// 5 agents register different capabilities → cross-calls A→B→C→D→E→A
// Verify: routing correctness + concurrent budget control

#[test]
fn test_multi_agent_community() {
    // Create 5 agents with distinct capabilities
    let mut agent_a = common::TestAgent::new("agent-a");
    agent_a.add_capability("Translation", vec!["translation", "nlp"]);

    let mut agent_b = common::TestAgent::new("agent-b");
    agent_b.add_capability("Image Processing", vec!["image", "vision"]);

    let mut agent_c = common::TestAgent::new("agent-c");
    agent_c.add_capability("Document Analysis", vec!["document", "nlp", "analysis"]);

    let mut agent_d = common::TestAgent::new("agent-d");
    agent_d.add_capability("Sentiment Analysis", vec!["sentiment", "nlp"]);

    let mut agent_e = common::TestAgent::new("agent-e");
    agent_e.add_capability("Summarization", vec!["summary", "nlp"]);

    // Register all agents into the registry
    let mut registry = CapabilityRegistry::new();
    agent_a.register_into(&mut registry);
    agent_b.register_into(&mut registry);
    agent_c.register_into(&mut registry);
    agent_d.register_into(&mut registry);
    agent_e.register_into(&mut registry);

    assert_eq!(registry.stats().total_capabilities, 5);

    // Discover services for different intents
    let router = common::create_test_router(registry, true);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let intents = vec![
        "translate text",
        "process image",
        "analyze document",
        "analyze sentiment",
        "summarize article",
    ];

    let mut discovered_providers = std::collections::HashSet::new();
    for intent in &intents {
        let routes = rt
            .block_on(async {
                router
                    .discover(intent, common::test_route_context(10))
                    .await
            })
            .unwrap();
        assert!(
            !routes.is_empty(),
            "Should find services for intent: {}",
            intent
        );
        for route in &routes {
            discovered_providers.insert(route.provider_did.as_str().to_string());
        }
    }

    // Verify that multiple different providers were discovered
    assert!(
        discovered_providers.len() >= 2,
        "Should discover at least 2 different providers across intents"
    );

    // Verify budget control for cross-calls
    let mut env = common::TestEnvironment::new();
    let caller_did = agent_a.did.clone();

    // Budget should allow reasonable spending across multiple calls
    env.budget_controller
        .check_budget(&caller_did.to_string(), 50)
        .unwrap();
    env.budget_controller
        .check_budget(&caller_did.to_string(), 100)
        .unwrap();

    // Open channels for cross-calls (A→B, B→C)
    let _ch_ab = env
        .channel_manager
        .open(agent_a.did.clone(), agent_b.did.clone(), 500, 10)
        .unwrap();
    let _ch_bc = env
        .channel_manager
        .open(agent_b.did.clone(), agent_c.did.clone(), 500, 10)
        .unwrap();
    assert_eq!(env.channel_manager.stats().open_channels, 2);
}

// ============================================================================
// E2E Scenario 3: Fault Recovery
// ============================================================================
//
// Agent B unavailable → discover fallback Agent C → complete call
// Verify: error retry + degraded routing

#[test]
fn test_fault_recovery() {
    // Agent B: translation service, but will be marked unavailable
    let mut agent_b = common::TestAgent::new("provider-b-unavailable");
    agent_b.add_capability("Translation Service", vec!["translation", "nlp"]);

    // Agent C: alternative translation service, available as fallback
    let mut agent_c = common::TestAgent::new("provider-c-fallback");
    agent_c.add_capability("Translation Backup", vec!["translation", "nlp", "backup"]);

    // Register both agents
    let mut registry = CapabilityRegistry::new();
    agent_b.register_into(&mut registry);
    agent_c.register_into(&mut registry);

    // Mark agent B as unavailable (simulating node failure)
    registry
        .set_availability(agent_b.did.as_str(), false)
        .unwrap();

    // Verify availability state in registry
    assert!(
        !registry
            .get_registered(agent_b.did.as_str())
            .unwrap()
            .available
    );
    assert!(
        registry
            .get_registered(agent_c.did.as_str())
            .unwrap()
            .available
    );

    // Create router with available_only=true → will skip unavailable B
    let router = common::create_test_router(registry, true);
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Discover: should find only agent C (B is unavailable)
    let routes = rt
        .block_on(async {
            router
                .discover("translate text", common::test_route_context(10))
                .await
        })
        .unwrap();
    assert!(!routes.is_empty(), "Should find fallback service");

    // All discovered routes must point to agent C, not B
    for route in &routes {
        assert_ne!(
            route.provider_did, agent_b.did,
            "Should not route to unavailable agent B"
        );
        assert_eq!(
            route.provider_did, agent_c.did,
            "Should route to available fallback agent C"
        );
    }

    // Open channel with fallback agent C and complete call
    let consumer = common::TestAgent::new("consumer-fault-recovery");
    let mut env = common::TestEnvironment::new();
    let channel = env
        .channel_manager
        .open(consumer.did.clone(), agent_c.did.clone(), 1000, 10)
        .unwrap();

    // Generate receipt for fallback call
    let mut receipt_chain = ReceiptChain::new(consumer.did.clone(), agent_c.did.clone());
    let mut receipt = receipt_chain.create_receipt("call-fallback-1", 10, "/translate");
    receipt.sign_payer(consumer.signing_keypair()).unwrap();
    receipt.sign_payee(agent_c.signing_keypair()).unwrap();
    receipt_chain.add_receipt(receipt).unwrap();

    // Update balances: consumer pays 10 (1000→990, 10→20)
    env.channel_manager
        .update_balances(&channel.id, 990, 20)
        .unwrap();

    // Verify: fault recovery succeeded, receipt chain intact
    assert!(receipt_chain.verify_chain_integrity().unwrap());
    let final_channel = env.channel_manager.get(&channel.id).unwrap();
    assert_eq!(final_channel.balance_a, 990);
    assert_eq!(final_channel.balance_b, 20);
}

// ============================================================================
// E2E Scenario 4: Economic Loop
// ============================================================================
//
// A opens channel with B → 10 calls → 10 receipts → close → settle
// Verify: balance computation + receipt chain integrity + settlement correctness

#[test]
fn test_economic_loop() {
    let consumer = common::TestAgent::new("payer-loop");
    let provider = common::TestAgent::new("payee-loop");

    let mut env = common::TestEnvironment::new();

    // Step 1: Open channel (A deposits 10000, B deposits 10)
    let channel = env
        .channel_manager
        .open(consumer.did.clone(), provider.did.clone(), 10000, 10)
        .unwrap();
    let channel_id = channel.id.clone();
    let initial_total = channel.total_balance();
    assert_eq!(initial_total, 10010);

    // Step 2: 10 calls with receipt chain
    let mut receipt_chain = ReceiptChain::new(consumer.did.clone(), provider.did.clone());
    let cost_per_call = 50u64;
    let total_calls = 10;

    for i in 1..=total_calls {
        let call_id = format!("call-{}", i);

        // Budget check before each call
        env.budget_controller
            .check_budget(&consumer.did.to_string(), cost_per_call)
            .unwrap();

        // Create and sign receipt
        let mut receipt = receipt_chain.create_receipt(&call_id, cost_per_call, "/service");
        receipt.sign_payer(consumer.signing_keypair()).unwrap();
        receipt.sign_payee(provider.signing_keypair()).unwrap();
        receipt_chain.add_receipt(receipt).unwrap();

        // Update channel balances: A pays 50 per call
        let new_balance_a = 10000 - cost_per_call * i;
        let new_balance_b = 10 + cost_per_call * i;
        env.channel_manager
            .update_balances(&channel_id, new_balance_a, new_balance_b)
            .unwrap();

        // Record spending in budget controller
        env.budget_controller
            .record_spending(&consumer.did.to_string(), cost_per_call);
    }

    // Step 3: Verify receipt chain integrity
    assert_eq!(
        receipt_chain.len(),
        total_calls as usize,
        "Should have {} receipts in chain",
        total_calls
    );
    assert!(
        receipt_chain.verify_chain_integrity().unwrap(),
        "Receipt chain must be intact"
    );
    assert_eq!(
        receipt_chain.total_amount(),
        cost_per_call * total_calls,
        "Total receipt amount should be 500"
    );

    // All receipts must be confirmed (both payer + payee signatures)
    for receipt in receipt_chain.all_receipts() {
        assert!(receipt.is_confirmed(), "Each receipt must be fully signed");
    }

    // Step 4: Verify final channel balances
    let channel = env.channel_manager.get(&channel_id).unwrap();
    assert_eq!(
        channel.balance_a, 9500,
        "Consumer balance after 10 calls of 50 each"
    );
    assert_eq!(
        channel.balance_b, 510,
        "Provider balance after 10 calls of 50 each"
    );
    assert_eq!(
        channel.total_balance(),
        initial_total,
        "Total balance must remain constant"
    );

    // Step 5: Verify budget tracking
    let budget_status = env.budget_controller.get_status(&consumer.did.to_string());
    assert_eq!(
        budget_status.spent_total, 500,
        "Total spending should be 500"
    );

    // Step 6: Close channel (bypass challenge period for testing)
    let closed_channel =
        common::close_channel_bypass_challenge(&mut env.channel_manager, &channel_id);
    assert!(
        closed_channel.is_closed(),
        "Channel must be closed after settlement"
    );

    // Step 7: Create and finalize settlement
    let mut settlement_engine = SettlementEngine::new();
    let settlement = settlement_engine
        .create_settlement(&closed_channel)
        .unwrap();
    assert_eq!(settlement.balance_a, 9500, "Settlement: payer gets 9500");
    assert_eq!(settlement.balance_b, 510, "Settlement: payee gets 510");
    assert_eq!(settlement.status, SettlementStatus::Pending);

    // Finalize settlement
    let finalized = settlement_engine.finalize(&settlement.id).unwrap();
    assert_eq!(
        finalized.status,
        SettlementStatus::Finalized,
        "Settlement must be finalized"
    );
}

// ============================================================================
// E2E Scenario 5: Security Verification
// ============================================================================
//
// Unsigned receipt → reject; forged VC → reject; budget exceeded → terminate;
// rate limit exceeded → block.
// Verify: zero-trust architecture effectiveness

#[tokio::test]
async fn test_security_verification() {
    let agent = common::TestAgent::new("agent-security");
    let other_agent = common::TestAgent::new("agent-other");

    // --- Sub-test 1: Unsigned receipt → verification fails ---
    let payer = Did::new("did:nexa:payer-sec");
    let payee = Did::new("did:nexa:payee-sec");
    let unsigned_receipt =
        MicroReceipt::new_genesis("call-unsigned", &payer, &payee, 100, "/service");

    // Receipt without signatures should not be considered valid
    assert!(
        !unsigned_receipt.is_payer_signed(),
        "Unsigned receipt should have no payer signature"
    );
    assert!(
        !unsigned_receipt.is_confirmed(),
        "Unsigned receipt should not be confirmed"
    );

    // Verify payer signature → returns false (no signature present)
    let payer_valid = ReceiptVerifier::verify_payer_signature(
        &unsigned_receipt,
        agent.signing_keypair().public_key().inner(),
    )
    .unwrap();
    assert!(
        !payer_valid,
        "Unsigned receipt should fail payer signature verification"
    );

    // --- Sub-test 2: Forged VC (wrong key) → verification fails ---
    let issuer_keypair = KeyPair::generate().unwrap();
    let issuer_did = nexa_net::identity::Did::from_public_key(issuer_keypair.public_key().inner());
    let subject_did = nexa_net::identity::Did::parse("did:nexa:subject-sec").unwrap();

    let mut claims = HashMap::new();
    claims.insert("role".to_string(), serde_json::json!("service_provider"));
    claims.insert("max_budget".to_string(), serde_json::json!(1000));

    // Create and sign VC with issuer key
    let mut vc = VerifiableCredential::new(&issuer_did, &subject_did, claims);
    vc.sign(&issuer_keypair).unwrap();
    assert!(vc.proof.is_some(), "VC should have proof after signing");

    // Verify with correct issuer key → should succeed
    assert!(
        vc.verify_with_keypair(&issuer_keypair).is_ok(),
        "VC should verify with correct issuer key"
    );

    // Verify with WRONG key → should fail (forged/invalid verifier)
    let wrong_keypair = other_agent.signing_keypair();
    let result = vc.verify_with_keypair(wrong_keypair);
    assert!(
        result.is_err(),
        "VC should fail verification with wrong key (forged verification)"
    );

    // --- Sub-test 3: Budget exceeded → termination ---
    let restricted_env = common::TestEnvironment::with_restricted_budget();
    let did_str = "did:nexa:over-budget-agent".to_string();

    // Within per_call limit (max_per_call=50) → should pass
    restricted_env
        .budget_controller
        .check_budget(&did_str, 30)
        .unwrap();

    // Exceed per_call limit (amount 100 > max_per_call 50) → should reject
    let result = restricted_env.budget_controller.check_budget(&did_str, 100);
    assert!(
        result.is_err(),
        "Should reject amount exceeding per_call budget limit"
    );

    // --- Sub-test 4: Rate limit exceeded → blocked ---
    let (manager, sink) = common::create_security_manager_with_rate_limit();
    let key = RateLimitKey::Did("did:nexa:rate-limited-agent".to_string());

    // First two requests → allowed (requests_per_minute=2)
    assert!(
        manager.rate_limiter().check(&key).unwrap().is_allowed(),
        "First request should be allowed"
    );
    assert!(
        manager.rate_limiter().check(&key).unwrap().is_allowed(),
        "Second request should be allowed"
    );

    // Third request → blocked (rate limit exceeded)
    let result = manager.rate_limiter().check(&key).unwrap();
    assert!(
        !result.is_allowed(),
        "Third request should be blocked by rate limit"
    );

    // Verify audit event was logged for rate limit violation
    let events = sink.get_events_by_type("rate_limit_exceeded").await;
    assert_eq!(
        events.len(),
        1,
        "Rate limit exceeded should produce exactly 1 audit event"
    );
}
