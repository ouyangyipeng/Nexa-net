//! HTTP-level End-to-End Tests for Nexa-net
//!
//! These tests verify the full REST API stack (JSON → axum handler → ProxyState → response → JSON)
//! using `TestProxy` which starts a real HTTP server on a random port.
//!
//! # Test Categories
//!
//! - **Scenario 1**: Dual agent communication (discover → call → channel)
//! - **Scenario 2**: Multi-agent community (5 agents, cross-discovery)
//! - **Scenario 3**: Economic loop (10 calls → receipt chain → settlement)
//! - **Scenario 4**: Fault recovery (unavailable agent → fallback routing)
//! - **Scenario 5**: Security verification (budget/rate-limit/audit)

#[path = "common/mod.rs"]
mod common;

use common::TestAgent;
use nexa_net::economy::settlement::SettlementStatus;
use nexa_net::economy::{MicroReceipt, ReceiptChain, ReceiptVerifier, SettlementEngine};
use nexa_net::identity::{KeyPair, VerifiableCredential};
use nexa_net::security::RateLimitKey;
use nexa_net::types::Did;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Scenario 1: HTTP Dual Agent Communication
// ============================================================================
//
// TestProxy starts → Agent B registers Translation → HTTP POST /v1/discover
// → HTTP GET /v1/health → verify routing → internal open_channel → receipt chain

#[tokio::test]
async fn test_http_dual_agent_communication() {
    // Start TestProxy on random port
    let proxy = common::TestProxy::new().start().await;

    // Step 1: Health check — verify server is reachable
    let health = proxy.health().await;
    assert_eq!(
        health.status, "healthy",
        "Server must be healthy after startup"
    );

    // Step 2: Register Agent B (provider) capability directly into ProxyState
    let mut provider = TestAgent::new("provider-b-http");
    provider.add_capability("Translation Service", vec!["translation", "nlp"]);
    for cap in &provider.capabilities {
        proxy.register_capability(cap.clone()).await;
    }

    // Step 3: HTTP POST /v1/discover — should find Agent B
    let discover_response = proxy.discover("translate text", 10).await;
    assert!(
        !discover_response.routes.is_empty(),
        "HTTP /v1/discover must find registered translation service"
    );

    // Verify the discovered route points to Agent B
    let route = &discover_response.routes[0];
    assert_eq!(
        route.provider_did,
        provider.did.as_str(),
        "Discovered route must point to Agent B"
    );
    assert_eq!(
        route.service_name, "default endpoint",
        "Service name must match registered endpoint"
    );

    // Step 4: Internal open channel (consumer A → provider B)
    let consumer = TestAgent::new("consumer-a-http");
    let channel = proxy
        .open_channel(consumer.did.clone(), provider.did.clone(), 1000, 10)
        .await;
    assert_eq!(
        channel.total_balance(),
        1010,
        "Channel total must be deposit_a + deposit_b"
    );

    // Step 5: Receipt chain generation (simulate payment)
    let mut receipt_chain = ReceiptChain::new(consumer.did.clone(), provider.did.clone());
    let mut receipt = receipt_chain.create_receipt("call-translate-http-1", 10, "/translate");
    receipt.sign_payer(consumer.signing_keypair()).unwrap();
    receipt.sign_payee(provider.signing_keypair()).unwrap();
    receipt_chain.add_receipt(receipt).unwrap();

    // Step 6: Update channel balances
    proxy.update_balances(&channel.id, 990, 20).await;
    let final_channel = proxy.get_channel(&channel.id).await;
    assert_eq!(final_channel.balance_a, 990, "Consumer pays 10");
    assert_eq!(final_channel.balance_b, 20, "Provider receives 10");
    assert_eq!(
        final_channel.total_balance(),
        1010,
        "Total balance must remain constant"
    );

    // Step 7: Verify receipt chain integrity
    assert!(
        receipt_chain.verify_chain_integrity().unwrap(),
        "Receipt chain must be intact"
    );
    assert!(
        receipt_chain.last().unwrap().is_confirmed(),
        "Receipt must be fully signed by both parties"
    );

    // Cleanup
    proxy.shutdown().await;
}

