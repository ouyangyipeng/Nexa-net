//! Multi-Agent Performance Simulation
//!
//! 启动真实 REST 服务器，通过 HTTP API 测量端到端性能，
//! 同时直接通过内部组件测量纯逻辑性能。
//!
//! 测量关键指标：
//! - 纯逻辑发现延迟
//! - HTTP API 发现延迟
//! - 通道打开/更新/关闭延迟
//! - 收据签名/验签延迟
//! - AES加密/解密延迟
//! - 速率限制检查延迟
//! - 并发吞吐量

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key,
};
use nexa_net::{
    api::rest::{
        ApiDiscoverRequest, ApiDiscoverResponse, ApiRegisterRequest, ApiRegisterResponse,
        RestServer,
    },
    economy::{ChannelConfig, ChannelManager, MicroReceipt, ReceiptChain, ReceiptVerifier},
    identity::{Did as IdentityDid, KeyPair},
    proxy::server::ProxyState,
    security::{RateLimitConfig, RateLimitKey, RateLimiter},
    types::{CapabilitySchema, Did as TypesDid, EndpointDefinition, RouteContext, ServiceMetadata},
};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Number of simulated agents
const NUM_AGENTS: usize = 5;

/// Number of iterations per measurement
const NUM_ITERS: usize = 100;

/// Capability definitions for the simulation
const CAPABILITY_DEFS: &[(&str, &str)] = &[
    ("translation", "translate text between languages"),
    ("image-processing", "process and analyze images"),
    (
        "document-analysis",
        "analyze and extract information from documents",
    ),
    ("code-generation", "generate code from specifications"),
    (
        "sentiment-analysis",
        "analyze sentiment and emotion in text",
    ),
];

/// Simulated agent with identity and signing key
struct SimAgent {
    did: TypesDid,
    keypair: KeyPair,
}

impl SimAgent {
    fn new() -> Self {
        let keypair = KeyPair::generate().expect("keypair generation failed");
        let identity_did = IdentityDid::from_public_key(keypair.public_key().inner());
        let types_did: TypesDid = identity_did.into();
        Self {
            did: types_did,
            keypair,
        }
    }
}

/// Format duration as human-readable string
fn fmt_duration(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns < 1_000 {
        format!("{} ns", ns)
    } else if ns < 1_000_000 {
        format!("{:.1} µs", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.2} ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.2} s", ns as f64 / 1_000_000_000.0)
    }
}

/// Format throughput as human-readable ops/s
fn fmt_throughput(op_count: u64, d: Duration) -> String {
    let seconds = d.as_secs_f64();
    if seconds > 0.0 {
        let ops = op_count as f64 / seconds;
        if ops >= 1_000_000.0 {
            format!("{:.0}M", ops / 1_000_000.0)
        } else if ops >= 1_000.0 {
            format!("{:.0}K", ops / 1_000.0)
        } else {
            format!("{}", ops as u64)
        }
    } else {
        "N/A".to_string()
    }
}

