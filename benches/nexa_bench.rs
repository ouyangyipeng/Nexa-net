//! Performance Benchmarks for Nexa-net
//!
//! Comprehensive Criterion benchmarks covering all 6 core modules:
//! - Identity: DID parsing, key generation, signing/verification, AES-GCM
//! - Discovery: HNSW insert/search, Kademlia routing, semantic search
//! - Transport: Serialization, compression, frame encode/decode
//! - Economy: Channel operations, receipt sign/verify, budget reserve/settle
//! - Security: AES-GCM encrypt/decrypt, rate limit, audit throughput
//! - Storage: MemoryStore CRUD + cache
//!
//! Run with: cargo bench

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nexa_net::discovery::router::{RoutingConfig, RoutingWeights};
use nexa_net::{
    api::rest::{
        ApiDiscoverRequest, ApiDiscoverResponse, ApiHealthResponse, ApiRegisterRequest,
        ApiRegisterResponse, RestServer,
    },
    discovery::{
        CapabilityRegistry, DhtNodeInfo, HnswIndex, KademliaRoutingTable, SemanticDHT,
        SemanticRouter, Vectorizer,
    },
    economy::{
        BudgetController, Channel, ChannelManager, MicroReceipt, ReceiptChain, ReceiptVerifier,
    },
    identity::{IdentityKeys, KeyPair},
    proxy::server::ProxyState,
    security::{
        AuditLogger, MemoryAuditSink, RateLimitConfig, RateLimitKey, RateLimiter, SecureKeyStorage,
    },
    transport::{
        compress, decompress, CompressionAlgorithm, Frame, FrameFlags, FrameHeader,
        SerializationEngine, SerializationFormat,
    },
    types::{CapabilitySchema, ServiceMetadata},
    Did,
};
use rand::Rng;
use reqwest;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a random vector of given dimensions
fn random_vector(dimensions: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    let mut v: Vec<f32> = (0..dimensions).map(|_| rng.gen_range(-1.0..1.0)).collect();
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

/// Generate random bytes of given size
fn random_bytes(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..size).map(|_| rng.gen::<u8>()).collect()
}

/// Create a test DID (returns types::Did for economy/storage/capability APIs)
fn bench_did(s: &str) -> Did {
    Did::new(s)
}

/// Create a test identity::Did (for discovery/identity APIs)
fn bench_identity_did(s: &str) -> nexa_net::identity::Did {
    nexa_net::identity::Did::parse(s).unwrap()
}

/// Create a test capability schema
fn make_capability_schema(did_str: &str, name: &str, tags: Vec<String>) -> CapabilitySchema {
    CapabilitySchema {
        version: "1.0".to_string(),
        metadata: ServiceMetadata {
            did: bench_did(did_str),
            name: name.to_string(),
            description: format!("{} service", name),
            tags,
        },
        endpoints: vec![],
    }
}

// ============================================================================
// Identity Benchmarks
// ============================================================================

fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("identity/keypair_generation", |b| {
        b.iter(|| KeyPair::generate().unwrap())
    });
}

fn bench_did_creation(c: &mut Criterion) {
    c.bench_function("identity/did_parse", |b| {
        b.iter(|| nexa_net::identity::Did::parse("did:nexa:test123").unwrap())
    });
}

fn bench_signing(c: &mut Criterion) {
    let keypair = KeyPair::generate().unwrap();
    let message = b"test message for signing benchmark";

    c.bench_function("identity/sign_message", |b| {
        b.iter(|| keypair.sign(message).unwrap())
    });
}

fn bench_verification(c: &mut Criterion) {
    let keypair = KeyPair::generate().unwrap();
    let message = b"test message for signing benchmark";
    let signature = keypair.sign(message).unwrap();

    c.bench_function("identity/verify_signature", |b| {
        b.iter(|| keypair.verify(message, &signature).unwrap())
    });
}

fn bench_identity_keys_generation(c: &mut Criterion) {
    c.bench_function("identity/identity_keys_generate", |b| {
        b.iter(|| IdentityKeys::generate().unwrap())
    });
}