// ============================================================================
// Scenario 2: HTTP Multi-Agent Community
// ============================================================================
//
// TestProxy starts → 5 Agents register → 5 HTTP /v1/discover calls
// → verify routing correctness → open channels → verify budget control

#[tokio::test]
async fn test_http_multi_agent_community() {
    let proxy = common::TestProxy::new().start().await;

    // Step 1: Register 5 agents with distinct capabilities
    let mut agent_a = TestAgent::new("agent-a-http");
    agent_a.add_capability("Translation", vec!["translation", "nlp"]);

    let mut agent_b = TestAgent::new("agent-b-http");
    agent_b.add_capability("Image Processing", vec!["image", "vision"]);

    let mut agent_c = TestAgent::new("agent-c-http");
    agent_c.add_capability("Document Analysis", vec!["document", "nlp", "analysis"]);

    let mut agent_d = TestAgent::new("agent-d-http");
    agent_d.add_capability("Sentiment Analysis", vec!["sentiment", "nlp"]);

    let mut agent_e = TestAgent::new("agent-e-http");
    agent_e.add_capability("Summarization", vec!["summary", "nlp"]);

    // Register all capabilities via direct ProxyState access
    for agent in &[&agent_a, &agent_b, &agent_c, &agent_d, &agent_e] {
        for cap in &agent.capabilities {
            proxy.register_capability(cap.clone()).await;
        }
    }

    // Verify registry stats
    let stats = proxy.registry_stats().await;
    assert_eq!(
        stats.total_capabilities, 5,
        "All 5 agents must be registered"
    );

    // Step 2: HTTP /v1/discover for each intent
    let intents = vec![
        "translate text",
        "process image",
        "analyze document",
        "analyze sentiment",
        "summarize article",
    ];

    let mut discovered_providers = HashSet::new();
    for intent in &intents {
        let response = proxy.discover(intent, 10).await;
        assert!(
            !response.routes.is_empty(),
            "HTTP /v1/discover must find services for intent: {}",
            intent
        );
        for route in &response.routes {
            discovered_providers.insert(route.provider_did.clone());
        }
    }

    // Verify multiple different providers were discovered
    assert!(
        discovered_providers.len() >= 2,
        "Should discover at least 2 different providers across intents"
    );

    // Step 3: Budget control — check budget for cross-calls
    proxy
        .check_budget(&agent_a.did.to_string(), 50)
        .await
        .unwrap();
    proxy
        .check_budget(&agent_a.did.to_string(), 100)
        .await
        .unwrap();

    // Step 4: Open channels for cross-calls (A→B, B→C)
    let ch_ab = proxy
        .open_channel(agent_a.did.clone(), agent_b.did.clone(), 500, 10)
        .await;
    let ch_bc = proxy
        .open_channel(agent_b.did.clone(), agent_c.did.clone(), 500, 10)
        .await;

    // Verify channels via HTTP /v1/channels
    let channels = proxy.list_channels().await;
    assert_eq!(
        channels.len(),
        2,
        "HTTP /v1/channels must list 2 open channels"
    );

    // Verify channel IDs match
    let channel_ids: Vec<&str> = channels.iter().map(|c| c.channel_id.as_str()).collect();
    assert!(
        channel_ids.contains(&ch_ab.id.as_str()),
        "Channel A→B must be listed"
    );
    assert!(
        channel_ids.contains(&ch_bc.id.as_str()),
        "Channel B→C must be listed"
    );

    // Cleanup
    proxy.shutdown().await;
}

// ============================================================================
// Scenario 3: HTTP Economic Loop
// ============================================================================
//
// TestProxy starts → Agent registers → open_channel → 10 calls → 10 receipts
// → close_channel → settlement → verify balance invariant