#[tokio::main]
async fn main() {
    println!("=== Nexa-net Multi-Agent Performance Simulation ===");
    println!();

    // =============================================================================
    // 1. Initialize environment — start REST server on random port
    // =============================================================================

    let proxy_state = Arc::new(ProxyState::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    println!("REST server listening on 127.0.0.1:{}", port);

    let router = RestServer::build_router(proxy_state.clone());
    let _server_handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Create agents
    let agents: Vec<SimAgent> = (0..NUM_AGENTS).map(|_| SimAgent::new()).collect();
    println!("Agents: {}", NUM_AGENTS);
    println!(
        "Capabilities: {} ({})",
        CAPABILITY_DEFS.len(),
        CAPABILITY_DEFS
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    // =============================================================================
    // 2. Register all agent capabilities via direct registry access
    // =============================================================================

    {
        let mut registry = proxy_state.registry.write().await;
        for (i, (name, desc)) in CAPABILITY_DEFS.iter().enumerate() {
            let agent = &agents[i];
            let schema = CapabilitySchema {
                version: "1.0.0".to_string(),
                metadata: ServiceMetadata {
                    did: agent.did.clone(),
                    name: name.to_string(),
                    description: desc.to_string(),
                    tags: vec![name.to_string()],
                },
                endpoints: vec![EndpointDefinition {
                    id: "main".to_string(),
                    name: name.to_string(),
                    description: String::new(),
                    input_schema: serde_json::Value::Object(serde_json::Map::new()),
                    output_schema: serde_json::Value::Object(serde_json::Map::new()),
                    base_cost: 10,
                    rate_limit: 100,
                }],
            };
            registry.register(schema).unwrap();
        }
    }
    println!("All capabilities registered successfully");
    println!();

    // =============================================================================
    // 3. Register capabilities via HTTP API
    // =============================================================================

    let http_client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);

    for (name, desc) in CAPABILITY_DEFS.iter() {
        let req = ApiRegisterRequest {
            name: name.to_string(),
            description: desc.to_string(),
            tags: vec![name.to_string()],
            endpoint: format!("https://api.example.com/{}", name),
            cost_per_call: 10,
        };
        let resp: ApiRegisterResponse = http_client
            .post(format!("{}{}", base_url, "/v1/register"))
            .json(&req)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        println!("HTTP register: {} → did={}", name, resp.did);
    }
    println!();

    // =============================================================================
    // 4. Discovery Performance — pure logic (internal API)
    // =============================================================================

    let route_ctx = RouteContext {
        max_candidates: 10,
        similarity_threshold: 0.0,
        preferred_providers: Vec::new(),
        excluded_providers: Vec::new(),
        max_latency_ms: None,
        max_cost: None,
    };

    let semantic_router = proxy_state.router.read().await;
    let mut pure_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let start = Instant::now();
        let _routes = semantic_router
            .discover("translation", route_ctx.clone())
            .await
            .unwrap();
        pure_durations.push(start.elapsed());
    }
    let avg_pure = pure_durations.iter().sum::<Duration>() / (pure_durations.len() as u32);

    println!("--- Discovery Performance ---");
    println!(
        "Pure logic discovery latency: {} (avg over {} runs)",
        fmt_duration(avg_pure),
        NUM_ITERS
    );

    // =============================================================================
    // 5. Discovery Performance — HTTP API
    // =============================================================================

    let mut http_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let req = ApiDiscoverRequest {
            intent: "translation".to_string(),
            max_results: Some(10),
            threshold: Some(0.0),
        };
        let start = Instant::now();
        let _resp: ApiDiscoverResponse = http_client
            .post(format!("{}{}", base_url, "/v1/discover"))
            .json(&req)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        http_durations.push(start.elapsed());
    }
    let avg_http = http_durations.iter().sum::<Duration>() / (http_durations.len() as u32);
    println!(
        "HTTP API discovery latency: {} (avg over {} runs)",
        fmt_duration(avg_http),
        NUM_ITERS
    );

    // =============================================================================
    // 6. Concurrent discovery — 5 agents simultaneously
    // =============================================================================

    let start = Instant::now();
    let mut tasks = Vec::with_capacity(NUM_AGENTS);
    for (name, _) in CAPABILITY_DEFS.iter().take(NUM_AGENTS) {
        let req = ApiDiscoverRequest {
            intent: name.to_string(),
            max_results: Some(10),
            threshold: Some(0.0),
        };
        tasks.push(
            http_client
                .post(format!("{}{}", base_url, "/v1/discover"))
                .json(&req)
                .send(),
        );
    }
    let _results = futures::future::join_all(tasks).await;
    let total_concurrent = start.elapsed();
    let per_agent_concurrent = total_concurrent / NUM_AGENTS as u32;
    println!(
        "Concurrent discovery ({} agents): {} total, {} per agent",
        NUM_AGENTS,
        fmt_duration(total_concurrent),
        fmt_duration(per_agent_concurrent),
    );

    println!("--- Channel & Receipt Performance ---");

    // =============================================================================
    // 7. Channel Performance — direct ChannelManager operations
    // =============================================================================

    // Create a standalone ChannelManager for benchmarking (not through ProxyState,
    // to avoid RwLock overhead in micro-benchmarks)
    let mut channel_mgr = ChannelManager::with_config(ChannelConfig {
        min_deposit: 1,
        max_deposit: 1_000_000,
        challenge_period: Duration::from_secs(1),
        max_channels_per_peer: NUM_ITERS + 10,
    });

    // Measure channel open latency
    let mut open_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let start = Instant::now();
        let _ch = channel_mgr
            .open(agents[0].did.clone(), agents[1].did.clone(), 1000, 500)
            .unwrap();
        open_durations.push(start.elapsed());
    }
    let avg_open = open_durations.iter().sum::<Duration>() / (open_durations.len() as u32);
    println!("Channel open: {}", fmt_duration(avg_open));

    // Open a specific channel for update/close tests
    let ch = channel_mgr
        .open(agents[0].did.clone(), agents[1].did.clone(), 1000, 500)
        .unwrap();
    let ch_id = ch.id.clone();

    // Measure channel update latency
    let mut update_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let start = Instant::now();
        channel_mgr.update_balances(&ch_id, 900, 600).unwrap();
        update_durations.push(start.elapsed());
    }
    let avg_update = update_durations.iter().sum::<Duration>() / (update_durations.len() as u32);
    println!("Channel update: {}", fmt_duration(avg_update));

    // Measure channel close latency (force close, bypassing challenge period)
    let mut close_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let start = Instant::now();
        let channel = channel_mgr.get_mut(&ch_id).unwrap();
        channel.state = nexa_net::types::ChannelState::Closed;
        channel.updated_at = chrono::Utc::now();
        close_durations.push(start.elapsed());
    }
    let avg_close = close_durations.iter().sum::<Duration>() / (close_durations.len() as u32);
    println!("Channel close: {}", fmt_duration(avg_close));

    // =============================================================================
    // 8. Receipt Performance
    // =============================================================================

    // Measure receipt sign (payer) latency
    let mut sign_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let start = Instant::now();
        let mut receipt =
            MicroReceipt::new_genesis("call-1", &agents[0].did, &agents[1].did, 25, "main");
        receipt.sign_payer(&agents[0].keypair).unwrap();
        sign_durations.push(start.elapsed());
    }
    let avg_sign = sign_durations.iter().sum::<Duration>() / (sign_durations.len() as u32);
    println!("Receipt sign: {}", fmt_duration(avg_sign));

    // Measure receipt verify latency (sign + verify payer signature)
    let payer_pubkey = agents[0].keypair.public_key().inner();
    let mut verify_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let start = Instant::now();
        let mut receipt =
            MicroReceipt::new_genesis("call-1", &agents[0].did, &agents[1].did, 25, "main");
        receipt.sign_payer(&agents[0].keypair).unwrap();
        receipt.sign_payee(&agents[1].keypair).unwrap();
        let _valid = ReceiptVerifier::verify_payer_signature(&receipt, payer_pubkey).unwrap();
        verify_durations.push(start.elapsed());
    }
    let avg_verify = verify_durations.iter().sum::<Duration>() / (verify_durations.len() as u32);
    println!("Receipt verify: {}", fmt_duration(avg_verify));

    // Measure receipt chain (10 receipts) latency
    let mut chain_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let start = Instant::now();
        let mut chain = ReceiptChain::new(agents[0].did.clone(), agents[1].did.clone());
        for j in 0..10 {
            let mut receipt = chain.create_receipt(&format!("call-{}", j), 10 + j as u64, "main");
            receipt.sign_payer(&agents[0].keypair).unwrap();
            receipt.sign_payee(&agents[1].keypair).unwrap();
            chain.add_receipt(receipt).unwrap();
        }
        chain_durations.push(start.elapsed());
    }
    let avg_chain = chain_durations.iter().sum::<Duration>() / (chain_durations.len() as u32);
    println!("Receipt chain (10 receipts): {}", fmt_duration(avg_chain));

    println!("--- Security Performance ---");

    // =============================================================================
    // 9. AES-256-GCM Performance — direct cryptographic operations
    // =============================================================================

    let aes_key = Key::<Aes256Gcm>::from([42u8; 32]);
    let cipher = Aes256Gcm::new(&aes_key);

    // Encrypt 32 bytes
    let plaintext_32 = vec![0u8; 32];
    let mut encrypt_32_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let start = Instant::now();
        let _ciphertext = cipher.encrypt(&nonce, plaintext_32.as_slice()).unwrap();
        encrypt_32_durations.push(start.elapsed());
    }
    let avg_encrypt_32 =
        encrypt_32_durations.iter().sum::<Duration>() / (encrypt_32_durations.len() as u32);
    println!(
        "AES-256-GCM encrypt (32 bytes): {}",
        fmt_duration(avg_encrypt_32)
    );

    // Decrypt 32 bytes — encrypt with a nonce, then decrypt with the SAME nonce
    let mut decrypt_32_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext_32 = cipher.encrypt(&nonce, plaintext_32.as_slice()).unwrap();
        let start = Instant::now();
        let _decrypted = cipher.decrypt(&nonce, ciphertext_32.as_slice()).unwrap();
        decrypt_32_durations.push(start.elapsed());
    }
    let avg_decrypt_32 =
        decrypt_32_durations.iter().sum::<Duration>() / (decrypt_32_durations.len() as u32);
    println!(
        "AES-256-GCM decrypt (32 bytes): {}",
        fmt_duration(avg_decrypt_32)
    );

    // Encrypt 1KB
    let plaintext_1k = vec![0u8; 1024];
    let mut encrypt_1k_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let start = Instant::now();
        let _ciphertext = cipher.encrypt(&nonce, plaintext_1k.as_slice()).unwrap();
        encrypt_1k_durations.push(start.elapsed());
    }
    let avg_encrypt_1k =
        encrypt_1k_durations.iter().sum::<Duration>() / (encrypt_1k_durations.len() as u32);
    println!(
        "AES-256-GCM encrypt (1KB): {}",
        fmt_duration(avg_encrypt_1k)
    );

    // =============================================================================
    // 10. Rate Limit Performance
    // =============================================================================

    let rate_config = RateLimitConfig {
        requests_per_minute: 10000,
        requests_per_hour: 100000,
        requests_per_day: 1_000_000,
        burst_size: 100,
        enabled: true,
    };
    let rate_limiter = RateLimiter::new(rate_config);

    let mut rate_check_durations: Vec<Duration> = Vec::with_capacity(NUM_ITERS);
    for _ in 0..NUM_ITERS {
        let key = RateLimitKey::Did(agents[0].did.as_str().to_string());
        let start = Instant::now();
        let _result = rate_limiter.check(&key).unwrap();
        rate_check_durations.push(start.elapsed());
    }
    let avg_rate =
        rate_check_durations.iter().sum::<Duration>() / (rate_check_durations.len() as u32);
    println!("Rate limit check: {}", fmt_duration(avg_rate));

    println!("--- Throughput ---");

    // =============================================================================
    // 11. Discovery throughput (1s sustained run)
    // =============================================================================

    let start = Instant::now();
    let mut count: u64 = 0;
    loop {
        let _ = semantic_router
            .discover("translation", route_ctx.clone())
            .await
            .unwrap();
        count += 1;
        if start.elapsed() >= Duration::from_secs(1) {
            break;
        }
    }
    println!(
        "Discovery throughput: {} ops/s",
        fmt_throughput(count, Duration::from_secs(1))
    );

    // =============================================================================
    // 12. Channel update throughput (1s sustained run)
    // =============================================================================

    // Re-open a fresh channel for throughput test (previous was closed in close benchmark)
    let throughput_ch = channel_mgr
        .open(agents[0].did.clone(), agents[1].did.clone(), 1000, 500)
        .unwrap();
    let throughput_ch_id = throughput_ch.id.clone();

    let start = Instant::now();
    let mut count: u64 = 0;
    loop {
        channel_mgr
            .update_balances(&throughput_ch_id, 900, 600)
            .unwrap();
        count += 1;
        if start.elapsed() >= Duration::from_secs(1) {
            break;
        }
    }
    println!(
        "Channel update TPS: {}",
        fmt_throughput(count, Duration::from_secs(1))
    );

    // =============================================================================
    // 13. Receipt sign throughput (1s sustained run)
    // =============================================================================

    let start = Instant::now();
    let mut count: u64 = 0;
    loop {
        let mut receipt = MicroReceipt::new_genesis(
            &format!("call-{}", count),
            &agents[0].did,
            &agents[1].did,
            25,
            "main",
        );
        receipt.sign_payer(&agents[0].keypair).unwrap();
        count += 1;
        if start.elapsed() >= Duration::from_secs(1) {
            break;
        }
    }
    println!(
        "Receipt sign TPS: {}",
        fmt_throughput(count, Duration::from_secs(1))
    );
}