fn bench_aes_gcm_encrypt_decrypt(c: &mut Criterion) {
    let encryption_key = [42u8; 32];
    let storage = Arc::new(SecureKeyStorage::new(Some(encryption_key)));

    let mut group = c.benchmark_group("identity/aes_gcm");
    for size in [16usize, 256, 1024, 4096, 16384] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            let key_data = random_bytes(s);
            b.iter(|| {
                storage
                    .store_key("bench-key", "signing", &key_data, None)
                    .unwrap();
                storage.get_key("bench-key").unwrap().unwrap();
                storage.delete_key("bench-key").unwrap();
            });
        });
    }
    group.finish();
}

fn bench_secure_key_store_operations(c: &mut Criterion) {
    let storage = Arc::new(SecureKeyStorage::insecure());

    c.bench_function("identity/keystore_store", |b| {
        b.iter(|| {
            storage
                .store_key("bench-key", "signing", b"key-data-123", None)
                .unwrap();
        })
    });

    c.bench_function("identity/keystore_store+get", |b| {
        b.iter(|| {
            storage
                .store_key("bench-key", "signing", b"key-data-123", None)
                .unwrap();
            storage.get_key("bench-key").unwrap();
        })
    });
}

// ============================================================================
// Discovery Benchmarks
// ============================================================================

fn bench_vectorization(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let text = "translate English text to Chinese for document processing";

    c.bench_function("discovery/vectorize_text", |b| {
        b.iter(|| vectorizer.vectorize(text).unwrap())
    });
}

fn bench_batch_vectorization(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let texts: Vec<&str> = vec![
        "translate English to Chinese",
        "summarize this document",
        "generate code for sorting",
        "analyze sentiment of text",
        "extract entities from document",
    ];

    c.bench_function("discovery/vectorize_batch_5", |b| {
        b.iter(|| vectorizer.vectorize_batch(&texts).unwrap())
    });
}

fn bench_semantic_similarity(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let vec1 = vectorizer
        .vectorize("translate English to Chinese")
        .unwrap();
    let vec2 = vectorizer
        .vectorize("translation from English to Chinese")
        .unwrap();

    c.bench_function("discovery/cosine_similarity", |b| {
        b.iter(|| vec1.cosine_similarity(&vec2))
    });
}

fn bench_hnsw_insert(c: &mut Criterion) {
    let dimensions = 384;
    let mut group = c.benchmark_group("discovery/hnsw_insert");

    for size in [100usize, 1000, 10000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            let vectors: Vec<(String, Vec<f32>)> = (0..s)
                .map(|i| (format!("vec_{}", i), random_vector(dimensions)))
                .collect();

            b.iter(|| {
                let mut index = HnswIndex::new();
                for (key, vector) in &vectors {
                    index.insert(key.clone(), vector.clone()).unwrap();
                }
                index
            });
        });
    }
    group.finish();
}

fn bench_hnsw_search(c: &mut Criterion) {
    let dimensions = 384;
    let mut group = c.benchmark_group("discovery/hnsw_search");

    for size in [100usize, 1000, 10000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            let mut index = HnswIndex::new();
            for i in 0..s {
                index
                    .insert(format!("vec_{}", i), random_vector(dimensions))
                    .unwrap();
            }
            let query = random_vector(dimensions);

            b.iter(|| index.search(&query, 10));
        });
    }
    group.finish();
}