#[tokio::test]
async fn test_http_economic_loop() {
    let proxy = common::TestProxy::new().start().await;

    // Step 1: Register provider capability
    let mut provider = TestAgent::new("payee-loop-http");
    provider.add_capability_with_cost("Data Processing", vec!["data", "processing"], 50);
    for cap in &provider.capabilities {
        proxy.register_capability(cap.clone()).await;
    }

    let consumer = TestAgent::new("payer-loop-http");

    // Step 2: Open payment channel (A deposits 10000, B deposits 10)
    let channel = proxy
        .open_channel(consumer.did.clone(), provider.did.clone(), 10000, 10)
        .await;
    let channel_id = channel.id.clone();
    let initial_total = channel.total_balance();
    assert_eq!(initial_total, 10010);

    // Step 3: 10 calls with receipt chain + budget tracking
    let mut receipt_chain = ReceiptChain::new(consumer.did.clone(), provider.did.clone());
    let cost_per_call = 50u64;
    let total_calls = 10;

    for i in 1..=total_calls {
        let call_id = format!("call-http-{}", i);

        // Budget check before each call
        proxy
            .check_budget(&consumer.did.to_string(), cost_per_call)
            .await
            .unwrap();

        // Create and sign receipt
        let mut receipt = receipt_chain.create_receipt(&call_id, cost_per_call, "/service");
        receipt.sign_payer(consumer.signing_keypair()).unwrap();
        receipt.sign_payee(provider.signing_keypair()).unwrap();
        receipt_chain.add_receipt(receipt).unwrap();

        // Update channel balances: A pays 50 per call
        let new_balance_a = 10000 - cost_per_call * i;
        let new_balance_b = 10 + cost_per_call * i;
        proxy
            .update_balances(&channel_id, new_balance_a, new_balance_b)
            .await;

        // Record spending
        proxy
            .record_spending(&consumer.did.to_string(), cost_per_call)
            .await;
    }

    // Step 4: Verify receipt chain integrity
    assert_eq!(
        receipt_chain.len(),
        total_calls as usize,
        "Should have {} receipts",
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

    // All receipts must be confirmed
    for receipt in receipt_chain.all_receipts() {
        assert!(receipt.is_confirmed(), "Each receipt must be fully signed");
    }

    // Step 5: Verify final channel balances
    let channel = proxy.get_channel(&channel_id).await;
    assert_eq!(
        channel.balance_a, 9500,
        "Consumer after 10 calls of 50 each"
    );
    assert_eq!(channel.balance_b, 510, "Provider after 10 calls of 50 each");
    assert_eq!(
        channel.total_balance(),
        initial_total,
        "Total balance must remain constant"
    );

    // Step 6: Verify budget tracking
    let budget_status = proxy.budget_status(&consumer.did.to_string()).await;
    assert_eq!(
        budget_status.spent_total, 500,
        "Total spending should be 500"
    );

    // Step 7: Close channel (bypass challenge period)
    let closed_channel = proxy.close_channel_bypass(&channel_id).await;
    assert!(
        closed_channel.is_closed(),
        "Channel must be closed after settlement"
    );

    // Step 8: Create and finalize settlement
    let mut settlement_engine = SettlementEngine::new();
    let settlement = settlement_engine
        .create_settlement(&closed_channel)
        .unwrap();
    assert_eq!(settlement.balance_a, 9500);
    assert_eq!(settlement.balance_b, 510);
    assert_eq!(settlement.status, SettlementStatus::Pending);

    let finalized = settlement_engine.finalize(&settlement.id).unwrap();
    assert_eq!(
        finalized.status,
        SettlementStatus::Finalized,
        "Settlement must be finalized"
    );

    // Cleanup
    proxy.shutdown().await;
}

// ============================================================================
// Scenario 4: HTTP Fault Recovery
// ============================================================================
//
// TestProxy starts → Agent B registers → set_availability(false) →
// Agent C registers as fallback → HTTP /v1/discover → verify returns C

#[tokio::test]
async fn test_http_fault_recovery() {
    let proxy = common::TestProxy::new().start().await;

    // Step 1: Register Agent B (translation, will go offline)
    let mut agent_b = TestAgent::new("provider-b-unavailable-http");
    agent_b.add_capability("Translation Service", vec!["translation", "nlp"]);
    for cap in &agent_b.capabilities {
        proxy.register_capability(cap.clone()).await;
    }

    // Step 2: Register Agent C (fallback translation)
    let mut agent_c = TestAgent::new("provider-c-fallback-http");
    agent_c.add_capability("Translation Backup", vec!["translation", "nlp", "backup"]);
    for cap in &agent_c.capabilities {
        proxy.register_capability(cap.clone()).await;
    }

    // Verify both are registered
    let stats = proxy.registry_stats().await;
    assert_eq!(stats.total_capabilities, 2);

    // Step 3: Mark Agent B as unavailable (simulating node failure)
    proxy.set_availability(agent_b.did.as_str(), false).await;

    // Step 4: HTTP POST /v1/discover with available_only=true
    // The router in ProxyState uses default config (available_only=true)
    // But our test router config has available_only=false...
    // We need to update the router config. Let's use the internal discover
    // method that respects available_only, and then verify via HTTP.
    //
    // Since the router in ProxyState has default config (min_similarity=0.5, min_quality=0.8),
    // we need to configure it for testing. Let's modify the router config via ProxyState.

    // Reconfigure router for available_only=true (filter out unavailable Agent B)
    proxy.configure_router_for_testing(true).await;

    // Step 4: HTTP POST /v1/discover — should find only Agent C
    let discover_response = proxy.discover("translate text", 10).await;
    assert!(
        !discover_response.routes.is_empty(),
        "HTTP /v1/discover must find fallback service"
    );

    // All discovered routes must point to Agent C, not B
    for route in &discover_response.routes {
        assert_ne!(
            route.provider_did,
            agent_b.did.as_str(),
            "Should not route to unavailable Agent B"
        );
        assert_eq!(
            route.provider_did,
            agent_c.did.as_str(),
            "Should route to available fallback Agent C"
        );
    }

    // Step 5: Open channel with Agent C and verify
    let consumer = TestAgent::new("consumer-fault-http");
    let channel = proxy
        .open_channel(consumer.did.clone(), agent_c.did.clone(), 1000, 10)
        .await;

    // Generate receipt for fallback call
    let mut receipt_chain = ReceiptChain::new(consumer.did.clone(), agent_c.did.clone());
    let mut receipt = receipt_chain.create_receipt("call-fallback-http-1", 10, "/translate");
    receipt.sign_payer(consumer.signing_keypair()).unwrap();
    receipt.sign_payee(agent_c.signing_keypair()).unwrap();
    receipt_chain.add_receipt(receipt).unwrap();

    // Update balances
    proxy.update_balances(&channel.id, 990, 20).await;

    // Verify fault recovery succeeded
    assert!(receipt_chain.verify_chain_integrity().unwrap());
    let final_channel = proxy.get_channel(&channel.id).await;
    assert_eq!(final_channel.balance_a, 990);
    assert_eq!(final_channel.balance_b, 20);

    // Cleanup
    proxy.shutdown().await;
}

// ============================================================================
// Scenario 5: HTTP Security Verification
// ============================================================================
//
// TestProxy starts → unsigned receipt rejected → forged VC rejected →
// budget exceeded rejected → rate limit → verify audit events

#[tokio::test]
async fn test_http_security_verification() {
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
    let agent = TestAgent::new("agent-security-http");
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
    let other_agent = TestAgent::new("agent-other-http");
    let result = vc.verify_with_keypair(other_agent.signing_keypair());
    assert!(
        result.is_err(),
        "VC should fail verification with wrong key (forged verification)"
    );

    // --- Sub-test 3: Budget exceeded → rejection ---
    // Use ProxyState with restricted budget
    let proxy = common::TestProxy::new().start().await;
    let state = proxy.state();

    // Configure restricted budget on ProxyState
    {
        let mut budget = state.budget.write().await;
        *budget =
            nexa_net::economy::BudgetController::with_limits(nexa_net::economy::BudgetLimit {
                max_per_call: 50,
                max_per_minute: 100,
                max_per_hour: 500,
                max_per_day: 1000,
                max_total: 500,
            });
    }

    let did_str = "did:nexa:over-budget-agent-http".to_string();

    // Within per_call limit (max_per_call=50) → should pass
    proxy.check_budget(&did_str, 30).await.unwrap();

    // Exceed per_call limit (amount 100 > max_per_call 50) → should reject
    let result = proxy.check_budget(&did_str, 100).await;
    assert!(
        result.is_err(),
        "Should reject amount exceeding per_call budget limit"
    );

    // --- Sub-test 4: Rate limit exceeded → blocked ---
    let (manager, sink) = common::create_security_manager_with_rate_limit();
    let key = RateLimitKey::Did("did:nexa:rate-limited-agent-http".to_string());

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

    // Cleanup
    proxy.shutdown().await;
}