fn bench_kademlia_routing_table(c: &mut Criterion) {
    let local_id = [0u8; 32];

    c.bench_function("discovery/kademlia_add_node", |b| {
        b.iter(|| {
            let mut routing = KademliaRoutingTable::new(local_id);
            for i in 1..=20 {
                let mut node_id = [0u8; 32];
                node_id[31] = i as u8;
                let node = DhtNodeInfo {
                    id: node_id,
                    did: format!("did:nexa:node{}", i),
                    address: format!("127.0.0.1:{}", 8000 + i),
                    last_seen: 1000 + i as u64,
                };
                routing.add_or_update(node);
            }
        });
    });

    // Build routing table once for lookup benchmark
    let mut routing = KademliaRoutingTable::new(local_id);
    for i in 1..=200 {
        let mut node_id = [0u8; 32];
        node_id[30] = (i / 256) as u8;
        node_id[31] = (i % 256) as u8;
        let node = DhtNodeInfo {
            id: node_id,
            did: format!("did:nexa:node{}", i),
            address: format!("127.0.0.1:{}", 8000 + i),
            last_seen: 1000 + i as u64,
        };
        routing.add_or_update(node);
    }

    let mut target_id = [0u8; 32];
    target_id[31] = 5;

    c.bench_function("discovery/kademlia_find_closest", |b| {
        b.iter(|| routing.find_closest(&target_id, 20))
    });
}

fn bench_semantic_dht_end_to_end(c: &mut Criterion) {
    // SemanticDHT::new takes identity::Did
    let did = bench_identity_did("did:nexa:benchnode");
    let dht = Arc::new(SemanticDHT::new(&did));
    let dimensions = 384;

    // Insert vectors
    for i in 0..100 {
        let vector = random_vector(dimensions);
        dht.store(format!("key_{}", i), vector).unwrap();
    }

    let query = random_vector(dimensions);

    c.bench_function("discovery/semantic_dht_search", |b| {
        b.iter(|| dht.find_similar(&query, 10, 0.5))
    });
}

fn bench_capability_registration(c: &mut Criterion) {
    let mut registry = CapabilityRegistry::new();

    c.bench_function("discovery/register_capability", |b| {
        b.iter_batched(
            || {
                let did = bench_did(&format!("did:nexa:bench{}", uuid::Uuid::new_v4()));
                CapabilitySchema {
                    version: "1.0".to_string(),
                    metadata: ServiceMetadata {
                        did,
                        name: "bench-service".to_string(),
                        description: "Benchmark service".to_string(),
                        tags: vec!["bench".to_string()],
                    },
                    endpoints: vec![],
                }
            },
            |schema| registry.register(schema).unwrap(),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_cosine_distance_simd_friendly(c: &mut Criterion) {
    let dimensions = 384;
    let vec_a = random_vector(dimensions);
    let vec_b = random_vector(dimensions);

    c.bench_function("discovery/cosine_distance_384d", |b| {
        b.iter(|| HnswIndex::cosine_distance(&vec_a, &vec_b))
    });

    let vec_a_large = random_vector(768);
    let vec_b_large = random_vector(768);

    c.bench_function("discovery/cosine_distance_768d", |b| {
        b.iter(|| HnswIndex::cosine_distance(&vec_a_large, &vec_b_large))
    });
}

// ============================================================================
// Transport Benchmarks
// ============================================================================

fn bench_json_serialization(c: &mut Criterion) {
    let engine = SerializationEngine::new(SerializationFormat::Json);
    let data = serde_json::json!({
        "intent": "translate English to Chinese",
        "data": [1, 2, 3, 4, 5],
        "budget": 100,
        "timeout_ms": 30000
    });

    c.bench_function("transport/json_serialize", |b| {
        b.iter(|| engine.serialize(&data).unwrap())
    });
}

fn bench_json_deserialization(c: &mut Criterion) {
    let engine = SerializationEngine::new(SerializationFormat::Json);
    let data = serde_json::json!({
        "intent": "translate English to Chinese",
        "data": [1, 2, 3, 4, 5],
        "budget": 100,
        "timeout_ms": 30000
    });
    let serialized = engine.serialize(&data).unwrap();

    c.bench_function("transport/json_deserialize", |b| {
        b.iter(|| {
            engine
                .deserialize::<serde_json::Value>(&serialized)
                .unwrap()
        })
    });
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/compress");

    for &size in &[100usize, 1000, 10000, 100000] {
        let data = random_bytes(size);

        for &algo in &[
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
        ] {
            let algo_name = match algo {
                CompressionAlgorithm::Lz4 => "lz4",
                CompressionAlgorithm::Zstd => "zstd",
                CompressionAlgorithm::Gzip => "gzip",
                _ => "none",
            };

            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(BenchmarkId::new(algo_name, size), &data, |b, _| {
                b.iter(|| compress(&data, algo).unwrap())
            });
        }
    }
    group.finish();
}

fn bench_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/decompress");

    for &size in &[100usize, 1000, 10000, 100000] {
        let data = random_bytes(size);

        for &algo in &[
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
        ] {
            let algo_name = match algo {
                CompressionAlgorithm::Lz4 => "lz4",
                CompressionAlgorithm::Zstd => "zstd",
                CompressionAlgorithm::Gzip => "gzip",
                _ => "none",
            };
            let compressed = compress(&data, algo).unwrap();

            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(BenchmarkId::new(algo_name, size), &compressed, |b, _| {
                b.iter(|| decompress(&compressed, algo).unwrap())
            });
        }
    }
    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/compression_ratio");

    for &size in &[1000usize, 10000, 100000] {
        // Create repetitive data (simulates JSON/text payloads)
        let pattern = b"{\"key\": \"value\", \"data\": [1,2,3,4,5], \"nested\": {\"a\": true}}";
        let mut data = Vec::with_capacity(size);
        while data.len() < size {
            let remaining = size - data.len();
            let chunk_size = remaining.min(pattern.len());
            data.extend_from_slice(&pattern[..chunk_size]);
        }

        for &algo in &[
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Gzip,
        ] {
            let algo_name = match algo {
                CompressionAlgorithm::Lz4 => "lz4",
                CompressionAlgorithm::Zstd => "zstd",
                CompressionAlgorithm::Gzip => "gzip",
                _ => "none",
            };

            group.bench_with_input(BenchmarkId::new(algo_name, size), &data, |b, _| {
                b.iter(|| {
                    let compressed = compress(&data, algo).unwrap();
                    compressed.len() as f64 / data.len() as f64
                })
            });
        }
    }
    group.finish();
}

fn bench_frame_encode_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/frame");

    for &payload_size in &[0usize, 100, 1000, 10000, 65535] {
        group.throughput(Throughput::Bytes(payload_size as u64 + 12));
        group.bench_with_input(
            BenchmarkId::from_parameter(payload_size),
            &payload_size,
            |b, &ps| {
                let payload = random_bytes(ps);
                let frame = Frame::data(1, payload, false);

                b.iter(|| {
                    let encoded = frame.encode();
                    Frame::decode(&encoded).unwrap()
                });
            },
        );
    }
    group.finish();
}

fn bench_frame_header_encode_decode(c: &mut Criterion) {
    let header = FrameHeader::data(1, 100, FrameFlags::new(FrameFlags::COMPRESSED));

    c.bench_function("transport/frame_header_encode", |b| {
        b.iter(|| header.encode())
    });

    let encoded = header.encode();
    c.bench_function("transport/frame_header_decode", |b| {
        b.iter(|| FrameHeader::decode(&encoded).unwrap())
    });
}

fn bench_serialization_compression_pipeline(c: &mut Criterion) {
    let engine = SerializationEngine::with_compression(
        SerializationFormat::Json,
        CompressionAlgorithm::Zstd,
    );
    let data = serde_json::json!({
        "intent": "translate English to Chinese",
        "data": vec![1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        "budget": 100,
        "timeout_ms": 30000,
        "metadata": {"source": "bench", "version": "1.0"}
    });

    c.bench_function("transport/serialize+zstd_pipeline", |b| {
        b.iter(|| engine.serialize(&data).unwrap())
    });
}

// ============================================================================
// Economy Benchmarks
// ============================================================================

fn bench_channel_creation(c: &mut Criterion) {
    let party_a = bench_did("did:nexa:party_a");
    let party_b = bench_did("did:nexa:party_b");

    c.bench_function("economy/channel_creation", |b| {
        b.iter(|| Channel::new("channel-1", party_a.clone(), party_b.clone(), 1000, 500))
    });
}

fn bench_channel_transfer(c: &mut Criterion) {
    let party_a = bench_did("did:nexa:party_a");
    let party_b = bench_did("did:nexa:party_b");
    let mut channel = Channel::new("channel-1", party_a, party_b, 1000, 500);

    c.bench_function("economy/channel_transfer", |b| {
        b.iter(|| {
            channel.transfer_a_to_b(10).unwrap();
            channel.transfer_b_to_a(10).unwrap();
        })
    });
}

fn bench_channel_manager_open(c: &mut Criterion) {
    c.bench_function("economy/channel_manager_open", |b| {
        b.iter_batched(
            || (bench_did("did:nexa:alice"), bench_did("did:nexa:bob")),
            |(party_a, party_b)| {
                let mut manager = ChannelManager::new();
                manager.open(party_a, party_b, 100, 50).unwrap()
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_channel_update_tps(c: &mut Criterion) {
    let party_a = bench_did("did:nexa:party_a");
    let party_b = bench_did("did:nexa:party_b");
    let mut channel = Channel::new("channel-1", party_a, party_b, 10000, 10000);

    let mut group = c.benchmark_group("economy/channel_update_tps");
    for &batch_size in &[1usize, 10, 100, 1000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &bs| {
                b.iter(|| {
                    for _ in 0..bs {
                        channel.transfer_a_to_b(1).unwrap();
                        channel.transfer_b_to_a(1).unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_receipt_sign_verify(c: &mut Criterion) {
    let payer_keypair = KeyPair::generate().unwrap();
    let payee_keypair = KeyPair::generate().unwrap();
    let payer = bench_did("did:nexa:payer");
    let payee = bench_did("did:nexa:payee");

    c.bench_function("economy/receipt_sign_payer", |b| {
        b.iter(|| {
            let mut receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 25, "translate");
            receipt.sign_payer(&payer_keypair).unwrap();
            receipt
        })
    });

    c.bench_function("economy/receipt_sign_both", |b| {
        b.iter(|| {
            let mut receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 25, "translate");
            receipt.sign_payer(&payer_keypair).unwrap();
            receipt.sign_payee(&payee_keypair).unwrap();
            receipt
        })
    });

    // Pre-sign for verification benchmark
    let mut receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 25, "translate");
    receipt.sign_payer(&payer_keypair).unwrap();
    receipt.sign_payee(&payee_keypair).unwrap();

    c.bench_function("economy/receipt_verify_both", |b| {
        b.iter(|| {
            ReceiptVerifier::verify_both_signatures(
                &receipt,
                payer_keypair.public_key().inner(),
                payee_keypair.public_key().inner(),
            )
            .unwrap()
        })
    });
}

fn bench_receipt_hash_chain(c: &mut Criterion) {
    let payer = bench_did("did:nexa:payer");
    let payee = bench_did("did:nexa:payee");

    c.bench_function("economy/receipt_compute_hash", |b| {
        let receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 25, "translate");
        b.iter(|| receipt.compute_hash())
    });

    c.bench_function("economy/receipt_chain_build_10", |b| {
        b.iter(|| {
            let mut chain = ReceiptChain::new(payer.clone(), payee.clone());
            for i in 0..10 {
                let receipt = chain.create_receipt(&format!("call-{}", i), 10, "translate");
                chain.add_receipt(receipt).unwrap();
            }
            chain
        })
    });
}

fn bench_budget_reserve(c: &mut Criterion) {
    let mut controller = BudgetController::new();

    c.bench_function("economy/budget_reserve", |b| {
        b.iter(|| {
            controller
                .reserve_budget("did:nexa:user1", "call-1", 10)
                .unwrap()
        })
    });
}

// ============================================================================
// Security Benchmarks
// ============================================================================

fn bench_aes_gcm_security_storage(c: &mut Criterion) {
    let encryption_key = [42u8; 32];

    let mut group = c.benchmark_group("security/aes_gcm_size");
    for &size in &[16usize, 64, 256, 1024, 4096, 16384] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            let storage = SecureKeyStorage::new(Some(encryption_key));
            let key_data = random_bytes(s);
            b.iter(|| {
                storage
                    .store_key("bench-key", "signing", &key_data, None)
                    .unwrap();
                let _ = storage.get_key("bench-key").unwrap().unwrap();
                storage.delete_key("bench-key").unwrap();
            });
        });
    }
    group.finish();
}

fn bench_rate_limit_check(c: &mut Criterion) {
    let limiter = RateLimiter::default_limiter();
    let key = RateLimitKey::Did("did:nexa:bench".to_string());

    c.bench_function("security/rate_limit_check", |b| {
        b.iter(|| limiter.check(&key).unwrap())
    });
}

fn bench_rate_limit_concurrent(c: &mut Criterion) {
    let limiter = Arc::new(RateLimiter::new(RateLimitConfig {
        requests_per_minute: 10000,
        requests_per_hour: 100000,
        requests_per_day: 1000000,
        burst_size: 100,
        enabled: true,
    }));

    let mut group = c.benchmark_group("security/rate_limit_concurrent");
    for &concurrency in &[1usize, 10, 100] {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &concurrency,
            |b, &c| {
                b.iter(|| {
                    let mut results = Vec::with_capacity(c);
                    for i in 0..c {
                        let key = RateLimitKey::Did(format!("did:nexa:user{}", i));
                        results.push(limiter.check(&key).unwrap());
                    }
                    results
                });
            },
        );
    }
    group.finish();
}

fn bench_audit_logging_throughput(c: &mut Criterion) {
    let sink = Arc::new(MemoryAuditSink::new(10000));
    let logger = AuditLogger::new(sink);

    let mut group = c.benchmark_group("security/audit_throughput");
    for &batch_size in &[1usize, 10, 100, 1000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &bs| {
                b.iter(|| {
                    for i in 0..bs {
                        logger
                            .log_key_generated(&format!("key-{}", i), "ed25519")
                            .unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

// ============================================================================
// Storage Benchmarks
// ============================================================================

fn bench_storage_capability(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = Arc::new(nexa_net::storage::MemoryStore::default_store());

    let schema = CapabilitySchema {
        version: "1.0".to_string(),
        metadata: ServiceMetadata {
            did: bench_did("did:nexa:bench"),
            name: "bench-service".to_string(),
            description: "Benchmark service".to_string(),
            tags: vec!["bench".to_string()],
        },
        endpoints: vec![],
    };

    c.bench_function("storage/register_capability", |b| {
        b.to_async(&rt).iter(|| {
            let store = store.clone();
            let schema = schema.clone();
            async move { store.register_capability(schema).await.unwrap() }
        })
    });

    c.bench_function("storage/get_capability", |b| {
        b.to_async(&rt).iter(|| {
            let store = store.clone();
            async move { store.get_capability("did:nexa:bench").await.unwrap() }
        })
    });
}

fn bench_storage_cache(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = Arc::new(nexa_net::storage::MemoryStore::default_store());
    let value = serde_json::json!({"key": "value", "data": [1, 2, 3, 4, 5]});

    c.bench_function("storage/cache_set_get", |b| {
        b.to_async(&rt).iter(|| {
            let store = store.clone();
            let value = value.clone();
            async move {
                store.cache_set("bench-key", value.clone()).await.unwrap();
                store.cache_get("bench-key").await.unwrap()
            }
        })
    });
}

fn bench_storage_crud_cycle(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = Arc::new(nexa_net::storage::MemoryStore::default_store());

    let mut group = c.benchmark_group("storage/crud_cycle");
    for &n_ops in &[1usize, 10, 100] {
        group.throughput(Throughput::Elements(n_ops as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n_ops), &n_ops, |b, &n| {
            b.to_async(&rt).iter(|| {
                let store = store.clone();
                async move {
                    for i in 0..n {
                        let did_str = format!("did:nexa:crud{}", i);
                        let schema = make_capability_schema(
                            &did_str,
                            &format!("svc-{}", i),
                            vec!["bench".to_string()],
                        );
                        store.register_capability(schema).await.unwrap();
                    }
                    for i in 0..n {
                        let did_str = format!("did:nexa:crud{}", i);
                        store.get_capability(&did_str).await.unwrap();
                    }
                    for i in 0..n {
                        let did_str = format!("did:nexa:crud{}", i);
                        store.unregister_capability(&did_str).await.unwrap();
                    }
                }
            });
        });
    }
    group.finish();
}

// ============================================================================
// Throughput / Parametric Benchmarks
// ============================================================================

fn bench_vectorization_throughput(c: &mut Criterion) {
    let vectorizer = Vectorizer::new();
    let mut group = c.benchmark_group("throughput/vectorization");

    for &size in &[10usize, 50, 100, 500] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            let texts: Vec<String> = (0..s)
                .map(|i| format!("test intent number {} for benchmarking", i))
                .collect();
            let texts_ref: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();

            b.iter(|| vectorizer.vectorize_batch(&texts_ref).unwrap())
        });
    }
    group.finish();
}

fn bench_signature_throughput(c: &mut Criterion) {
    let keypair = KeyPair::generate().unwrap();
    let mut group = c.benchmark_group("throughput/signature");

    for &size in &[100usize, 500, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
            let message = b"test message for throughput benchmark";

            b.iter(|| {
                for _ in 0..s {
                    keypair.sign(message).unwrap();
                }
            })
        });
    }
    group.finish();
}

// ============================================================================
// REST API Benchmarks
// ============================================================================

/// Setup a REST API test server on a random port for benchmarking.
/// Returns (base_url, shutdown_sender, server_task_handle, ProxyState).
///
/// Uses the same pattern as TestProxy: TcpListener on random port,
/// axum::serve with graceful shutdown, and router config relaxed for
/// hash-based MockEmbedder (min_similarity=-1.0).
async fn setup_rest_server() -> (String, oneshot::Sender<()>, JoinHandle<()>, Arc<ProxyState>) {
    let state = Arc::new(ProxyState::new());
    let router = RestServer::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<()>();

    let handle = tokio::spawn(async {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                rx.await.ok();
            })
            .await
            .ok();
    });

    // Wait for server ready by polling health endpoint
    let client = reqwest::Client::new();
    let health_url = format!("http://127.0.0.1:{}/v1/health", addr.port());
    for _ in 0..20 {
        if client.get(&health_url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Configure router with relaxed test settings (hash-based MockEmbedder
    // produces negative similarity scores, so min_similarity=-1.0 is needed)
    let config = RoutingConfig {
        min_similarity: -1.0,
        min_quality: 0.0,
        available_only: false,
        max_cost: 0,
        max_latency_ms: 0,
        weights: RoutingWeights::default(),
    };
    let new_router = SemanticRouter::with_shared(state.registry.clone(), state.node_status.clone())
        .with_config(config);
    {
        let mut router_lock = state.router.write().await;
        *router_lock = new_router;
    }

    (
        format!("http://127.0.0.1:{}", addr.port()),
        tx,
        handle,
        state,
    )
}

fn bench_rest_health(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (base_url, shutdown_tx, server_handle, _state) = rt.block_on(setup_rest_server());

    let client = reqwest::Client::new();
    let url = format!("{}/v1/health", base_url);

    c.bench_function("api/rest_health", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let url = url.clone();
            async move {
                let resp = client.get(&url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
                let _body: ApiHealthResponse = resp.json().await.unwrap();
            }
        })
    });

    // Shutdown server
    let _ = shutdown_tx.send(());
    rt.block_on(server_handle).ok();
}

fn bench_rest_register(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (base_url, shutdown_tx, server_handle, _state) = rt.block_on(setup_rest_server());

    let client = reqwest::Client::new();
    let url = format!("{}/v1/register", base_url);

    c.bench_function("api/rest_register", |b| {
        b.to_async(&rt).iter_batched(
            || ApiRegisterRequest {
                name: format!("bench-service-{}", uuid::Uuid::new_v4()),
                description: "Benchmark service".to_string(),
                tags: vec!["bench".to_string()],
                endpoint: "http://127.0.0.1:9999/service".to_string(),
                cost_per_call: 10,
            },
            |request| {
                let client = client.clone();
                let url = url.clone();
                async move {
                    let resp = client.post(&url).json(&request).send().await.unwrap();
                    assert_eq!(resp.status(), 200);
                    let _body: ApiRegisterResponse = resp.json().await.unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    let _ = shutdown_tx.send(());
    rt.block_on(server_handle).ok();
}

fn bench_rest_discover(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (base_url, shutdown_tx, server_handle, _state) = rt.block_on(setup_rest_server());

    let client = reqwest::Client::new();

    // Pre-register 1 capability so discover has something to find
    let register_url = format!("{}/v1/register", base_url);
    let register_req = ApiRegisterRequest {
        name: "translation-service".to_string(),
        description: "Translate English to Chinese".to_string(),
        tags: vec!["translation".to_string(), "nlp".to_string()],
        endpoint: "http://127.0.0.1:9999/translate".to_string(),
        cost_per_call: 10,
    };
    rt.block_on(async {
        let resp = client
            .post(&register_url)
            .json(&register_req)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    });

    let discover_url = format!("{}/v1/discover", base_url);

    c.bench_function("api/rest_discover", |b| {
        b.to_async(&rt).iter(|| {
            let client = client.clone();
            let discover_url = discover_url.clone();
            async move {
                let request = ApiDiscoverRequest {
                    intent: "translate text".to_string(),
                    max_results: Some(10),
                    threshold: None,
                };
                let resp = client
                    .post(&discover_url)
                    .json(&request)
                    .send()
                    .await
                    .unwrap();
                // Accept 200 (success) or 500 (internal error from MockEmbedder)
                assert!(resp.status() == 200 || resp.status() == 500);
                let _body: ApiDiscoverResponse = resp.json().await.unwrap();
            }
        })
    });

    let _ = shutdown_tx.send(());
    rt.block_on(server_handle).ok();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    identity_benches,
    bench_keypair_generation,
    bench_did_creation,
    bench_signing,
    bench_verification,
    bench_identity_keys_generation,
    bench_aes_gcm_encrypt_decrypt,
    bench_secure_key_store_operations,
);

criterion_group!(
    discovery_benches,
    bench_vectorization,
    bench_batch_vectorization,
    bench_semantic_similarity,
    bench_hnsw_insert,
    bench_hnsw_search,
    bench_kademlia_routing_table,
    bench_semantic_dht_end_to_end,
    bench_capability_registration,
    bench_cosine_distance_simd_friendly,
);

criterion_group!(
    transport_benches,
    bench_json_serialization,
    bench_json_deserialization,
    bench_compression,
    bench_decompression,
    bench_compression_ratio,
    bench_frame_encode_decode,
    bench_frame_header_encode_decode,
    bench_serialization_compression_pipeline,
);

criterion_group!(
    economy_benches,
    bench_channel_creation,
    bench_channel_transfer,
    bench_channel_manager_open,
    bench_channel_update_tps,
    bench_receipt_sign_verify,
    bench_receipt_hash_chain,
    bench_budget_reserve,
);

criterion_group!(
    security_benches,
    bench_aes_gcm_security_storage,
    bench_rate_limit_check,
    bench_rate_limit_concurrent,
    bench_audit_logging_throughput,
);

criterion_group!(
    storage_benches,
    bench_storage_capability,
    bench_storage_cache,
    bench_storage_crud_cycle,
);

criterion_group!(
    throughput_benches,
    bench_vectorization_throughput,
    bench_signature_throughput,
);

criterion_group!(
    api_benches,
    bench_rest_health,
    bench_rest_register,
    bench_rest_discover,
);

criterion_main!(
    identity_benches,
    discovery_benches,
    transport_benches,
    economy_benches,
    security_benches,
    storage_benches,
    throughput_benches,
    api_benches,
);
